//! Synchronous managed-tool installation and health (contract 027, part 4c).

use std::{
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
    ArtifactShape, Declaration, DigestSource, ExtractionLimits, InstallError, InstallFault,
    InstallRefusal, ProbePhase, ProcessRunner, ProvisionError, Refusal, ReleaseError, ReleaseQuery,
    ReleaseRefusal, ReleaseSource, SignatureVerifier, extract_executable, runner::RunnerError,
    verify_artifact, verify_signature,
};

/// Maximum bytes read from a downloaded checksum file or sidecar.
pub const MAX_DIGEST_METADATA_BYTES: usize = 64 * 1024;
/// Maximum bytes retained in each redacted probe output field.
pub const MAX_PROBE_FIELD_BYTES: usize = 4096;
/// Maximum bytes inspected from one runner output stream before decoding.
const MAX_PROBE_CAPTURE_BYTES: usize = 64 * 1024;

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

struct RollbackContext<'a> {
    binary: &'a Path,
    backup: &'a Path,
    had_previous: bool,
    receipt: &'a Path,
    receipt_snapshot: &'a ReceiptSnapshot,
    bin_dir: &'a Path,
    name: &'a str,
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

    fs::create_dir_all(managed_root).map_err(|error| io_error(managed_root, error))?;
    let managed_root =
        fs::canonicalize(managed_root).map_err(|error| io_error(managed_root, error))?;
    let bin_dir = managed_root.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|error| io_error(&bin_dir, error))?;
    let bin_dir = fs::canonicalize(&bin_dir).map_err(|error| io_error(&bin_dir, error))?;
    let binary = bin_dir.join(tool.binary_name);
    let _guard = reserve_install(&binary, &managed_root)?;
    let backup = bin_dir.join(format!(".{}.previous", tool.binary_name));
    if backup.exists() {
        return Err(InstallFault::RecoveryRequired { backup }.into());
    }

    let had_previous = managed_binary_exists(&binary)?;
    let receipt_path = receipt_path(&managed_root, tool.name);
    let previous_probe = if had_previous {
        probe_version(
            &binary,
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
        && let Some(pin) = tool.release.pinned_tag.and_then(crate::version_from_tag)
        && installed > &pin
        && tool
            .release
            .version_floor
            .is_none_or(|floor| installed >= floor)
    {
        let receipt = read_stored_receipt(&receipt_path).ok().flatten();
        return Ok(InstallReceipt {
            name: tool.name.to_owned(),
            version: installed.to_string(),
            release_tag: receipt.map(|stored| stored.release_tag),
            replaced_version: None,
            already_current: true,
        });
    }

    progress(InstallStage::Resolving);
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

    if let (Some(installed), Some(version)) = (previous_probe.as_ref(), release.version())
        && installed == version
        && read_stored_receipt(&receipt_path)
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
        .tempdir_in(&bin_dir)
        .map_err(|error| io_error(&bin_dir, error))?;

    progress(InstallStage::Downloading);
    let artifact_path = source.download_asset(&release, tool.asset, staging.path())?;

    let digest_text = match tool.digest {
        DigestDeclaration::Pinned(_) | DigestDeclaration::None => None,
        DigestDeclaration::ReleaseChecksums { asset } | DigestDeclaration::Sidecar { asset } => {
            if !release.has_asset(asset) {
                return Err(ReleaseError::Refused(ReleaseRefusal::MissingAsset {
                    release: release.tag().to_owned(),
                    asset: asset.to_owned(),
                })
                .into());
            }
            progress(InstallStage::DownloadingChecksums);
            let path = source.download_asset(&release, asset, staging.path())?;
            Some(read_digest_text(
                &path,
                asset,
                matches!(tool.digest, DigestDeclaration::Sidecar { .. }),
            )?)
        }
    };
    let digest_source = match (tool.digest, digest_text.as_deref()) {
        (DigestDeclaration::Pinned(pin), _) => Some(DigestSource::Pinned(pin)),
        (DigestDeclaration::ReleaseChecksums { .. }, Some(text)) => {
            Some(DigestSource::ReleaseChecksums(text))
        }
        (DigestDeclaration::Sidecar { .. }, Some(text)) => Some(DigestSource::Sidecar(text)),
        (DigestDeclaration::None, _) => None,
        _ => {
            return Err(ProvisionError::Fault(crate::Fault::Usage {
                detail: "digest metadata did not match its declaration".to_owned(),
            })
            .into());
        }
    };

    progress(InstallStage::Verifying);
    let verified = verify_artifact(
        &artifact_path,
        tool.asset,
        Declaration {
            digest: digest_source,
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
    if let Some(expected) = release.version()
        && &staged_version != expected
    {
        return Err(InstallRefusal::VersionMismatch {
            found: staged_version,
            expected: expected.clone(),
        }
        .into());
    }
    if let Some(floor) = tool.release.version_floor
        && staged_version < *floor
    {
        return Err(InstallRefusal::BelowFloor {
            found: staged_version,
            floor: floor.clone(),
        }
        .into());
    }

    let replaced_version = previous_probe.map(|version| version.to_string());
    let receipt_snapshot = snapshot_receipt(&receipt_path)?;
    let receipt_dir = receipt_path.parent().ok_or_else(|| InstallFault::Io {
        path: receipt_path.clone(),
        detail: "the receipt path has no parent directory".to_owned(),
    })?;
    fs::create_dir_all(receipt_dir).map_err(|error| io_error(receipt_dir, error))?;
    let rollback_context = RollbackContext {
        binary: &binary,
        backup: &backup,
        had_previous,
        receipt: &receipt_path,
        receipt_snapshot: &receipt_snapshot,
        bin_dir: &bin_dir,
        name: tool.binary_name,
    };

    let mut incoming = tempfile::Builder::new()
        .prefix(&format!(".{}.incoming-", tool.binary_name))
        .tempfile_in(&bin_dir)
        .map_err(|error| io_error(&bin_dir, error))?;
    let mut staged_file =
        File::open(installable.path()).map_err(|error| io_error(installable.path(), error))?;
    io::copy(&mut staged_file, incoming.as_file_mut())
        .map_err(|error| io_error(incoming.path(), error))?;
    make_executable(incoming.path()).map_err(|error| io_error(incoming.path(), error))?;
    incoming
        .as_file()
        .sync_all()
        .map_err(|error| io_error(incoming.path(), error))?;

    staging.close().map_err(|error| io_error(&bin_dir, error))?;
    progress(InstallStage::Installing);
    if had_previous {
        write_backup(&binary, &backup, &bin_dir, tool.binary_name)?;
    }

    if let Err(error) = incoming.persist(&binary) {
        let cause = InstallError::Fault(io_error(&binary, error.error));
        return Err(if had_previous {
            cleanup_failed_swap(cause, &backup)
        } else {
            cause
        });
    }

    let post_swap = probe_version(
        &binary,
        tool.version_arguments,
        runner,
        redactor,
        ProbePhase::Installed,
    )
    .and_then(|version| {
        if version == staged_version {
            Ok(version)
        } else {
            Err(InstallRefusal::VersionMismatch {
                found: version,
                expected: staged_version.clone(),
            }
            .into())
        }
    });
    let installed_version = match post_swap {
        Ok(version) => version,
        Err(cause) => return Err(rollback(cause, &rollback_context)),
    };

    let sha256 = match crate::sha256_file(&binary) {
        Ok(sha256) => sha256,
        Err(cause) => return Err(rollback(cause.into(), &rollback_context)),
    };
    let installed_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let stored = StoredReceipt {
        name: tool.name.to_owned(),
        version: installed_version.to_string(),
        release_tag: release.tag().to_owned(),
        asset: tool.asset.to_owned(),
        sha256,
        installed_at_unix,
    };
    let receipt_bytes = match serde_json::to_vec(&stored) {
        Ok(bytes) => bytes,
        Err(error) => {
            let cause = InstallError::Fault(InstallFault::Io {
                path: receipt_path.clone(),
                detail: format!("the install receipt could not be encoded: {error}"),
            });
            return Err(rollback(cause, &rollback_context));
        }
    };
    if let Err(error) = write_atomic(&receipt_path, receipt_dir, &receipt_bytes) {
        let cause = InstallError::Fault(io_error(&receipt_path, error));
        return Err(rollback(cause, &rollback_context));
    }

    if had_previous && let Err(error) = fs::remove_file(&backup) {
        let cause = InstallError::Fault(io_error(&backup, error));
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
        if !is_plain_name(value) {
            return Err(ProvisionError::Fault(crate::Fault::Usage {
                detail: format!("{label} must be a plain, non-hidden file name"),
            })
            .into());
        }
    }
    if !is_plain_asset(tool.asset) {
        return Err(ProvisionError::Fault(crate::Fault::Usage {
            detail: "the release asset must be a plain file name".to_owned(),
        })
        .into());
    }
    if let DigestDeclaration::ReleaseChecksums { asset } | DigestDeclaration::Sidecar { asset } =
        tool.digest
        && !is_plain_asset(asset)
    {
        return Err(ProvisionError::Fault(crate::Fault::Usage {
            detail: "the digest metadata asset must be a plain file name".to_owned(),
        })
        .into());
    }
    Ok(())
}

fn is_plain_name(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.starts_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn is_plain_asset(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+'))
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
    let (stdout, stdout_truncated) =
        bounded_output(output.stdout(), output.stdout_truncated(), redactor);
    let (stderr, stderr_truncated) =
        bounded_output(output.stderr(), output.stderr_truncated(), redactor);
    let parsed = parse_version(&stdout);
    if output.success()
        && let Some(version) = parsed
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
    if bytes.len() > MAX_DIGEST_METADATA_BYTES {
        return Err(ProvisionError::Refused(if sidecar {
            Refusal::MalformedSidecar {
                asset: asset.to_owned(),
            }
        } else {
            Refusal::MalformedChecksum {
                asset: asset.to_owned(),
            }
        })
        .into());
    }
    String::from_utf8(bytes).map_err(|_| {
        ProvisionError::Refused(if sidecar {
            Refusal::MalformedSidecar {
                asset: asset.to_owned(),
            }
        } else {
            Refusal::MalformedChecksum {
                asset: asset.to_owned(),
            }
        })
        .into()
    })
}

fn snapshot_receipt(path: &Path) -> Result<ReceiptSnapshot, InstallError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(ReceiptSnapshot {
            bytes: Some(fs::read(path).map_err(|error| io_error(path, error))?),
            obstructed: false,
        }),
        Ok(metadata) if metadata.file_type().is_dir() => Ok(ReceiptSnapshot {
            bytes: None,
            obstructed: true,
        }),
        Ok(_) => Err(ProvisionError::Fault(crate::Fault::Usage {
            detail: "the receipt path must be absent, a regular file, or a directory obstruction"
                .to_owned(),
        })
        .into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ReceiptSnapshot {
            bytes: None,
            obstructed: false,
        }),
        Err(error) => Err(io_error(path, error).into()),
    }
}

fn read_stored_receipt(path: &Path) -> io::Result<Option<StoredReceipt>> {
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).ok()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn receipt_path(managed_root: &Path, name: &str) -> PathBuf {
    managed_root
        .join("state")
        .join("tool-provision")
        .join(format!("{name}.json"))
}

fn write_backup(
    binary: &Path,
    backup: &Path,
    bin_dir: &Path,
    name: &str,
) -> Result<(), InstallError> {
    if backup.exists() {
        return Err(InstallFault::RecoveryRequired {
            backup: backup.to_path_buf(),
        }
        .into());
    }
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
    temporary
        .persist_noclobber(backup)
        .map_err(|error| io_error(backup, error.error))?;
    Ok(())
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

fn rollback(cause: InstallError, context: &RollbackContext<'_>) -> InstallError {
    let mut failures = Vec::new();
    if context.had_previous {
        match restore_binary(
            context.backup,
            context.binary,
            context.bin_dir,
            context.name,
        ) {
            Ok(()) => {
                if let Err(error) = remove_if_present(context.backup) {
                    failures.push(format!("{}: {error}", context.backup.display()));
                }
            }
            Err(error) => failures.push(format!("{}: {error}", context.binary.display())),
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

fn restore_binary(backup: &Path, binary: &Path, bin_dir: &Path, name: &str) -> io::Result<()> {
    let mut temporary = tempfile::Builder::new()
        .prefix(&format!(".{name}.restore-"))
        .tempfile_in(bin_dir)?;
    let mut previous = File::open(backup)?;
    io::copy(&mut previous, temporary.as_file_mut())?;
    make_executable(temporary.path())?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(binary)
        .map(|_| ())
        .map_err(|error| error.error)
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

impl From<RunnerError> for InstallError {
    fn from(error: RunnerError) -> Self {
        Self::Fault(InstallFault::ProcessStart {
            program: PathBuf::new(),
            detail: error.to_string(),
        })
    }
}

#[cfg(test)]
mod unit_tests {
    use super::parse_version;
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
}
