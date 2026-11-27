use std::{fs, io, path::Path};

use crate::ExportOverwrite;

#[derive(Debug)]
pub(super) enum CommitError {
    AtomicNoReplaceUnavailable(io::Error),
    Io(io::Error),
}

pub(super) fn commit_name(
    staging: &Path,
    target: &Path,
    overwrite: ExportOverwrite,
) -> Result<(), CommitError> {
    match overwrite {
        ExportOverwrite::Refuse => {
            commit_no_replace(staging, target, rename_no_replace, |from, to| {
                fs::hard_link(from, to)
            })
        }
        ExportOverwrite::Replace => fs::rename(staging, target).map_err(CommitError::Io),
    }
}

pub(super) fn commit_no_replace(
    staging: &Path,
    target: &Path,
    rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
    link: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<(), CommitError> {
    match rename(staging, target) {
        Ok(()) => return Ok(()),
        Err(error) if rename_unsupported(&error) => {}
        Err(error) => return Err(CommitError::Io(error)),
    }

    // A hard link can publish the same fully staged file without replacing a name.
    match link(staging, target) {
        Ok(()) => {
            if let Err(error) = fs::remove_file(staging) {
                longhorn_core::report_best_effort_failure("config.backup.partial-cleanup", error);
            }
            Ok(())
        }
        // Filesystems without hard-link support may report EPERM.
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::Unsupported | io::ErrorKind::PermissionDenied
            ) =>
        {
            Err(CommitError::AtomicNoReplaceUnavailable(error))
        }
        Err(error) => Err(CommitError::Io(error)),
    }
}

fn rename_unsupported(error: &io::Error) -> bool {
    match error.kind() {
        io::ErrorKind::Unsupported => true,
        // Linux also uses EINVAL for a filesystem without support for this flag.
        // Both paths are sibling regular-file names and the flag is fixed.
        #[cfg(target_os = "linux")]
        io::ErrorKind::InvalidInput => {
            error.raw_os_error() == Some(rustix::io::Errno::INVAL.raw_os_error())
        }
        _ => false,
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_no_replace(staging: &Path, target: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};

    renameat_with(CWD, staging, CWD, target, RenameFlags::NOREPLACE).map_err(|error| {
        use rustix::io::Errno;
        if error == Errno::NOSYS || error == Errno::NOTSUP || error == Errno::OPNOTSUPP {
            io::ErrorKind::Unsupported.into()
        } else {
            error.into()
        }
    })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_no_replace(_staging: &Path, _target: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}
