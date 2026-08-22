use std::{
    ffi::OsStr,
    io::{self, Write},
    path::{Path, PathBuf},
};

use cap_std::{
    ambient_authority,
    fs::{Dir, File, OpenOptions},
};

use crate::ResolvedFile;
use crate::atomic_file::{TEMP_ATTEMPTS, temporary_name};

use super::{Durability, DurabilityRequirement, PublicationFailure, PublicationStage};

pub(crate) fn publish(
    target: &ResolvedFile,
    bytes: &[u8],
    requirement: DurabilityRequirement,
) -> Result<Durability, PublicationFailure> {
    publish_inner(target, bytes, requirement, None)
}

fn publish_inner(
    target: &ResolvedFile,
    bytes: &[u8],
    requirement: DurabilityRequirement,
    injected_failure: Option<PublicationStage>,
) -> Result<Durability, PublicationFailure> {
    inject(target, PublicationStage::OpenRoot, injected_failure)?;
    let root = Dir::open_ambient_dir(target.root(), ambient_authority())
        .map_err(|error| failure(target, PublicationStage::OpenRoot, false, error.to_string()))?;
    let relative = target.relative_path().as_path();
    let parent_path = relative.parent().unwrap_or_else(|| Path::new(""));

    if !parent_path.as_os_str().is_empty() {
        inject(target, PublicationStage::CreateParent, injected_failure)?;
        root.create_dir_all(parent_path).map_err(|error| {
            failure(
                target,
                PublicationStage::CreateParent,
                false,
                error.to_string(),
            )
        })?;
    }

    inject(target, PublicationStage::OpenParent, injected_failure)?;
    let parent = if parent_path.as_os_str().is_empty() {
        root.try_clone()
    } else {
        root.open_dir(parent_path)
    }
    .map_err(|error| {
        failure(
            target,
            PublicationStage::OpenParent,
            false,
            error.to_string(),
        )
    })?;

    let file_name = relative.file_name().ok_or_else(|| {
        failure(
            target,
            PublicationStage::CreateTemporary,
            false,
            "target has no file name",
        )
    })?;
    let (mut temporary, temporary_name) =
        create_temporary(target, &parent, file_name, injected_failure)?;

    if let Err(error) =
        inject(target, PublicationStage::WriteTemporary, injected_failure).and_then(|()| {
            temporary.write_all(bytes).map_err(|error| {
                failure(
                    target,
                    PublicationStage::WriteTemporary,
                    false,
                    error.to_string(),
                )
            })
        })
    {
        drop(temporary);
        return Err(cleanup_failure(error, &parent, &temporary_name));
    }

    if let Err(error) =
        inject(target, PublicationStage::SyncTemporary, injected_failure).and_then(|()| {
            temporary.sync_all().map_err(|error| {
                failure(
                    target,
                    PublicationStage::SyncTemporary,
                    false,
                    error.to_string(),
                )
            })
        })
    {
        drop(temporary);
        return Err(cleanup_failure(error, &parent, &temporary_name));
    }
    drop(temporary);

    if let Err(error) = inject(target, PublicationStage::Rename, injected_failure).and_then(|()| {
        parent
            .rename(&temporary_name, &parent, file_name)
            .map_err(|error| failure(target, PublicationStage::Rename, false, error.to_string()))
    }) {
        return Err(cleanup_failure(error, &parent, &temporary_name));
    }

    let directory_sync = inject(target, PublicationStage::SyncDirectory, injected_failure)
        .and_then(|()| {
            sync_directory(&parent).map_err(|error| {
                failure(
                    target,
                    PublicationStage::SyncDirectory,
                    true,
                    error.to_string(),
                )
            })
        });

    match (directory_sync, requirement) {
        (Ok(()), _) => Ok(Durability::FileAndDirectorySynced),
        (Err(_), DurabilityRequirement::Atomic) => Ok(Durability::FileSynced),
        (Err(error), DurabilityRequirement::Durable) => Err(error),
    }
}

fn create_temporary(
    target: &ResolvedFile,
    parent: &Dir,
    file_name: &OsStr,
    injected_failure: Option<PublicationStage>,
) -> Result<(File, PathBuf), PublicationFailure> {
    inject(target, PublicationStage::CreateTemporary, injected_failure)?;
    let display_name = file_name.to_string_lossy();

    for _ in 0..TEMP_ATTEMPTS {
        let name = temporary_name(Path::new(""), &display_name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        set_private_mode(&mut options);

        match parent.open_with(&name, &options) {
            Ok(file) => return Ok((file, name)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(failure(
                    target,
                    PublicationStage::CreateTemporary,
                    false,
                    error.to_string(),
                ));
            }
        }
    }

    Err(failure(
        target,
        PublicationStage::CreateTemporary,
        false,
        "temporary name collision retry limit reached",
    ))
}

fn inject(
    target: &ResolvedFile,
    stage: PublicationStage,
    injected_failure: Option<PublicationStage>,
) -> Result<(), PublicationFailure> {
    if injected_failure == Some(stage) {
        Err(failure(
            target,
            stage,
            stage == PublicationStage::SyncDirectory,
            "injected failure",
        ))
    } else {
        Ok(())
    }
}

fn cleanup_failure(
    mut primary: PublicationFailure,
    parent: &Dir,
    temporary_name: &Path,
) -> PublicationFailure {
    if let Err(error) = parent.remove_file(temporary_name) {
        primary.detail = format!("{}; temporary cleanup failed: {error}", primary.detail);
    }
    primary
}

fn failure(
    target: &ResolvedFile,
    stage: PublicationStage,
    published: bool,
    detail: impl Into<String>,
) -> PublicationFailure {
    PublicationFailure {
        stage,
        path: target.full_path().to_path_buf(),
        published,
        detail: detail.into(),
    }
}

#[cfg(unix)]
fn set_private_mode(options: &mut OpenOptions) {
    use cap_std::fs::OpenOptionsExt;

    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_mode(_options: &mut OpenOptions) {}

/// Fsyncs the parent directory so the rename itself is durable.
///
/// On Linux, cap-std opens directory capabilities with `O_PATH`, and
/// `fsync(2)` on an `O_PATH` fd is `EBADF` — so cloning the `Dir` and
/// syncing it fails unconditionally there (Soundcheck Linux acceptance,
/// 2026-08-22). Reopen `.` relative to the capability as a real
/// `O_RDONLY | O_DIRECTORY` fd and fsync that. macOS has no `O_PATH`
/// directory handles, so the original clone-and-sync stays byte-identical
/// off Linux.
#[cfg(target_os = "linux")]
fn sync_directory(parent: &Dir) -> io::Result<()> {
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

/// Windows has no directory-flush operation: `FlushFileBuffers` on a
/// directory handle fails with `ERROR_ACCESS_DENIED` (cap-std handles lack
/// `FILE_FLAG_BACKUP_SEMANTICS`, and the call is not defined for directory
/// handles even with it), which failed every `Durable` publication there
/// (Soundcheck Windows acceptance, its g04 card 144, 2026-08-22). The
/// documented posture is a no-op: NTFS journals the rename metadata
/// itself, `std`/`tokio` and Soundcheck's own directory-sync sites take
/// the same stance, and `Durability::FileAndDirectorySynced` on Windows
/// means the platform's directory-durability guarantee applies — there is
/// no stronger operation to perform.
#[cfg(windows)]
fn sync_directory(_parent: &Dir) -> io::Result<()> {
    Ok(())
}

/// See the Linux and Windows variants: everywhere else (macOS and the
/// remaining unixes) the cloned handle is a real fd and `sync_all` is
/// valid on it — byte-identical to the original behavior.
#[cfg(not(any(target_os = "linux", windows)))]
fn sync_directory(parent: &Dir) -> io::Result<()> {
    parent.try_clone()?.into_std_file().sync_all()
}

#[cfg(test)]
mod tests;
