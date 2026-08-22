use std::{fs, path::Path};

use crate::{Durability, DurabilityRequirement};

use super::super::{
    EncodedBackupArchive, inspect_backup_archive,
    publication_types::{
        BackupArchiveFileName, BackupDestinationKind, BackupExportTarget, BackupOperationalRoot,
        BackupPublicationError, BackupPublicationOptions, BackupPublicationReceipt,
        BackupPublicationStage, ExportOverwrite,
    },
};
use super::support::{
    cleanup, create_temporary, publication_error, read_bounded_archive, verification_error,
};

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

pub(super) fn publish(
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
    // Validate the parent by metadata, not by opening it: Windows cannot
    // open a directory with plain CreateFileW at all (ERROR_ACCESS_DENIED),
    // so holding a File on it was Unix-only reasoning (Soundcheck Windows
    // finding, 2026-08-22). The durability barrier at SyncDirectory goes
    // through `crate::dir_sync` instead.
    match fs::metadata(parent_path) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            return Err(publication_error(
                BackupPublicationStage::OpenParent,
                target.clone(),
                false,
                "publication parent exists but is not a directory",
            ));
        }
        Err(error) => {
            return Err(publication_error(
                BackupPublicationStage::OpenParent,
                target.clone(),
                false,
                error.to_string(),
            ));
        }
    }
    let mut temporary = create_temporary(parent_path, file_name, &target)?;
    let staged_bytes = if corrupt_staging {
        &archive.bytes()[..archive.bytes().len().saturating_sub(1)]
    } else {
        archive.bytes()
    };
    if let Err(error) = temporary.write(staged_bytes) {
        return Err(cleanup(
            publication_error(
                BackupPublicationStage::WriteTemporary,
                target,
                false,
                error.to_string(),
            ),
            temporary,
        ));
    }
    temporary.close();

    let verified = match read_bounded_archive(temporary.path(), options.archive_limits)
        .and_then(|bytes| inspect_backup_archive(&bytes, options.archive_limits))
    {
        Ok(inspection) => inspection,
        Err(error) => {
            return Err(cleanup(verification_error(target, false, error), temporary));
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
            temporary,
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
            temporary,
        ));
    }
    if let Err(error) = fs::rename(temporary.path(), &target) {
        return Err(cleanup(
            publication_error(
                BackupPublicationStage::Rename,
                target,
                false,
                error.to_string(),
            ),
            temporary,
        ));
    }
    drop(temporary);

    let sync = crate::dir_sync::sync_dir_path(parent_path).map_err(|error| {
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
