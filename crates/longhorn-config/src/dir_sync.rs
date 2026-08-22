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
use std::{io, path::Path};

use cap_std::fs::Dir;

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
}
