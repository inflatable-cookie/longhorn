use std::{
    ffi::OsString,
    fs::{self, File, TryLockError},
    io,
    path::{Path, PathBuf},
};

use longhorn_update::InstallFailure;

use crate::classify;

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
        let mut directory_name = OsString::from(".longhorn-install-");
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
        let lock = File::options()
            .read(true)
            .write(true)
            .create_new(initialized)
            .open(&lock_path)
            .map_err(|error| classify(&error, &lock_path))?;
        match lock.try_lock() {
            Ok(()) => Ok(Some(Self { root, _lock: lock })),
            Err(TryLockError::WouldBlock) => Err(InstallFailure::Failed {
                detail: format!("another update is active for {}", target.display()),
            }),
            Err(TryLockError::Error(error)) => Err(classify(&error, &lock_path)),
        }
    }
}

/// Includes dangling symlinks when deciding whether an entry is occupied.
pub(super) fn entry_exists(path: &Path) -> Result<bool, InstallFailure> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(classify(&error, path)),
    }
}
