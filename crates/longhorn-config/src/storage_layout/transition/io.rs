use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use super::StorageTransitionError;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StorageTransitionError> {
    let parent = path
        .parent()
        .ok_or_else(|| StorageTransitionError::Filesystem {
            path: path.to_path_buf(),
            detail: "target has no parent".into(),
        })?;
    fs::create_dir_all(parent).map_err(|error| fs_error(parent, error))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| StorageTransitionError::Filesystem {
            path: path.to_path_buf(),
            detail: "target has no file name".into(),
        })?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    set_private_mode(&mut options);
    let mut file = options
        .open(&temporary)
        .map_err(|error| fs_error(&temporary, error))?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|error| fs_error(&temporary, error))?;
        file.sync_all()
            .map_err(|error| fs_error(&temporary, error))?;
        drop(file);
        fs::rename(&temporary, path).map_err(|error| fs_error(path, error))?;
        sync_parent(parent)
    })();
    if result.is_err()
        && let Err(error) = fs::remove_file(&temporary)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        longhorn_core::report_best_effort_failure(
            "config.storage-transition.temporary-cleanup",
            error,
        );
    }
    result
}

pub(super) fn remove_file(path: &Path) -> Result<(), StorageTransitionError> {
    match fs::remove_file(path) {
        Ok(()) => sync_parent(path.parent().unwrap_or_else(|| Path::new("/"))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(fs_error(path, error)),
    }
}

pub(super) fn remove_tree(path: &Path) -> Result<(), StorageTransitionError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(fs_error(path, error)),
    }
}

fn sync_parent(path: &Path) -> Result<(), StorageTransitionError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| fs_error(path, error))
}

fn fs_error(path: &Path, error: std::io::Error) -> StorageTransitionError {
    StorageTransitionError::Filesystem {
        path: PathBuf::from(path),
        detail: error.to_string(),
    }
}

#[cfg(unix)]
fn set_private_mode(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_mode(_options: &mut OpenOptions) {}
