use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{Durability, DurabilityRequirement};

use super::{
    BackupArchiveError, BackupArchiveLimits, EncodedBackupArchive, inspect_backup_archive,
    publication_types::{
        BackupArchiveFileName, BackupDestinationKind, BackupExportTarget, BackupOperationalRoot,
        BackupPublicationError, BackupPublicationOptions, BackupPublicationReceipt,
        BackupPublicationStage, ExportOverwrite,
    },
};

const TEMP_ATTEMPTS: u64 = 32;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Publishes one verified archive below the operational backup root.
pub fn publish_operational_backup(
    root: &BackupOperationalRoot,
    file_name: &BackupArchiveFileName,
    archive: &EncodedBackupArchive,
    options: BackupPublicationOptions,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    publish(
        root.path(),
        file_name.as_str(),
        archive,
        BackupDestinationKind::Operational,
        false,
        options,
        false,
    )
}

/// Publishes one verified archive to an explicit user-selected destination.
pub fn export_backup(
    target: &BackupExportTarget,
    archive: &EncodedBackupArchive,
    overwrite: ExportOverwrite,
    options: BackupPublicationOptions,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    if !target.parent().is_dir() {
        return Err(publication_error(
            BackupPublicationStage::ValidateDestination,
            target.path(),
            false,
            "export parent does not exist or is not a directory",
        ));
    }
    publish(
        target.parent(),
        target.file_name().as_str(),
        archive,
        BackupDestinationKind::UserExport,
        overwrite == ExportOverwrite::Replace,
        options,
        false,
    )
}

fn publish(
    parent_path: &Path,
    file_name: &str,
    archive: &EncodedBackupArchive,
    destination: BackupDestinationKind,
    allow_overwrite: bool,
    options: BackupPublicationOptions,
    corrupt_staging: bool,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    let inspection = inspect_backup_archive(archive.bytes(), options.archive_limits)
        .map_err(|error| verification_error(parent_path.join(file_name), false, error))?;
    let target = parent_path.join(file_name);
    let archive_kind = inspection.manifest().kind();
    let kind_mismatch = match destination {
        BackupDestinationKind::Operational => archive_kind == crate::BackupKind::UserExport,
        BackupDestinationKind::UserExport => archive_kind != crate::BackupKind::UserExport,
    };
    if kind_mismatch {
        return Err(publication_error(
            BackupPublicationStage::ValidateDestination,
            target,
            false,
            "archive kind does not match destination authority",
        ));
    }
    if destination == BackupDestinationKind::Operational {
        fs::create_dir_all(parent_path).map_err(|error| {
            publication_error(
                BackupPublicationStage::CreateRoot,
                parent_path.to_path_buf(),
                false,
                error.to_string(),
            )
        })?;
    }
    let existed = target.exists();
    if existed && !allow_overwrite {
        return Err(publication_error(
            BackupPublicationStage::ValidateDestination,
            target,
            false,
            "destination already exists and overwrite was not authorized",
        ));
    }
    let parent = fs::File::open(parent_path).map_err(|error| {
        publication_error(
            BackupPublicationStage::OpenParent,
            target.clone(),
            false,
            error.to_string(),
        )
    })?;
    let (mut temporary, temporary_path) = create_temporary(parent_path, file_name, &target)?;
    let staged_bytes = if corrupt_staging {
        &archive.bytes()[..archive.bytes().len().saturating_sub(1)]
    } else {
        archive.bytes()
    };
    if let Err(error) = temporary.write_all(staged_bytes) {
        drop(temporary);
        return Err(cleanup(
            publication_error(
                BackupPublicationStage::WriteTemporary,
                target,
                false,
                error.to_string(),
            ),
            &temporary_path,
        ));
    }
    if let Err(error) = temporary.sync_all() {
        drop(temporary);
        return Err(cleanup(
            publication_error(
                BackupPublicationStage::SyncTemporary,
                target,
                false,
                error.to_string(),
            ),
            &temporary_path,
        ));
    }
    drop(temporary);

    let verified = match read_bounded_archive(&temporary_path, options.archive_limits)
        .and_then(|bytes| inspect_backup_archive(&bytes, options.archive_limits))
    {
        Ok(inspection) => inspection,
        Err(error) => {
            return Err(cleanup(
                verification_error(target, false, error),
                &temporary_path,
            ));
        }
    };
    if verified.archive_sha256() != archive.sha256() {
        return Err(cleanup(
            publication_error(
                BackupPublicationStage::VerifyTemporary,
                target,
                false,
                "staged archive hash changed",
            ),
            &temporary_path,
        ));
    }
    if target.exists() && !allow_overwrite {
        return Err(cleanup(
            publication_error(
                BackupPublicationStage::ValidateDestination,
                target,
                false,
                "destination appeared before rename",
            ),
            &temporary_path,
        ));
    }
    if let Err(error) = fs::rename(&temporary_path, &target) {
        return Err(cleanup(
            publication_error(
                BackupPublicationStage::Rename,
                target,
                false,
                error.to_string(),
            ),
            &temporary_path,
        ));
    }

    let sync = parent.sync_all().map_err(|error| {
        publication_error(
            BackupPublicationStage::SyncDirectory,
            target.clone(),
            true,
            error.to_string(),
        )
    });
    let durability = match (sync, options.durability) {
        (Ok(()), _) => Durability::FileAndDirectorySynced,
        (Err(_), DurabilityRequirement::Atomic) => Durability::FileSynced,
        (Err(error), DurabilityRequirement::Durable) => return Err(error),
    };
    Ok(BackupPublicationReceipt {
        path: target,
        destination,
        archive_sha256: archive.sha256().clone(),
        durability,
        replaced_existing: existed,
    })
}

fn create_temporary(
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

pub(super) fn read_bounded_archive(
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

fn verification_error(
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

fn publication_error(
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

fn cleanup(mut error: BackupPublicationError, temporary: &Path) -> BackupPublicationError {
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

#[cfg(test)]
pub(super) fn publish_corrupt_staging_for_test(
    root: &BackupOperationalRoot,
    file_name: &BackupArchiveFileName,
    archive: &EncodedBackupArchive,
    options: BackupPublicationOptions,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    fs::create_dir_all(root.path()).unwrap();
    publish(
        root.path(),
        file_name.as_str(),
        archive,
        BackupDestinationKind::Operational,
        false,
        options,
        true,
    )
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use serde_json::json;
    use tempfile::TempDir;
    use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

    use super::*;

    #[test]
    fn failed_reopen_verification_cleans_partial_and_publishes_nothing() {
        let temporary = TempDir::new().unwrap();
        let root = BackupOperationalRoot::new(temporary.path().join("backups")).unwrap();
        let name = BackupArchiveFileName::new("failed.longhorn-backup").unwrap();
        let archive = minimal_operational_archive();
        let error = publish_corrupt_staging_for_test(
            &root,
            &name,
            &archive,
            BackupPublicationOptions::new(
                DurabilityRequirement::Durable,
                BackupArchiveLimits::default(),
            ),
        )
        .unwrap_err();
        assert_eq!(error.stage, BackupPublicationStage::VerifyTemporary);
        assert!(!error.published);
        assert!(!root.path().join(name.as_str()).exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    fn minimal_operational_archive() -> EncodedBackupArchive {
        let manifest = serde_json::to_vec(&json!({
            "format": "longhorn.config-backup",
            "formatVersion": 1,
            "archiveId": "publication-test",
            "kind": "operational",
            "createdAt": "2026-07-28T12:00:00Z",
            "application": {"id": "com.example.app", "version": "1.0.0"},
            "producer": {"name": "longhorn-config", "version": "0.1.0"},
            "consistencyGroups": [],
            "domains": [],
            "exclusions": []
        }))
        .unwrap();
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(super::super::DEFLATE_LEVEL))
            .last_modified_time(DateTime::default())
            .unix_permissions(0o600)
            .large_file(false);
        writer
            .start_file(super::super::MANIFEST_PATH, options)
            .unwrap();
        writer.write_all(&manifest).unwrap();
        EncodedBackupArchive::new(writer.finish().unwrap().into_inner())
    }
}
