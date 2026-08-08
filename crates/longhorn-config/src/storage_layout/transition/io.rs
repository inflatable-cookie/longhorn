use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::atomic_file::{UniqueTemporary, sync_directory};

use super::StorageTransitionError;

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
    let mut temporary =
        UniqueTemporary::create(parent, &file_name).map_err(|error| fs_error(parent, error))?;
    temporary
        .write(bytes)
        .map_err(|error| fs_error(temporary.path(), error))?;
    temporary
        .commit(path)
        .map_err(|error| fs_error(path, error))
}

pub(super) fn remove_file(path: &Path) -> Result<(), StorageTransitionError> {
    match fs::remove_file(path) {
        Ok(()) => sync_directory(path.parent().unwrap_or_else(|| Path::new("/")))
            .map_err(|error| fs_error(path, error)),
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

fn fs_error(path: &Path, error: std::io::Error) -> StorageTransitionError {
    StorageTransitionError::Filesystem {
        path: PathBuf::from(path),
        detail: error.to_string(),
    }
}
