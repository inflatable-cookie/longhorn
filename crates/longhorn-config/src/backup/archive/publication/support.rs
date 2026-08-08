use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use super::super::publication_types::{BackupPublicationError, BackupPublicationStage};
use super::super::{BackupArchiveError, BackupArchiveLimits};

const TEMP_ATTEMPTS: u64 = 32;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn create_temporary(
    parent: &Path,
    file_name: &str,
    target: &Path,
) -> Result<(fs::File, PathBuf), BackupPublicationError> {
    for _ in 0..TEMP_ATTEMPTS {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let partial_name = format!(
            ".{file_name}.{}.{}.longhorn-partial",
            std::process::id(),
            sequence
        );
        let path = parent.join(partial_name);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        set_private_mode(&mut options);
        match options.open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(publication_error(
                    BackupPublicationStage::CreateTemporary,
                    target.to_path_buf(),
                    false,
                    error.to_string(),
                ));
            }
        }
    }
    Err(publication_error(
        BackupPublicationStage::CreateTemporary,
        target.to_path_buf(),
        false,
        "temporary name collision retry limit reached",
    ))
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
    temporary: &Path,
) -> BackupPublicationError {
    if let Err(cleanup_error) = fs::remove_file(temporary) {
        error.detail = format!("{}; partial cleanup failed: {cleanup_error}", error.detail);
    }
    error
}

#[cfg(unix)]
fn set_private_mode(options: &mut fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_mode(_options: &mut fs::OpenOptions) {}
