use std::{
    ffi::OsString,
    fs::{self, File, TryLockError},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use longhorn_update::InstallFailure;

use crate::classify;

const WORKSPACE_FORMAT: &[u8] = b"Longhorn update install workspace v1\n";
// The reserved prefix has no letters, so a filesystem's case folding cannot
// turn a permitted target name into a workspace name.
const WORKSPACE_NAMESPACE: &str = ".--.";

/// Rejects targets in install state before apply can move anything. Check the
/// parent separately because a symlink inside a workspace can resolve outside it.
pub(super) fn check_target(target: &Path) -> Result<(), InstallFailure> {
    if in_workspace_namespace(target) {
        return Err(reserved_target(target));
    }
    let parent = target_parent(target);
    let resolved_parent = fs::canonicalize(parent).map_err(|error| classify(&error, parent))?;
    if in_workspace_namespace(&resolved_parent) {
        return Err(reserved_target(target));
    }

    match fs::canonicalize(target) {
        Ok(resolved) if in_workspace_namespace(&resolved) => Err(reserved_target(target)),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(classify(&error, target)),
    }
}

fn in_workspace_namespace(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .as_encoded_bytes()
            .starts_with(WORKSPACE_NAMESPACE.as_bytes())
    })
}

fn reserved_target(target: &Path) -> InstallFailure {
    InstallFailure::Failed {
        detail: format!(
            "install target {} overlaps the workspace namespace",
            target.display()
        ),
    }
}

fn target_parent(target: &Path) -> &Path {
    target
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// Shared for ordinary applies, exclusive when escalation cannot lock a
/// target workspace. The directory inode remains in place during replacement.
pub(super) struct ParentLock {
    #[cfg(unix)]
    _directory: File,
}

impl ParentLock {
    pub(super) fn shared(target: &Path) -> Result<Self, InstallFailure> {
        #[cfg(unix)]
        {
            Self::acquire(target, File::try_lock_shared)
        }
        #[cfg(not(unix))]
        {
            let _ = target;
            Ok(Self {})
        }
    }

    pub(super) fn into_exclusive(self, target: &Path) -> Result<Self, InstallFailure> {
        #[cfg(unix)]
        {
            // An upgrade on the same handle is platform-dependent. Reopen
            // and have the caller recheck the workspace under the new lock.
            drop(self);
            Self::acquire(target, File::try_lock)
        }
        #[cfg(not(unix))]
        {
            Err(InstallFailure::NotWritable {
                detail: format!(
                    "cannot coordinate privileged replacement of {}",
                    target.display()
                ),
            })
        }
    }

    #[cfg(unix)]
    fn acquire(
        target: &Path,
        lock: impl FnOnce(&File) -> Result<(), TryLockError>,
    ) -> Result<Self, InstallFailure> {
        let parent = target_parent(target);
        let directory = File::open(parent).map_err(|error| classify(&error, parent))?;
        match lock(&directory) {
            Ok(()) => Ok(Self {
                _directory: directory,
            }),
            Err(TryLockError::WouldBlock) => Err(InstallFailure::Failed {
                detail: format!("another update is active in {}", parent.display()),
            }),
            Err(TryLockError::Error(error)) => Err(classify(&error, parent)),
        }
    }
}

/// Per-target install state on the target's volume. Keep the lock file in
/// place so all processes lock the same file, including after a crash.
pub(super) struct InstallWorkspace {
    pub(super) root: PathBuf,
    _lock: File,
}

impl InstallWorkspace {
    /// `None` means the parent is read-only and there is no workspace to
    /// recover or lock before handing off to privileged replacement.
    pub(super) fn acquire(target: &Path) -> Result<Option<Self>, InstallFailure> {
        let name = target.file_name().ok_or_else(|| InstallFailure::Failed {
            detail: "install target has no bundle name".to_owned(),
        })?;
        let mut directory_name = OsString::from(WORKSPACE_NAMESPACE);
        directory_name.push("longhorn-install-");
        directory_name.push(name);
        let root = target.with_file_name(directory_name);

        #[cfg(unix)]
        let created = {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new().mode(0o700).create(&root)
        };
        #[cfg(not(unix))]
        let created = fs::create_dir(&root);

        let initialized = match created {
            Ok(()) => true,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
            Err(error)
                if error.kind() == io::ErrorKind::PermissionDenied && !entry_exists(&root)? =>
            {
                return Ok(None);
            }
            Err(error) => return Err(classify(&error, &root)),
        };

        let lock_path = root.join("longhorn-update-install-v1.lock");
        if !initialized
            && !(fs::symlink_metadata(&root).is_ok_and(|metadata| metadata.is_dir())
                && fs::symlink_metadata(&lock_path).is_ok_and(|metadata| metadata.is_file()))
        {
            return Err(InstallFailure::Failed {
                detail: format!(
                    "install workspace at {} is unrecognized or still initializing",
                    root.display(),
                ),
            });
        }

        // Only the process that created the directory may initialize it.
        let mut lock = File::options()
            .read(true)
            .write(true)
            .create_new(initialized)
            .open(&lock_path)
            .map_err(|error| classify(&error, &lock_path))?;
        if initialized {
            lock.write_all(WORKSPACE_FORMAT)
                .map_err(|error| classify(&error, &lock_path))?;
        } else {
            let mut format = Vec::new();
            (&lock)
                .take((WORKSPACE_FORMAT.len() + 1) as u64)
                .read_to_end(&mut format)
                .map_err(|error| classify(&error, &lock_path))?;
            if format != WORKSPACE_FORMAT {
                let backup = root.join("previous");
                return Err(InstallFailure::Failed {
                    detail: format!(
                        "install workspace at {} is unrecognized or still initializing. Inspect {} for a previous bundle",
                        root.display(),
                        backup.display(),
                    ),
                });
            }
        }
        match lock.try_lock() {
            Ok(()) => Ok(Some(Self { root, _lock: lock })),
            Err(TryLockError::WouldBlock) => Err(InstallFailure::Failed {
                detail: format!("another update is active for {}", target.display()),
            }),
            Err(TryLockError::Error(error)) => Err(classify(&error, &lock_path)),
        }
    }

    pub(super) fn backup(&self) -> Backup<'_> {
        Backup {
            root: &self.root,
            bundle: self.root.join("previous"),
            receipt: self.root.join("previous.identity"),
        }
    }
}

pub(super) struct Backup<'workspace> {
    root: &'workspace Path,
    pub(super) bundle: PathBuf,
    receipt: PathBuf,
}

impl Backup<'_> {
    /// Record the destination's identity before moving it. A crash before the
    /// move leaves only a receipt, and one after the move leaves both entries.
    pub(super) fn record(&self, target: &Path) -> Result<(), InstallFailure> {
        let metadata = match fs::symlink_metadata(target) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(classify(&error, target)),
        };
        if let Some(identity) = self.identity(&metadata) {
            let mut receipt = tempfile::NamedTempFile::new_in(self.root)
                .map_err(|error| classify(&error, &self.receipt))?;
            receipt
                .write_all(&identity)
                .map_err(|error| classify(&error, &self.receipt))?;
            receipt
                .persist(&self.receipt)
                .map_err(|error| classify(&error.error, &self.receipt))?;
        }
        Ok(())
    }

    pub(super) fn restore(&self, target: &Path) -> Result<(), InstallFailure> {
        let metadata = match fs::symlink_metadata(&self.bundle) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(classify(&error, &self.bundle)),
        };
        if entry_exists(target)? {
            return Err(InstallFailure::Failed {
                detail: format!(
                    "{} already exists. Inspect and move the previous bundle at {} before retrying",
                    target.display(),
                    self.bundle.display(),
                ),
            });
        }
        if !self.matches(&metadata) {
            return Err(InstallFailure::Failed {
                detail: format!(
                    "could not verify the previous bundle at {}. Inspect it before restoring {}",
                    self.bundle.display(),
                    target.display(),
                ),
            });
        }
        fs::rename(&self.bundle, target).map_err(|error| InstallFailure::Failed {
            detail: format!(
                "could not restore {} from {}: {error}",
                target.display(),
                self.bundle.display(),
            ),
        })?;
        self.clear_receipt();
        Ok(())
    }

    fn matches(&self, metadata: &fs::Metadata) -> bool {
        let Some(identity) = self.identity(metadata) else {
            return false;
        };
        if !fs::symlink_metadata(&self.receipt).is_ok_and(|metadata| metadata.is_file()) {
            return false;
        }
        let mut recorded = Vec::new();
        File::open(&self.receipt)
            .and_then(|file| {
                file.take((identity.len() + 1) as u64)
                    .read_to_end(&mut recorded)
            })
            .is_ok()
            && recorded == identity
    }

    #[cfg(unix)]
    fn identity(&self, metadata: &fs::Metadata) -> Option<Vec<u8>> {
        let workspace = file_identity(&fs::symlink_metadata(self.root).ok()?)?;
        let bundle = file_identity(metadata)?;
        let mut identity = format!("{workspace}{bundle}").into_bytes();
        // Bind the receipt to the workspace location as well as its inode.
        let root = fs::canonicalize(self.root).ok()?;
        identity.extend_from_slice(root.as_os_str().as_encoded_bytes());
        Some(identity)
    }

    #[cfg(not(unix))]
    fn identity(&self, _metadata: &fs::Metadata) -> Option<Vec<u8>> {
        None
    }

    /// Retire the receipt before reclaiming a displaced bundle, so it cannot
    /// match a later allocation. Cleanup after publication is best effort.
    pub(super) fn clear_receipt(&self) -> bool {
        match fs::remove_file(&self.receipt) {
            Ok(()) => true,
            Err(error) => error.kind() == io::ErrorKind::NotFound,
        }
    }
}

#[cfg(unix)]
fn file_identity(metadata: &fs::Metadata) -> Option<String> {
    use std::{os::unix::fs::MetadataExt, time::UNIX_EPOCH};

    let birth = metadata.created().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(format!(
        "{}:{}:{}:{}\n",
        metadata.dev(),
        metadata.ino(),
        birth.as_secs(),
        birth.subsec_nanos(),
    ))
}

/// Includes dangling symlinks when deciding whether an entry is occupied.
pub(super) fn entry_exists(path: &Path) -> Result<bool, InstallFailure> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(classify(&error, path)),
    }
}
