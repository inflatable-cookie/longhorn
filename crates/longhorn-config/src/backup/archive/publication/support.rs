use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::atomic_file::UniqueTemporary;

use super::super::publication_types::{BackupPublicationError, BackupPublicationStage};
use super::super::{BackupArchiveError, BackupArchiveLimits};

pub(super) fn create_temporary(
    parent: &Path,
    file_name: &str,
    target: &Path,
) -> Result<UniqueTemporary, BackupPublicationError> {
    UniqueTemporary::create(parent, file_name).map_err(|error| {
        publication_error(
            BackupPublicationStage::CreateTemporary,
            target.to_path_buf(),
            false,
            error.to_string(),
        )
    })
}

pub fn read_bounded_archive(
    path: &Path,
    limits: BackupArchiveLimits,
) -> Result<Vec<u8>, BackupArchiveError> {
    let mut file = fs::File::open(path).map_err(|error| BackupArchiveError::Read {
        path: path.display().to_string(),
        detail: error.to_string(),
    })?;
    let observed = file
        .metadata()
        .map_err(|error| BackupArchiveError::Read {
            path: path.display().to_string(),
            detail: error.to_string(),
        })?
        .len();
    if observed > limits.max_archive_bytes() as u64 {
        return Err(BackupArchiveError::ArchiveTooLarge {
            limit: limits.max_archive_bytes(),
            observed: usize::try_from(observed).unwrap_or(usize::MAX),
        });
    }
    let mut bytes = Vec::with_capacity(usize::try_from(observed).unwrap_or(0));
    Read::by_ref(&mut file)
        .take(limits.max_archive_bytes() as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| BackupArchiveError::Read {
            path: path.display().to_string(),
            detail: error.to_string(),
        })?;
    if bytes.len() > limits.max_archive_bytes() {
        return Err(BackupArchiveError::ArchiveTooLarge {
            limit: limits.max_archive_bytes(),
            observed: bytes.len(),
        });
    }
    Ok(bytes)
}

pub(super) fn verification_error(
    path: PathBuf,
    published: bool,
    error: BackupArchiveError,
) -> BackupPublicationError {
    BackupPublicationError {
        stage: BackupPublicationStage::VerifyTemporary,
        path,
        published,
        detail: error.to_string(),
        verification: Some(error),
    }
}

pub(super) fn publication_error(
    stage: BackupPublicationStage,
    path: PathBuf,
    published: bool,
    detail: impl Into<String>,
) -> BackupPublicationError {
    BackupPublicationError {
        stage,
        path,
        published,
        verification: None,
        detail: detail.into(),
    }
}

pub(super) fn cleanup(
    mut error: BackupPublicationError,
    temporary: UniqueTemporary,
) -> BackupPublicationError {
    if let Err(cleanup_error) = temporary.discard() {
        error.detail = format!("{}; partial cleanup failed: {cleanup_error}", error.detail);
    }
    error
}
