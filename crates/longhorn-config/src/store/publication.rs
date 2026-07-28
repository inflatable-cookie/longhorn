use std::{
    ffi::OsStr,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cap_std::{
    ambient_authority,
    fs::{Dir, File, OpenOptions},
};

use crate::ResolvedFile;

use super::{Durability, DurabilityRequirement, PublicationFailure, PublicationStage};

const TEMP_ATTEMPTS: u64 = 32;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn publish(
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
            parent
                .try_clone()
                .and_then(|directory| directory.into_std_file().sync_all())
                .map_err(|error| {
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
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = PathBuf::from(format!(
            ".{display_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
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

#[cfg(test)]
mod tests;
