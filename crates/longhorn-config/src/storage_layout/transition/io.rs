use std::{
    fs, io,
    path::{Path, PathBuf},
};

use cap_std::{ambient_authority, fs::Dir};

use crate::atomic_file::{UniqueTemporary, sync_directory};

use super::StorageTransitionError;

/// Reads a regular transition file below its declared authority.
pub(super) fn read(
    root: &Path,
    relative: &Path,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, StorageTransitionError> {
    let path = root.join(relative);
    let result = (|| {
        let directory = Dir::open_ambient_dir(root, ambient_authority())?;
        let metadata = directory.symlink_metadata(relative)?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected a regular file",
            ));
        }
        let bytes = crate::store::load::read_bounded_in(&directory, relative, max_bytes)?;
        if bytes.len() as u64 != metadata.len() {
            return Err(io::Error::other(
                "file changed while transition evidence was read",
            ));
        }
        Ok(bytes)
    })();
    match result {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
            Err(StorageTransitionError::BoundExceeded { path })
        }
        Err(error) => Err(fs_error(&path, error)),
    }
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StorageTransitionError> {
    let parent = path
        .parent()
        .ok_or_else(|| StorageTransitionError::Filesystem {
            path: path.to_path_buf(),
            detail: "target has no parent".into(),
        })?;
    crate::dir_sync::create_dir_all_synced(parent).map_err(|error| fs_error(parent, error))?;
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
