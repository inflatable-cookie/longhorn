use std::path::{Path, PathBuf};

use crate::{
    atomic_file::UniqueTemporary,
    bounded_file::{self, BoundedReadError},
};

use super::super::BackupArchiveError;
use super::super::publication_types::{BackupPublicationError, BackupPublicationStage};

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

pub(crate) fn read_archive_bytes(path: &Path, limit: usize) -> Result<Vec<u8>, BackupArchiveError> {
    bounded_file::read_path(path, limit).map_err(|error| match error {
        BoundedReadError::TooLarge { limit, observed } => {
            BackupArchiveError::ArchiveTooLarge { limit, observed }
        }
        BoundedReadError::Io(error) => BackupArchiveError::Read {
            path: path.display().to_string(),
            detail: error.to_string(),
        },
    })
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
