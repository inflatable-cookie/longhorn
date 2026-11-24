//! Synchronous managed-tool installation and health (contract 027, part 4c).

use std::{
    borrow::Cow,
    collections::HashSet,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use fs4::{FileExt, TryLockError as FileTryLockError};
use longhorn_redact::Redactor;
use longhorn_support_log::bound_field;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactShape, Declaration, DigestSource, ExtractionLimits, Fault, InstallError, InstallFault,
    InstallRefusal, ProbePhase, ProcessRunner, ProvisionError, Refusal, Release, ReleaseError,
    ReleaseQuery, ReleaseRefusal, ReleaseSource, SignatureVerifier, extract_executable,
    name::{is_managed_file_name, is_plain_segment},
    verify_artifact, verify_signature,
};

/// Maximum bytes read from a downloaded checksum file or sidecar.
pub const MAX_DIGEST_METADATA_BYTES: usize = 64 * 1024;
/// Maximum bytes read from a stored install receipt, for a snapshot or a
/// health check. A larger file is not a receipt this crate wrote.
pub const MAX_RECEIPT_BYTES: usize = 64 * 1024;
/// Maximum bytes retained in each redacted probe output field.
pub const MAX_PROBE_FIELD_BYTES: usize = 4096;
/// Maximum bytes inspected from one runner output stream before decoding.
const MAX_PROBE_CAPTURE_BYTES: usize = 64 * 1024;
/// The private subdirectory of the staging directory that downloads land in.
/// Its dot prefix keeps it apart from every binary name, which cannot start
/// with a dot, so a downloaded asset can never collide with the executable.
const DOWNLOAD_DIR: &str = ".download";

/// Digest declaration for one release artifact.
#[derive(Clone, Copy, Debug)]
pub enum DigestDeclaration<'a> {
    /// Use a caller-pinned lowercase SHA-256 digest.
    Pinned(&'a str),
    /// Download and parse a release checksum file with this asset name.
    ReleaseChecksums {
        /// Name of the release checksum file.
        asset: &'a str,
    },
    /// Download and parse an artifact sidecar with this asset name.
    Sidecar {
        /// Name of the artifact's sidecar file.
        asset: &'a str,
    },
    /// No digest; a vendor team must be declared for verification to pass.
    None,
}

/// Caller-owned, product-neutral description of one managed tool.
#[derive(Clone, Copy, Debug)]
pub struct ToolDescription<'a> {
    /// Caller-owned receipt identity.
    pub name: &'a str,
    /// Single-file executable name under `managed_root/bin`.
    pub binary_name: &'a str,
    /// Release owner, repository, optional exact tag and optional minimum.
    pub release: ReleaseQuery<'a>,
    /// Exact executable asset name in the release.
    pub asset: &'a str,
    /// Artifact format and already-expanded member path.
    pub shape: ArtifactShape<'a>,
    /// Digest file or pin declaration.
    pub digest: DigestDeclaration<'a>,
    /// Expected signing team, when the host requires a signature.
    pub vendor_team: Option<&'a str>,
    /// Version-probe arguments; the binary path itself is fixed by the pipeline.
    pub version_arguments: &'a [&'a str],
}

/// Install stages a host may map to its own product-facing labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallStage {
    /// Validate inputs and inspect the current install.
    Checking,
    /// Resolve the requested release.
    Resolving,
    /// Download the executable artifact.
    Downloading,
    /// Download the declared checksum metadata.
    DownloadingChecksums,
    /// Check the artifact digest.
    Verifying,
    /// Extract the requested executable.
    Extracting,
    /// Check the executable's vendor team.
    VerifyingSignature,
    /// Run the candidate's version command.
    Probing,
    /// Replace the managed binary and write its receipt.
    Installing,
}

/// The result returned by a completed install or an already-current check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallReceipt {
    name: String,
    version: String,
    release_tag: Option<String>,
    replaced_version: Option<String>,
    already_current: bool,
}

impl InstallReceipt {
    /// The caller's tool name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The version reported by the installed executable.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The resolved release tag, when this invocation resolved a release.
    pub fn release_tag(&self) -> Option<&str> {
        self.release_tag.as_deref()
    }

    /// The previous installed version, if it could be probed.
    pub fn replaced_version(&self) -> Option<&str> {
        self.replaced_version.as_deref()
    }

    /// Whether no artifact replacement was needed.
    pub const fn already_current(&self) -> bool {
        self.already_current
    }
}

/// Health of one tool at its managed binary path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolHealth {
    /// The executable runs and reports a version.
    Installed {
        /// The version reported by the executable.
        version: String,
        /// The receipt's recorded version, if the receipt can be read.
        receipt_version: Option<String>,
        /// Whether the reported version satisfies the caller's floor.
        at_or_above_floor: bool,
    },
    /// No binary exists at the managed path.
    NotInstalled,
    /// A binary exists but cannot be probed or reports no version.
    Broken {
        /// A bounded, redacted probe summary.
        detail: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredReceipt {
    name: String,
    version: String,
    release_tag: String,
    asset: String,
    sha256: String,
    installed_at_unix: u64,
}

#[derive(Clone, Debug)]
struct ReceiptSnapshot {
    bytes: Option<Vec<u8>>,
    obstructed: bool,
}

/// What a failed install must put back: the previous binary and receipt.
struct RollbackContext<'a> {
    binary: &'a Path,
    backup: &'a Path,
    had_previous: bool,
    receipt: &'a Path,
    receipt_snapshot: &'a ReceiptSnapshot,
}

/// The managed paths one install uses, under canonical roots.
struct ManagedLayout {
    managed_root: PathBuf,
    bin_dir: PathBuf,
    binary: PathBuf,
    backup: PathBuf,
    receipt: PathBuf,
}

impl ManagedLayout {
    fn create(tool: &ToolDescription<'_>, managed_root: &Path) -> Result<Self, InstallError> {
        fs::create_dir_all(managed_root).map_err(|error| io_error(managed_root, error))?;
        let managed_root =
            fs::canonicalize(managed_root).map_err(|error| io_error(managed_root, error))?;
        let bin_dir = managed_root.join("bin");
        fs::create_dir_all(&bin_dir).map_err(|error| io_error(&bin_dir, error))?;
        let bin_dir = fs::canonicalize(&bin_dir).map_err(|error| io_error(&bin_dir, error))?;
        Ok(Self {
            binary: bin_dir.join(tool.binary_name),
            backup: bin_dir.join(format!(".{}.previous", tool.binary_name)),
            receipt: receipt_path(&managed_root, tool.name),
            managed_root,
            bin_dir,
        })
    }
}

/// Digest metadata as the downloads produced it. The variants own the
/// downloaded text, so verification can borrow it without a second read.
enum FetchedDigest<'a> {
    Absent,
    Pinned(&'a str),
    ReleaseChecksums(String),
    Sidecar(String),
}

impl FetchedDigest<'_> {
    fn source(&self) -> Option<DigestSource<'_>> {
        match self {
            Self::Absent => None,
            Self::Pinned(pin) => Some(DigestSource::Pinned(pin)),
            Self::ReleaseChecksums(text) => Some(DigestSource::ReleaseChecksums(text)),
            Self::Sidecar(text) => Some(DigestSource::Sidecar(text)),
        }
    }
}

static ACTIVE_INSTALLS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

/// Installs one caller-described tool synchronously.
///
/// The host supplies the release source, signature verifier, process runner,
/// redactor, managed root and product-facing progress callback. The pipeline
/// performs no Tauri calls and creates no runtime.
///
/// # Errors
///
/// Returns preserved release or verification classes, an artifact-version
/// refusal, or a classified host fault. An install failure after replacement
/// restores the previous binary and receipt when those operations succeed.
pub fn install_tool(
    tool: &ToolDescription<'_>,
    managed_root: &Path,
    source: &dyn ReleaseSource,
    verifier: &dyn SignatureVerifier,
    runner: &dyn ProcessRunner,
    redactor: &Redactor,
    progress: &mut dyn FnMut(InstallStage),
) -> Result<InstallReceipt, InstallError> {
    validate_tool(tool)?;
    progress(InstallStage::Checking);

    let layout = ManagedLayout::create(tool, managed_root)?;
    let _guard = reserve_install(&layout.binary, &layout.managed_root)?;
    if layout.backup.exists() {
        return Err(InstallFault::RecoveryRequired {
            backup: layout.backup,
        }
        .into());
    }

    let had_previous = managed_binary_exists(&layout.binary)?;
    let previous_probe = if had_previous {
        probe_version(
            &layout.binary,
            tool.version_arguments,
            runner,
            redactor,
            ProbePhase::Installed,
        )
        .ok()
    } else {
        None
    };
    if let Some(installed) = previous_probe.as_ref()
        && let Some(receipt) = retained_newer_install(tool, installed, &layout.receipt)
    {
        return Ok(receipt);
    }

    progress(InstallStage::Resolving);
    let release = resolve_release(tool, source)?;
    if let (Some(installed), Some(version)) = (previous_probe.as_ref(), release.version())
        && installed == version
        && read_stored_receipt(&layout.receipt)
            .ok()
            .flatten()
            .is_some_and(|receipt| receipt.version == version.to_string())
    {
        return Ok(InstallReceipt {
            name: tool.name.to_owned(),
            version: installed.to_string(),
            release_tag: Some(release.tag().to_owned()),
            replaced_version: None,
            already_current: true,
        });
    }

    if matches!(tool.digest, DigestDeclaration::None) && tool.vendor_team.is_none() {
        return Err(ProvisionError::Refused(Refusal::NothingDeclared {
            asset: tool.asset.to_owned(),
        })
        .into());
    }

    let staging = tempfile::Builder::new()
        .prefix(".longhorn-tool-provision-")
        .tempdir_in(&layout.bin_dir)
        .map_err(|error| io_error(&layout.bin_dir, error))?;
    let downloads = staging.path().join(DOWNLOAD_DIR);
    fs::create_dir(&downloads).map_err(|error| io_error(&downloads, error))?;

    progress(InstallStage::Downloading);
    let artifact_path = source.download_asset(&release, tool.asset, &downloads)?;
    let fetched = fetch_digest(tool, &release, source, &downloads, progress)?;

    progress(InstallStage::Verifying);
    let verified = verify_artifact(
        &artifact_path,
        tool.asset,
        Declaration {
            digest: fetched.source(),
            vendor_team: tool.vendor_team,
        },
    )?;

    progress(InstallStage::Extracting);
    let staged = extract_executable(
        &verified,
        tool.shape,
        staging.path(),
        tool.binary_name,
        ExtractionLimits::DEFAULT,
    )?;

    if tool.vendor_team.is_some() {
        progress(InstallStage::VerifyingSignature);
    }
    let installable = verify_signature(staged, verifier)?;

    progress(InstallStage::Probing);
    let staged_version = probe_version(
        installable.path(),
        tool.version_arguments,
        runner,
        redactor,
        ProbePhase::Staged,
    )?;
    check_staged_version(tool, &release, &staged_version)?;

    let replaced_version = previous_probe.map(|version| version.to_string());
    let receipt_snapshot = snapshot_receipt(&layout.receipt)?;
    let receipt_dir = layout.receipt.parent().ok_or_else(|| InstallFault::Io {
        path: layout.receipt.clone(),
        detail: "the receipt path has no parent directory".to_owned(),
    })?;
    fs::create_dir_all(receipt_dir).map_err(|error| io_error(receipt_dir, error))?;
    let rollback_context = RollbackContext {
        binary: &layout.binary,
        backup: &layout.backup,
        had_previous,
        receipt: &layout.receipt,
        receipt_snapshot: &receipt_snapshot,
    };

    progress(InstallStage::Installing);
    if had_previous {
        write_backup(
            &layout.binary,
            &layout.backup,
            &layout.bin_dir,
            tool.binary_name,
        )?;
    }
    if let Err(cause) = place_candidate(installable.path(), &layout.binary) {
        return Err(if had_previous {
            cleanup_failed_swap(cause, &layout.backup)
        } else {
            cause
        });
    }
    // The candidate has moved out of the staging directory, so only downloads
    // and scratch files remain. They are removed here, best effort: the swap
    // has happened and nothing after it may fail on their account.
    drop(staging);

    let installed_version =
        match probe_installed(tool, &layout.binary, &staged_version, runner, redactor) {
            Ok(version) => version,
            Err(cause) => return Err(rollback(cause, &rollback_context)),
        };
    let stored = match stored_receipt(tool, &release, &layout.binary, &installed_version) {
        Ok(stored) => stored,
        Err(cause) => return Err(rollback(cause, &rollback_context)),
    };
    let receipt_bytes = match serde_json::to_vec(&stored) {
        Ok(bytes) => bytes,
        Err(error) => {
            let cause = InstallError::Fault(InstallFault::Io {
                path: layout.receipt.clone(),
                detail: format!("the install receipt could not be encoded: {error}"),
            });
            return Err(rollback(cause, &rollback_context));
        }
    };
    if let Err(error) = write_atomic(&layout.receipt, receipt_dir, &receipt_bytes) {
        let cause = InstallError::Fault(io_error(&layout.receipt, error));
        return Err(rollback(cause, &rollback_context));
    }

    if had_previous && let Err(error) = fs::remove_file(&layout.backup) {
        let cause = InstallError::Fault(io_error(&layout.backup, error));
        return Err(rollback(cause, &rollback_context));
    }

    Ok(InstallReceipt {
        name: tool.name.to_owned(),
        version: installed_version.to_string(),
        release_tag: Some(release.tag().to_owned()),
        replaced_version,
        already_current: false,
    })
}

/// A managed binary newer than the pinned tag, and within the floor, is kept
/// as it is. The receipt supplies the tag it was installed from, if readable.
fn retained_newer_install(
    tool: &ToolDescription<'_>,
    installed: &Version,
    receipt_path: &Path,
) -> Option<InstallReceipt> {
    let pin = tool.release.pinned_tag.and_then(crate::version_from_tag)?;
    let meets_floor = tool
        .release
        .version_floor
        .is_none_or(|floor| installed >= floor);
    if installed <= &pin || !meets_floor {
        return None;
    }
    Some(InstallReceipt {
        name: tool.name.to_owned(),
        version: installed.to_string(),
        release_tag: read_stored_receipt(receipt_path)
            .ok()
            .flatten()
            .map(|stored| stored.release_tag),
        replaced_version: None,
        already_current: true,
    })
}

/// Resolves the release and applies the caller's tag pin and version floor.
fn resolve_release(
    tool: &ToolDescription<'_>,
    source: &dyn ReleaseSource,
) -> Result<Release, InstallError> {
    let release = source.resolve_release(&tool.release)?;
    if let Some(requested) = tool.release.pinned_tag
        && release.tag() != requested
    {
        return Err(ReleaseError::Refused(ReleaseRefusal::TagMismatch {
            requested: requested.to_owned(),
            found: release.tag().to_owned(),
        })
        .into());
    }
    if let Some(floor) = tool.release.version_floor
        && release.version().is_none_or(|version| version < floor)
    {
        return Err(ReleaseError::Refused(ReleaseRefusal::NoCompatibleRelease {
            floor: Some(floor.clone()),
            requested: tool.release.pinned_tag.map(str::to_owned),
        })
        .into());
    }
    Ok(release)
}

/// Downloads the digest metadata the declaration names, if any.
fn fetch_digest<'a>(
    tool: &ToolDescription<'a>,
    release: &Release,
    source: &dyn ReleaseSource,
    downloads: &Path,
    progress: &mut dyn FnMut(InstallStage),
) -> Result<FetchedDigest<'a>, InstallError> {
    match tool.digest {
        DigestDeclaration::None => Ok(FetchedDigest::Absent),
        DigestDeclaration::Pinned(pin) => Ok(FetchedDigest::Pinned(pin)),
        DigestDeclaration::ReleaseChecksums { asset } => Ok(FetchedDigest::ReleaseChecksums(
            fetch_digest_text(release, source, downloads, asset, false, progress)?,
        )),
        DigestDeclaration::Sidecar { asset } => Ok(FetchedDigest::Sidecar(fetch_digest_text(
            release, source, downloads, asset, true, progress,
        )?)),
    }
}

fn fetch_digest_text(
    release: &Release,
    source: &dyn ReleaseSource,
    downloads: &Path,
    asset: &str,
    sidecar: bool,
    progress: &mut dyn FnMut(InstallStage),
) -> Result<String, InstallError> {
    if !release.has_asset(asset) {
        return Err(ReleaseError::Refused(ReleaseRefusal::MissingAsset {
            release: release.tag().to_owned(),
            asset: asset.to_owned(),
        })
        .into());
    }
    progress(InstallStage::DownloadingChecksums);
    let path = source.download_asset(release, asset, downloads)?;
    read_digest_text(&path, asset, sidecar)
}

/// Checks the staged executable's version against the release and the floor.
fn check_staged_version(
    tool: &ToolDescription<'_>,
    release: &Release,
    found: &Version,
) -> Result<(), InstallError> {
    if let Some(expected) = release.version()
        && found != expected
    {
        return Err(InstallRefusal::VersionMismatch {
            found: found.clone(),
            expected: expected.clone(),
        }
        .into());
    }
    if let Some(floor) = tool.release.version_floor
        && found < floor
    {
        return Err(InstallRefusal::BelowFloor {
            found: found.clone(),
            floor: floor.clone(),
        }
        .into());
    }
    Ok(())
}

/// Probes the installed binary after the swap; it must report the version
/// the staged candidate reported.
fn probe_installed(
    tool: &ToolDescription<'_>,
    binary: &Path,
    staged_version: &Version,
    runner: &dyn ProcessRunner,
    redactor: &Redactor,
) -> Result<Version, InstallError> {
    let version = probe_version(
        binary,
        tool.version_arguments,
        runner,
        redactor,
        ProbePhase::Installed,
    )?;
    if &version == staged_version {
        Ok(version)
    } else {
        Err(InstallRefusal::VersionMismatch {
            found: version,
            expected: staged_version.clone(),
        }
        .into())
    }
}

fn stored_receipt(
    tool: &ToolDescription<'_>,
    release: &Release,
    binary: &Path,
    version: &Version,
) -> Result<StoredReceipt, InstallError> {
    let sha256 = crate::sha256_file(binary).map_err(InstallError::from)?;
    let installed_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(StoredReceipt {
        name: tool.name.to_owned(),
        version: version.to_string(),
        release_tag: release.tag().to_owned(),
        asset: tool.asset.to_owned(),
        sha256,
        installed_at_unix,
    })
}

/// Checks the managed binary without contacting the release source.
pub fn tool_health(
    tool: &ToolDescription<'_>,
    managed_root: &Path,
    runner: &dyn ProcessRunner,
    redactor: &Redactor,
) -> ToolHealth {
    if validate_tool(tool).is_err() {
        return ToolHealth::Broken {
            detail: "the tool description is invalid".to_owned(),
        };
    }
    let binary = managed_root.join("bin").join(tool.binary_name);
    if !binary.is_file() {
        return ToolHealth::NotInstalled;
    }
    match probe_version(
        &binary,
        tool.version_arguments,
        runner,
        redactor,
        ProbePhase::Installed,
    ) {
        Ok(version) => {
            let receipt_version = read_stored_receipt(&receipt_path(managed_root, tool.name))
                .ok()
                .flatten()
                .map(|receipt| receipt.version);
            let at_or_above_floor = tool
                .release
                .version_floor
                .is_none_or(|floor| version >= *floor);
            ToolHealth::Installed {
                version: version.to_string(),
                receipt_version,
                at_or_above_floor,
            }
        }
        Err(error) => ToolHealth::Broken {
            detail: error.to_string(),
        },
    }
}

/// Parses the first semantic version token from a tool's version output.
#[must_use]
pub fn parse_version(output: &str) -> Option<Version> {
    for token in output.split_whitespace() {
        let candidate = token.trim_start_matches('v').trim_end_matches([',', ')']);
        let candidate = candidate.split('+').next().unwrap_or(candidate);
        if let Ok(version) = Version::parse(candidate) {
            return Some(version);
        }
    }
    None
}

fn validate_tool(tool: &ToolDescription<'_>) -> Result<(), InstallError> {
    for (label, value) in [("tool name", tool.name), ("binary name", tool.binary_name)] {
        if !is_managed_file_name(value) {
            return Err(ProvisionError::Fault(Fault::Usage {
                detail: format!("{label} must be a plain, non-hidden file name"),
            })
            .into());
        }
    }
    if !is_plain_segment(tool.asset) {
        return Err(ProvisionError::Fault(Fault::Usage {
            detail: "the release asset must be a plain file name".to_owned(),
        })
        .into());
    }
    if let DigestDeclaration::ReleaseChecksums { asset } | DigestDeclaration::Sidecar { asset } =
        tool.digest
        && !is_plain_segment(asset)
    {
        return Err(ProvisionError::Fault(Fault::Usage {
            detail: "the digest metadata asset must be a plain file name".to_owned(),
        })
        .into());
    }
    Ok(())
}

fn reserve_install(binary: &Path, managed_root: &Path) -> Result<InstallGuard, InstallError> {
    let parent = binary.parent().unwrap_or_else(|| Path::new("."));
    let name = binary.file_name().ok_or_else(|| {
        InstallError::Fault(InstallFault::Io {
            path: binary.to_path_buf(),
            detail: "the managed binary path has no file name".to_owned(),
        })
    })?;
    let key = fs::canonicalize(parent)
        .map_err(|error| io_error(binary, error))?
        .join(name);
    let active_locks = ACTIVE_INSTALLS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut active_guard = active_locks
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !active_guard.insert(key.clone()) {
        return Err(InstallFault::ConcurrentInstall { binary: key }.into());
    }
    drop(active_guard);

    let lock_dir = managed_root.join("state/tool-provision/locks");
    let lock_path = lock_dir.join(format!("{}.lock", name.to_string_lossy()));
    let lock_result = (|| {
        fs::create_dir_all(&lock_dir)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        match FileExt::try_lock(&file) {
            Ok(()) => Ok(file),
            Err(FileTryLockError::WouldBlock) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "another process owns the managed-tool install lock",
            )),
            Err(FileTryLockError::Error(error)) => Err(error),
        }
    })();
    match lock_result {
        Ok(file) => Ok(InstallGuard {
            key,
            _lock_file: file,
        }),
        Err(error) => {
            active_locks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&key);
            if error.kind() == io::ErrorKind::WouldBlock {
                Err(InstallFault::ConcurrentInstall { binary: key }.into())
            } else {
                Err(io_error(&lock_path, error).into())
            }
        }
    }
}

fn managed_binary_exists(binary: &Path) -> Result<bool, InstallError> {
    match fs::symlink_metadata(binary) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(ProvisionError::Fault(crate::Fault::Usage {
            detail: "the managed binary path must be absent or a regular file".to_owned(),
        })
        .into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(binary, error).into()),
    }
}

struct InstallGuard {
    key: PathBuf,
    _lock_file: File,
}

impl Drop for InstallGuard {
    fn drop(&mut self) {
        if let Some(active) = ACTIVE_INSTALLS.get() {
            active
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&self.key);
        }
    }
}

fn probe_version(
    binary: &Path,
    arguments: &[&str],
    runner: &dyn ProcessRunner,
    redactor: &Redactor,
    phase: ProbePhase,
) -> Result<Version, InstallError> {
    let arguments = arguments
        .iter()
        .map(std::ffi::OsString::from)
        .collect::<Vec<_>>();
    let output = runner
        .run(binary, &arguments)
        .map_err(|error| InstallFault::ProcessStart {
            program: binary.to_path_buf(),
            detail: bound_field(redactor, error.detail(), MAX_PROBE_FIELD_BYTES),
        })?;
    // Admission reads the raw stream. Redaction and the 4 KiB field bound
    // shape diagnostic evidence only: a consumer redactor that protects the
    // printed version, or a bound that cuts it, must not decide the outcome.
    let admitted = parse_version(&raw_probe_text(output.stdout()));
    let (stdout, stdout_truncated) =
        bounded_output(output.stdout(), output.stdout_truncated(), redactor);
    let (stderr, stderr_truncated) =
        bounded_output(output.stderr(), output.stderr_truncated(), redactor);
    if output.success()
        && let Some(version) = admitted
    {
        return Ok(version);
    }
    Err(InstallFault::Probe {
        phase,
        stdout,
        stderr,
        exit_status: output.exit_status(),
        stdout_truncated,
        stderr_truncated,
        detail: if output.success() {
            "the process did not print a semantic version".to_owned()
        } else {
            "the process exited unsuccessfully".to_owned()
        },
    }
    .into())
}

/// The raw text a version decision reads: decoded, cut at the capture bound,
/// and never a straddling token's prefix. Trimming back to the last complete
/// token keeps a cut `1.2.3-alpha` from being admitted as `1.2.3-al`.
fn raw_probe_text(bytes: &[u8]) -> Cow<'_, str> {
    let bound = bytes.len().min(MAX_PROBE_CAPTURE_BYTES);
    let end = match bytes.get(bound) {
        Some(next) if !next.is_ascii_whitespace() => bytes[..bound]
            .iter()
            .rposition(|byte| byte.is_ascii_whitespace())
            .unwrap_or(0),
        _ => bound,
    };
    String::from_utf8_lossy(&bytes[..end])
}

fn bounded_output(bytes: &[u8], runner_truncated: bool, redactor: &Redactor) -> (String, bool) {
    let cut = bytes.len() > MAX_PROBE_CAPTURE_BYTES;
    let bytes = &bytes[..bytes.len().min(MAX_PROBE_CAPTURE_BYTES)];
    let decoded = String::from_utf8_lossy(bytes);
    let redacted_len = redactor.redact(&decoded).text.len();
    let bounded = bound_field(redactor, &decoded, MAX_PROBE_FIELD_BYTES);
    let truncated = runner_truncated || cut || redacted_len > MAX_PROBE_FIELD_BYTES;
    (bounded, truncated)
}

fn read_digest_text(path: &Path, asset: &str, sidecar: bool) -> Result<String, InstallError> {
    let mut file = File::open(path).map_err(|error| io_error(path, error))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_DIGEST_METADATA_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(path, error))?;
    let malformed = || {
        ProvisionError::Refused(if sidecar {
            Refusal::MalformedSidecar {
                asset: asset.to_owned(),
            }
        } else {
            Refusal::MalformedChecksum {
                asset: asset.to_owned(),
            }
        })
    };
    if bytes.len() > MAX_DIGEST_METADATA_BYTES {
        return Err(malformed().into());
    }
    String::from_utf8(bytes).map_err(|_| malformed().into())
}

fn snapshot_receipt(path: &Path) -> Result<ReceiptSnapshot, InstallError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => match read_receipt(path) {
            Ok(ReceiptRead::Valid { bytes, .. } | ReceiptRead::Invalid(bytes)) => {
                Ok(ReceiptSnapshot {
                    bytes: Some(bytes),
                    obstructed: false,
                })
            }
            Ok(ReceiptRead::Missing) => Ok(ReceiptSnapshot {
                bytes: None,
                obstructed: false,
            }),
            Ok(ReceiptRead::Oversized) => Err(receipt_path_expectation()),
            Err(error) => Err(io_error(path, error).into()),
        },
        Ok(metadata) if metadata.file_type().is_dir() => Ok(ReceiptSnapshot {
            bytes: None,
            obstructed: true,
        }),
        Ok(_) => Err(receipt_path_expectation()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ReceiptSnapshot {
            bytes: None,
            obstructed: false,
        }),
        Err(error) => Err(io_error(path, error).into()),
    }
}

fn read_stored_receipt(path: &Path) -> io::Result<Option<StoredReceipt>> {
    match read_receipt(path)? {
        ReceiptRead::Valid { receipt, .. } => Ok(Some(receipt)),
        ReceiptRead::Missing | ReceiptRead::Invalid(_) | ReceiptRead::Oversized => Ok(None),
    }
}

/// One bounded receipt read, shared by the rollback snapshot and health.
/// Missing, invalid and oversized distinct so each caller states its own
/// policy instead of inferring it from an allocation.
enum ReceiptRead {
    /// Nothing exists at the path.
    Missing,
    /// The file is larger than [`MAX_RECEIPT_BYTES`] and is not read.
    Oversized,
    /// Bounded bytes that are not a receipt document; kept for a rollback.
    Invalid(Vec<u8>),
    /// A receipt document with the exact bytes it was decoded from.
    Valid {
        bytes: Vec<u8>,
        receipt: StoredReceipt,
    },
}

fn read_receipt(path: &Path) -> io::Result<ReceiptRead> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(ReceiptRead::Missing),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_RECEIPT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Ok(ReceiptRead::Oversized);
    }
    Ok(match serde_json::from_slice(&bytes) {
        Ok(receipt) => ReceiptRead::Valid { bytes, receipt },
        Err(_) => ReceiptRead::Invalid(bytes),
    })
}

fn receipt_path_expectation() -> InstallError {
    ProvisionError::Fault(Fault::Usage {
        detail: format!(
            "the receipt path must be absent, a regular file of at most {MAX_RECEIPT_BYTES} bytes, or a directory obstruction"
        ),
    })
    .into()
}

fn receipt_path(managed_root: &Path, name: &str) -> PathBuf {
    managed_root
        .join("state")
        .join("tool-provision")
        .join(format!("{name}.json"))
}

/// Copies the current binary to the backup name, which must not exist. A
/// backup left by an earlier run is a recovery condition, never overwritten.
fn write_backup(
    binary: &Path,
    backup: &Path,
    bin_dir: &Path,
    name: &str,
) -> Result<(), InstallError> {
    let mut temporary = tempfile::Builder::new()
        .prefix(&format!(".{name}.previous-"))
        .tempfile_in(bin_dir)
        .map_err(|error| io_error(bin_dir, error))?;
    let mut original = File::open(binary).map_err(|error| io_error(binary, error))?;
    io::copy(&mut original, temporary.as_file_mut())
        .map_err(|error| io_error(temporary.path(), error))?;
    make_executable(temporary.path()).map_err(|error| io_error(temporary.path(), error))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| io_error(temporary.path(), error))?;
    temporary.persist_noclobber(backup).map_err(|error| {
        if error.error.kind() == io::ErrorKind::AlreadyExists {
            InstallError::Fault(InstallFault::RecoveryRequired {
                backup: backup.to_path_buf(),
            })
        } else {
            InstallError::Fault(io_error(backup, error.error))
        }
    })?;
    Ok(())
}

/// Makes the verified candidate the managed binary. The candidate sits in the
/// staging directory on the same volume, so the swap is one rename: nothing
/// is copied. The candidate is synced first, so the rename never publishes a
/// file whose bytes are not yet on disk.
fn place_candidate(candidate: &Path, binary: &Path) -> Result<(), InstallError> {
    File::open(candidate)
        .and_then(|file| file.sync_all())
        .map_err(|error| io_error(candidate, error))?;
    fs::rename(candidate, binary).map_err(|error| io_error(binary, error).into())
}

fn write_atomic(path: &Path, parent: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temporary = tempfile::Builder::new()
        .prefix(".receipt-incoming-")
        .tempfile_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
}

/// Restores what the failed install replaced, then reports the cause. A
/// restore that also fails keeps its own description beside the cause.
fn rollback(cause: InstallError, context: &RollbackContext<'_>) -> InstallError {
    let mut failures = Vec::new();
    if context.had_previous {
        // Rename consumes the backup, so a successful restore leaves none.
        if let Err(error) = fs::rename(context.backup, context.binary) {
            failures.push(format!("{}: {error}", context.binary.display()));
        }
    } else if let Err(error) = remove_if_present(context.binary) {
        failures.push(format!("{}: {error}", context.binary.display()));
    }

    if !context.receipt_snapshot.obstructed {
        let restored = match context.receipt_snapshot.bytes.as_deref() {
            Some(bytes) => context
                .receipt
                .parent()
                .ok_or_else(|| io::Error::other("receipt path has no parent"))
                .and_then(|parent| write_atomic(context.receipt, parent, bytes)),
            None => remove_if_present(context.receipt),
        };
        if let Err(error) = restored {
            failures.push(format!("{}: {error}", context.receipt.display()));
        }
    }

    if failures.is_empty() {
        cause
    } else {
        InstallFault::Rollback {
            cause: Box::new(cause),
            detail: failures.join("; "),
        }
        .into()
    }
}

fn cleanup_failed_swap(cause: InstallError, backup: &Path) -> InstallError {
    match remove_if_present(backup) {
        Ok(()) => cause,
        Err(error) => InstallFault::Rollback {
            cause: Box::new(cause),
            detail: format!("{}: {error}", backup.display()),
        }
        .into(),
    }
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn make_executable(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn io_error(path: &Path, error: io::Error) -> InstallFault {
    InstallFault::Io {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}

impl From<InstallFault> for InstallError {
    fn from(fault: InstallFault) -> Self {
        Self::Fault(fault)
    }
}

impl From<InstallRefusal> for InstallError {
    fn from(refusal: InstallRefusal) -> Self {
        Self::Refused(refusal)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::{MAX_PROBE_CAPTURE_BYTES, parse_version, raw_probe_text};
    use semver::Version;

    #[test]
    fn parse_version_reads_the_first_semver_token() {
        for (output, expected) in [
            ("tool v1.2.3\n", "1.2.3"),
            ("tool 2.3.4+vendor (build)\n", "2.3.4"),
            ("version 3.4.5, stable\n", "3.4.5"),
        ] {
            assert_eq!(
                parse_version(output),
                Some(Version::parse(expected).unwrap())
            );
        }
        assert_eq!(parse_version("no version here"), None);
    }

    #[test]
    fn raw_probe_text_keeps_a_version_inside_the_capture_bound() {
        let mut bytes = b"tool 9.9.9\n".to_vec();
        bytes.resize(MAX_PROBE_CAPTURE_BYTES + 16, b'x');
        assert_eq!(
            parse_version(&raw_probe_text(&bytes)),
            Some(Version::parse("9.9.9").unwrap())
        );
    }

    #[test]
    fn raw_probe_text_drops_a_token_that_straddles_the_capture_bound() {
        // The bound lands inside the pre-release identifier, where a prefix
        // such as `1.2.3-al` would parse as a complete, wrong version.
        let mut bytes = vec![b'x'; MAX_PROBE_CAPTURE_BYTES - 9];
        bytes.extend_from_slice(b" 1.2.3-alphabeta\n");
        assert_eq!(parse_version(&raw_probe_text(&bytes)), None);
    }
}
