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

/// Returns every ancestor relative to a root capability above `relative_parent`
/// in bottom-up order (ending at `""`, which denotes the root capability).
///
/// For `relative_parent = "a/b/c"`, returns `["a/b", "a", ""]`.
/// For `relative_parent = ""`, returns `[]`.
pub(crate) fn ancestor_chain_in_dir(relative_parent: &Path) -> Vec<PathBuf> {
    let mut ancestors = Vec::new();
    let mut current = PathBuf::new();
    for component in relative_parent.components() {
        let Component::Normal(segment) = component else {
            continue;
        };
        ancestors.push(current.clone());
        current.push(segment);
    }
    ancestors.reverse();
    ancestors
}

/// Returns every directory from `directory` up to `root` (inclusive) in
/// bottom-up order. If `directory` is not below `root`, returns `[directory]`.
pub(crate) fn ancestor_chain_to_root(root: &Path, directory: &Path) -> Vec<PathBuf> {
    let Ok(relative) = directory.strip_prefix(root) else {
        return vec![directory.to_path_buf()];
    };
    let mut chain = vec![root.to_path_buf()];
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            continue;
        };
        current.push(segment);
        chain.push(current.clone());
    }
    chain.reverse();
    chain
}

/// Ensures `directory` exists and syncs every directory from `directory` up to
/// `root` (inclusive) in bottom-up order.
///
/// Always syncing the full bounded chain to `root` guarantees that both a first
/// write (even when an earlier helper already created intermediate directories)
/// and a retry after a prior sync failure make every ancestor durable.
pub(crate) fn create_dir_all_synced_to_root(root: &Path, directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    sync_ancestors_to_root(root, directory)
}

/// Syncs every directory from `directory` up to `root` (inclusive) in
/// bottom-up order.
pub(crate) fn sync_ancestors_to_root(root: &Path, directory: &Path) -> io::Result<()> {
    for ancestor in ancestor_chain_to_root(root, directory) {
        sync_dir_path(&ancestor)?;
    }
    Ok(())
}

/// Creates missing directories in order, syncing each entry before descending.
///
/// Rechecks the parent of the deepest existing directory so a retry covers
/// an entry whose creation succeeded just before a failed sync.
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

    sync_entry_parent(cursor)?;
    for directory in missing.into_iter().rev() {
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && directory.is_dir() => {}
            Err(error) => return Err(error),
        }
        sync_entry_parent(&directory)?;
    }
    Ok(())
}

fn sync_entry_parent(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => sync_dir_path(parent),
        _ => Ok(()),
    }
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

        assert_eq!(
            ancestor_chain_in_dir(Path::new("a/b/c")),
            vec![PathBuf::from("a/b"), PathBuf::from("a"), PathBuf::from("")]
        );
        assert!(ancestor_chain_in_dir(Path::new("")).is_empty());

        let rollback_dir = temp.path().join(".longhorn/restore/rollback");
        assert_eq!(
            ancestor_chain_to_root(temp.path(), &rollback_dir),
            vec![
                rollback_dir.clone(),
                temp.path().join(".longhorn/restore"),
                temp.path().join(".longhorn"),
                temp.path().to_path_buf(),
            ]
        );
        create_dir_all_synced_to_root(temp.path(), &rollback_dir).unwrap();
        assert!(rollback_dir.is_dir());
        // Retrying after directories already exist still syncs the full chain.
        create_dir_all_synced_to_root(temp.path(), &rollback_dir).unwrap();

        let nested_plain = temp.path().join("plain/x/y/z");
        create_dir_all_synced(&nested_plain).unwrap();
        assert!(nested_plain.is_dir());
        create_dir_all_synced(&nested_plain).unwrap();
    }
}
