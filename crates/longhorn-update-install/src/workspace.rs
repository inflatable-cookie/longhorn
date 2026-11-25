use std::{
    ffi::OsString,
    fs::{self, File, TryLockError},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use longhorn_update::InstallFailure;

use crate::classify;

const WORKSPACE_FORMAT: &[u8] = b"Longhorn update install workspace v1\n";
// Written after the new bundle reaches the target, so a bundle left beside it
// is the remains of a finished swap rather than a rollback to preserve.
const PUBLISHED_FORMAT: &[u8] = b"Longhorn update install published v1\n";
// The reserved prefix has no letters, so a filesystem's case folding cannot
// turn a permitted target name into a workspace name.
const WORKSPACE_NAMESPACE: &str = ".--.";

/// Resolve the parent used for filesystem operations and keep the final entry
/// name, since replacement moves the entry itself, including a symlink.
fn resolve_target(target: &Path) -> Result<PathBuf, InstallFailure> {
    if in_workspace_namespace(target) {
        return Err(reserved_target(target));
    }
    let name = target.file_name().ok_or_else(|| InstallFailure::Failed {
        detail: "install target has no bundle name".to_owned(),
    })?;
    let parent = target_parent(target);
    let resolved_parent = fs::canonicalize(parent).map_err(|error| classify(&error, parent))?;
    // The final entry can resolve outside a workspace even if its parent is inside.
    if in_workspace_namespace(&resolved_parent) {
        return Err(reserved_target(target));
    }

    match fs::canonicalize(target) {
        Ok(resolved) if in_workspace_namespace(&resolved) => return Err(reserved_target(target)),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(classify(&error, target)),
    }
    Ok(resolved_parent.join(name))
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

/// A target and the directory locks held through its entire apply.
pub(super) struct LockedTarget {
    pub(super) path: PathBuf,
    #[cfg(unix)]
    _directories: Vec<File>,
}

impl LockedTarget {
    #[cfg(unix)]
    pub(super) fn acquire(target: &Path) -> Result<Self, InstallFailure> {
        let path = resolve_target(target)?;
        let parent = target_parent(&path);
        let mut ancestors: Vec<_> = parent.ancestors().collect();
        ancestors.reverse();

        // Each shared lock prevents an apply from replacing the next directory.
        // The exclusive parent lock also covers installs with no writable workspace.
        let mut directories = Vec::with_capacity(ancestors.len());
        for ancestor in ancestors {
            let directory = File::open(ancestor).map_err(|error| classify(&error, ancestor))?;
            let locked = if ancestor == parent {
                directory.try_lock()
            } else {
                directory.try_lock_shared()
            };
            match locked {
                Ok(()) => directories.push(directory),
                Err(TryLockError::WouldBlock) => {
                    return Err(InstallFailure::Failed {
                        detail: format!("another update is active in {}", ancestor.display()),
                    });
                }
                Err(TryLockError::Error(error)) => return Err(classify(&error, ancestor)),
            }
        }

        // A competing apply may have changed a path before we reached its lock.
        // Recheck the caller's path and the resolved spelling we will actually use.
        if resolve_target(target)? != path || resolve_target(&path)? != path {
            return Err(InstallFailure::Failed {
                detail: format!("install target parent changed for {}", target.display()),
            });
        }
        Ok(Self {
            path,
            _directories: directories,
        })
    }

    #[cfg(not(unix))]
    pub(super) fn acquire(target: &Path) -> Result<Self, InstallFailure> {
        let path = resolve_target(target)?;
        Err(InstallFailure::NotWritable {
            detail: format!("cannot lock install directory for {}", path.display()),
        })
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
            published: self.root.join("previous.published"),
        }
    }
}

pub(super) struct Backup<'workspace> {
    root: &'workspace Path,
    pub(super) bundle: PathBuf,
    receipt: PathBuf,
    published: PathBuf,
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
            // The receipt belongs to this displacement. A publication marker
            // left by an interrupted cleanup describes the previous one.
            self.clear_published();
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
            // Both entries present means one of two states. The receipt names
            // the bundle this workspace displaced and the publication marker
            // says the new bundle reached the target, so cleanup did not
            // finish and the leftover is reclaimable. Without that marker a
            // rollback left it here for recovery, and it stays for inspection.
            if self.is_published() && self.matches(&metadata) {
                return self.reclaim();
            }
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

    /// Retire the receipt and the publication marker together, so neither can
    /// match a later allocation. Removing these is bookkeeping after the fact:
    /// a leftover receipt cannot authorize a bundle whose identity differs.
    pub(super) fn clear_receipt(&self) {
        let _ = remove_if_present(&self.receipt);
        self.clear_published();
    }

    /// Marks that the new bundle reached the target.
    ///
    /// Written after publication and before the displaced bundle is removed, so
    /// an interrupted or failed cleanup leaves a state the next apply can
    /// identify. Without it, "the target and a bundle are both here" is
    /// ambiguous: a failed rollback leaves the same two entries, and that
    /// bundle is somebody's only copy.
    fn mark_published(&self) -> io::Result<()> {
        let mut marker = tempfile::NamedTempFile::new_in(self.root)?;
        marker.write_all(PUBLISHED_FORMAT)?;
        marker
            .persist(&self.published)
            .map_err(|error| error.error)?;
        Ok(())
    }

    fn is_published(&self) -> bool {
        fs::read(&self.published).is_ok_and(|marker| marker == PUBLISHED_FORMAT)
    }

    fn clear_published(&self) {
        let _ = remove_if_present(&self.published);
    }

    /// Removes the displaced bundle after the new one is published.
    ///
    /// The failure is reported rather than dropped: it is the only thing that
    /// would keep the next apply from starting, and it is recoverable there —
    /// the marker and the receipt stay, so [`Backup::restore`] reclaims the
    /// leftover instead of refusing to run.
    pub(super) fn retire(&self) -> Result<(), InstallFailure> {
        // A fault here is neither `NotWritable` nor malformed: the bundle is
        // installed. The receipt alone cannot say whether the leftover is
        // reclaimable, so this reports what a human has to look at.
        self.mark_published()
            .map_err(|error| InstallFailure::Failed {
                detail: format!(
                    "the update is installed, but the completed swap in {} could not be recorded \
                     ({error}). Inspect {} before the next apply",
                    self.root.display(),
                    self.bundle.display(),
                ),
            })?;
        self.remove_bundle()
            .map_err(|error| InstallFailure::Failed {
                detail: format!(
                    "the update is installed, but the previous bundle at {} could not be removed \
                     ({error}). The next apply reclaims it",
                    self.bundle.display(),
                ),
            })?;
        self.clear_receipt();
        Ok(())
    }

    /// Removes a leftover bundle the receipt and the marker both vouch for.
    fn reclaim(&self) -> Result<(), InstallFailure> {
        self.remove_bundle()
            .map_err(|error| InstallFailure::Failed {
                detail: format!(
                    "could not reclaim the previous bundle at {}: {error}",
                    self.bundle.display(),
                ),
            })?;
        self.clear_receipt();
        Ok(())
    }

    fn remove_bundle(&self) -> io::Result<()> {
        match fs::remove_dir_all(&self.bundle) {
            Ok(()) => Ok(()),
            // Already gone: a crash between the removal and the receipt.
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
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
