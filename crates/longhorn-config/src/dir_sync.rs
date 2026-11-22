//! The one directory-durability barrier.
//!
//! Every site that makes a rename or unlink durable syncs its parent
//! directory through this module — nowhere else. The platform facts that
//! forced the split (Soundcheck Linux and Windows acceptance, 2026-08-22):
//!
//! - **Linux**: cap-std opens directory capabilities with `O_PATH`, and
//!   `fsync(2)` on an `O_PATH` fd is `EBADF`; a capability must be
//!   reopened as a real `O_RDONLY | O_DIRECTORY` fd first. A plain
//!   `std::fs::File::open` on a directory path is already a real fd.
//! - **Windows**: there is no directory-flush operation. `CreateFileW`
//!   without `FILE_FLAG_BACKUP_SEMANTICS` cannot open a directory at all
//!   (`ERROR_ACCESS_DENIED`), and `FlushFileBuffers` is not defined for
//!   directory handles even with it. The documented posture is a no-op:
//!   NTFS journals rename metadata itself, `std`/`tokio` and the
//!   consumers' own sites take the same stance, and
//!   `Durability::FileAndDirectorySynced` on Windows means that platform
//!   guarantee applies — there is no stronger operation to perform.
//! - **macOS and the remaining unixes**: both handle and path forms are
//!   real fds and `sync_all` is valid — byte-identical to the original
//!   behavior.

#[cfg(not(windows))]
use std::fs::File;
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use cap_std::fs::Dir;

/// Creates every missing directory in `relative` below `root` and returns the
/// relative paths (from `root`, where `""` denotes `root` itself) of every
/// parent directory that received a new child entry and therefore must be
/// synced for first-write durability.
pub(crate) fn create_dir_all_collecting_parents(
    root: &Dir,
    relative: &Path,
) -> io::Result<Vec<PathBuf>> {
    let mut current = PathBuf::new();
    let mut parents_to_sync = Vec::new();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            continue;
        };
        let parent = current.clone();
        current.push(segment);
        match root.create_dir(&current) {
            Ok(()) => parents_to_sync.push(parent),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if !root.metadata(&current)?.is_dir() {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(parents_to_sync)
}

/// Creates every missing directory in `path` and syncs every parent directory
/// (including the deepest previously existing ancestor) that received a new
/// child entry.
pub(crate) fn create_dir_all_synced(path: &Path) -> io::Result<()> {
    let mut missing = Vec::new();
    let mut cursor = path;
    while !cursor.exists() {
        missing.push(cursor.to_path_buf());
        let Some(parent) = cursor.parent() else {
            break;
        };
        if parent == cursor || parent.as_os_str().is_empty() {
            break;
        }
        cursor = parent;
    }

    let mut parents_to_sync = Vec::new();
    for directory in missing.into_iter().rev() {
        match fs::create_dir(&directory) {
            Ok(()) => {
                if let Some(parent) = directory.parent()
                    && !parent.as_os_str().is_empty()
                {
                    parents_to_sync.push(parent.to_path_buf());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if !directory.is_dir() {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }

    for parent in parents_to_sync.into_iter().rev() {
        sync_dir_path(&parent)?;
    }
    Ok(())
}

/// Syncs the directory behind a cap-std capability.
#[cfg(target_os = "linux")]
pub(crate) fn sync_dir_handle(parent: &Dir) -> io::Result<()> {
    use rustix::fs::{Mode, OFlags};
    let directory = rustix::fs::openat(
        parent,
        ".",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    rustix::fs::fsync(&directory)?;
    Ok(())
}

/// Syncs the directory behind a cap-std capability.
#[cfg(windows)]
pub(crate) fn sync_dir_handle(_parent: &Dir) -> io::Result<()> {
    Ok(())
}

/// Syncs the directory behind a cap-std capability.
#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) fn sync_dir_handle(parent: &Dir) -> io::Result<()> {
    parent.try_clone()?.into_std_file().sync_all()
}

/// Syncs a directory addressed by path.
#[cfg(not(windows))]
pub(crate) fn sync_dir_path(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

/// Syncs a directory addressed by path.
#[cfg(windows)]
pub(crate) fn sync_dir_path(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use cap_std::ambient_authority;

    use super::*;

    /// Both barrier forms must succeed on every platform against a real
    /// directory — this is the assertion that failed as `EBADF` on Linux
    /// (handle form) and `ERROR_ACCESS_DENIED` on Windows (both forms)
    /// before the platform split.
    #[test]
    fn both_barrier_forms_succeed_on_this_platform() {
        let temp = tempfile::tempdir().unwrap();
        let handle = Dir::open_ambient_dir(temp.path(), ambient_authority()).unwrap();
        sync_dir_handle(&handle).unwrap();
        sync_dir_path(temp.path()).unwrap();
    }

    #[test]
    fn nested_directory_creation_records_every_created_ancestor_and_syncs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Dir::open_ambient_dir(temp.path(), ambient_authority()).unwrap();

        let created = create_dir_all_collecting_parents(&root, Path::new("a/b/c")).unwrap();
        assert_eq!(
            created,
            vec![PathBuf::from(""), PathBuf::from("a"), PathBuf::from("a/b")]
        );
        assert!(temp.path().join("a/b/c").is_dir());

        let repeated = create_dir_all_collecting_parents(&root, Path::new("a/b/c")).unwrap();
        assert!(repeated.is_empty());

        let nested_plain = temp.path().join("plain/x/y/z");
        create_dir_all_synced(&nested_plain).unwrap();
        assert!(nested_plain.is_dir());
        create_dir_all_synced(&nested_plain).unwrap();
    }
}
