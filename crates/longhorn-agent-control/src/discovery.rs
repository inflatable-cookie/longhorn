//! Discovery file model and lifecycle (contract 022).
//!
//! Each control-surface instance writes one file at
//! `<state root>/longhorn/agent-control/<app-id>-<pid>.json` carrying app
//! id, pid, port, token, and schema version. Agents enumerate the directory
//! to find live instances; stale files are detectable by dead pid; clean
//! exit removes the file.
//!
//! Path resolution runs through the contract 004 storage-profile resolver
//! with the fixed `longhorn` identity — never a hand-rolled dirs lookup —
//! so the macOS/Linux/Windows shapes stay exactly the profile's state-root
//! rules. Pure resolution does no filesystem or environment access; the
//! host injects platform directory facts, and tests inject temporary roots.

use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use longhorn_config::{
    PlatformDirectoryFacts, RootKind, StorageIdentity, StorageLayoutError, StorageLayoutOverrides,
    StorageLayoutRequest, resolve_storage_layout,
};
use serde::{Deserialize, Serialize};

use crate::InstanceToken;

/// Discovery file schema version. Pre-1.0, schema breaks bump this without
/// compatibility reads (contract 022).
pub const DISCOVERY_SCHEMA_VERSION: u32 = 1;

/// Fixed storage identity leaf shared by every app instance, so agents
/// enumerate one directory regardless of which apps are running.
const DISCOVERY_IDENTITY_LEAF: &str = "longhorn";

/// Directory child below the resolved state root.
const DISCOVERY_DIR_CHILD: &str = "agent-control";

/// Discovery resolution, publication, or enumeration failure.
#[derive(Debug)]
pub enum DiscoveryError {
    /// Storage layout resolution rejected the injected facts.
    Layout(StorageLayoutError),
    /// The app id is not one safe filename component.
    InvalidAppId {
        /// Rejected app id.
        app_id: String,
    },
    /// Filesystem failure beneath the discovery directory.
    Io {
        /// Path being operated on.
        path: PathBuf,
        /// Underlying I/O failure.
        source: io::Error,
    },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(source) => write!(formatter, "storage layout resolution failed: {source}"),
            Self::InvalidAppId { app_id } => {
                write!(
                    formatter,
                    "app id {app_id:?} is not one safe filename component"
                )
            }
            Self::Io { path, source } => {
                write!(formatter, "{}: {source}", path.display())
            }
        }
    }
}

impl Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Layout(source) => Some(source),
            Self::InvalidAppId { .. } => None,
            Self::Io { source, .. } => Some(source),
        }
    }
}

/// One instance's discovery record: everything an agent needs to reach it.
///
/// The token travels in this file by design — the file is how an agent
/// learns the credential — so the directory and file get the narrowest
/// practical permissions, and `Debug` stays redacted through
/// [`InstanceToken`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DiscoveryFile {
    /// Discovery schema version; see [`DISCOVERY_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Canonical application id of the serving instance.
    pub app_id: String,
    /// Operating system process id of the serving instance.
    pub pid: u32,
    /// Loopback port the instance's control server is bound to.
    pub port: u16,
    /// Per-instance bearer token required by the control server.
    pub token: InstanceToken,
}

/// Resolves the shared discovery directory from injected platform facts.
///
/// Always the `platform-native-v1` state root of the fixed `longhorn`
/// identity plus `agent-control`: `$XDG_STATE_HOME/longhorn/agent-control`
/// on Linux, `Application Support/longhorn/state/agent-control` on macOS.
/// The app's own profile choice never moves discovery — agents could not
/// find instances without knowing each app's profile.
pub fn resolve_discovery_dir(facts: &PlatformDirectoryFacts) -> Result<PathBuf, DiscoveryError> {
    resolve(facts, None)
}

/// Resolves the discovery directory with an explicit state-root override —
/// deployment and test policy per contract 004, recorded by the resolver
/// as an explicit override rather than a parallel root.
pub fn resolve_discovery_dir_with_state_override(
    facts: &PlatformDirectoryFacts,
    state_root: &Path,
) -> Result<PathBuf, DiscoveryError> {
    resolve(facts, Some(state_root))
}

fn resolve(
    facts: &PlatformDirectoryFacts,
    state_override: Option<&Path>,
) -> Result<PathBuf, DiscoveryError> {
    let identity = StorageIdentity::new(DISCOVERY_IDENTITY_LEAF)
        .expect("the fixed discovery leaf is a valid storage identity");
    let mut request = StorageLayoutRequest::new(identity, facts.clone());
    if let Some(state_root) = state_override {
        request =
            request.with_overrides(StorageLayoutOverrides::new().with(RootKind::State, state_root));
    }
    let layout = resolve_storage_layout(&request).map_err(DiscoveryError::Layout)?;
    Ok(layout.storage_roots().state().join(DISCOVERY_DIR_CHILD))
}

/// Validates that `app_id` is one safe filename component.
fn validate_app_id(app_id: &str) -> Result<(), DiscoveryError> {
    let safe = !app_id.is_empty()
        && app_id.len() <= 255
        && app_id != "."
        && app_id != ".."
        && app_id
            .chars()
            .all(|symbol| symbol.is_ascii_alphanumeric() || matches!(symbol, '.' | '_' | '-'));
    if safe {
        Ok(())
    } else {
        Err(DiscoveryError::InvalidAppId {
            app_id: app_id.to_owned(),
        })
    }
}

/// A published discovery file. Dropping does no I/O; clean exit calls
/// [`DiscoveryInstance::remove`].
#[derive(Clone, Debug)]
pub struct DiscoveryInstance {
    path: PathBuf,
    file: DiscoveryFile,
}

impl DiscoveryInstance {
    /// Returns the published file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the published record.
    #[must_use]
    pub const fn file(&self) -> &DiscoveryFile {
        &self.file
    }

    /// Removes the published file. Idempotent: an already-absent file is a
    /// clean removal, so crash-then-restart cleanup cannot fail.
    pub fn remove(self) -> Result<(), DiscoveryError> {
        remove_discovery_file(&self.path)
    }
}

/// Publishes one instance's discovery file below `dir`, creating the
/// directory when absent. The file is written owner-read-write only: it
/// carries the bearer token, a credential.
pub fn publish_discovery(
    dir: &Path,
    app_id: &str,
    port: u16,
    token: InstanceToken,
) -> Result<DiscoveryInstance, DiscoveryError> {
    validate_app_id(app_id)?;
    fs::create_dir_all(dir).map_err(|source| DiscoveryError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    narrow_directory_permissions(dir)?;

    let pid = std::process::id();
    let file = DiscoveryFile {
        schema_version: DISCOVERY_SCHEMA_VERSION,
        app_id: app_id.to_owned(),
        pid,
        port,
        token,
    };
    let path = dir.join(format!("{app_id}-{pid}.json"));
    let json = serde_json::to_string_pretty(&file).expect("discovery serialization cannot fail");
    write_credential_file(&path, json.as_bytes())?;

    Ok(DiscoveryInstance { path, file })
}

/// Removes one discovery file by path. Idempotent: absence is success.
pub fn remove_discovery_file(path: &Path) -> Result<(), DiscoveryError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(DiscoveryError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// One enumerated instance: the parsed record plus liveness verdict.
#[derive(Clone, Debug)]
pub struct DiscoveryRecord {
    path: PathBuf,
    file: DiscoveryFile,
    live: bool,
}

impl DiscoveryRecord {
    /// Returns the file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the parsed record.
    #[must_use]
    pub const fn file(&self) -> &DiscoveryFile {
        &self.file
    }

    /// Returns whether the record's pid is a live process.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.live
    }

    /// Returns whether the record is stale: the pid is dead and the file
    /// is leftover from an unclean exit.
    #[must_use]
    pub const fn is_stale(&self) -> bool {
        !self.live
    }
}

/// Result of enumerating the discovery directory.
#[derive(Clone, Debug, Default)]
pub struct DiscoveryScan {
    /// Parsed records with liveness verdicts.
    pub instances: Vec<DiscoveryRecord>,
    /// Files that failed name-shape, schema, or parse checks. A corrupt or
    /// foreign file never blocks enumeration of the rest.
    pub unreadable: Vec<PathBuf>,
}

/// Enumerates the discovery directory, probing liveness with
/// [`process_alive`]. A missing directory enumerates as empty.
pub fn enumerate_discovery(dir: &Path) -> Result<DiscoveryScan, DiscoveryError> {
    enumerate_discovery_with(dir, process_alive)
}

/// Enumerates with an injected liveness probe — the deterministic form for
/// fixtures and for hosts with their own process authority.
pub fn enumerate_discovery_with(
    dir: &Path,
    liveness: impl Fn(u32) -> bool,
) -> Result<DiscoveryScan, DiscoveryError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(DiscoveryScan::default());
        }
        Err(source) => {
            return Err(DiscoveryError::Io {
                path: dir.to_path_buf(),
                source,
            });
        }
    };

    let mut scan = DiscoveryScan::default();
    for entry in entries {
        let entry = entry.map_err(|source| DiscoveryError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        match parse_record(&path) {
            Some(file) => {
                let live = liveness(file.pid);
                scan.instances.push(DiscoveryRecord { path, file, live });
            }
            None => scan.unreadable.push(path),
        }
    }
    Ok(scan)
}

/// Reads and validates one discovery file: name shape `<app-id>-<pid>.json`,
/// current schema version, and pid agreement between name and payload.
fn parse_record(path: &Path) -> Option<DiscoveryFile> {
    let stem = path.file_stem()?.to_str()?;
    let (app_id, pid) = stem.rsplit_once('-')?;
    let pid: u32 = pid.parse().ok()?;
    if validate_app_id(app_id).is_err() {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let file: DiscoveryFile = serde_json::from_slice(&bytes).ok()?;
    if file.schema_version != DISCOVERY_SCHEMA_VERSION || file.app_id != app_id || file.pid != pid {
        return None;
    }
    Some(file)
}

/// Cross-platform process liveness probe behind the enumerator's default.
///
/// `std` has no safe liveness API and the workspace forbids `unsafe`, so
/// this is `sysinfo` scoped to one pid; anything else would hand-roll
/// platform process tables.
#[must_use]
pub fn process_alive(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessesToUpdate, System};

    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).is_some()
}

/// Writes owner-read-write-only bytes, creating or truncating the file.
fn write_credential_file(path: &Path, bytes: &[u8]) -> Result<(), DiscoveryError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|source| DiscoveryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        // An existing file keeps its old mode through truncate; re-assert.
        use std::os::unix::fs::PermissionsExt;
        let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
    }
    io::Write::write_all(&mut file, bytes).map_err(|source| DiscoveryError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Narrows the discovery directory to owner-only on unix.
fn narrow_directory_permissions(dir: &Path) -> Result<(), DiscoveryError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|source| {
            DiscoveryError::Io {
                path: dir.to_path_buf(),
                source,
            }
        })?;
    }
    let _ = dir;
    Ok(())
}
