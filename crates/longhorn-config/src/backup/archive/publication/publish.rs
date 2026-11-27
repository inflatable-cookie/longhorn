use std::{fs, io, path::Path};

use crate::{BackupKind, Durability, DurabilityRequirement, Sha256Digest};

use super::super::{
    BackupEnvelopeError, BackupEnvelopeReader, EncodedBackupArchive,
    envelope::ENCRYPTED_EXTENSION,
    inspect_backup_archive,
    publication_types::{
        BackupArchiveFileName, BackupDestinationKind, BackupExportTarget, BackupOperationalRoot,
        BackupPublicationError, BackupPublicationOptions, BackupPublicationReceipt,
        BackupPublicationStage, ExportOverwrite,
    },
};
use super::support::{
    cleanup, create_temporary, publication_error, read_archive_bytes, verification_error,
};

/// Publishes one verified archive below the operational backup root.
pub fn publish_operational_backup(
    root: &BackupOperationalRoot,
    file_name: &BackupArchiveFileName,
    archive: &EncodedBackupArchive,
    options: BackupPublicationOptions,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    publish_plaintext(
        root.path(),
        file_name.as_str(),
        archive,
        BackupDestinationKind::Operational,
        ExportOverwrite::Refuse,
        options,
    )
}

/// Authenticates and publishes an encrypted operational backup, appending `.age` to its name.
///
/// The reader must be able to authenticate the archive using injected identity
/// authority both before staging and when the staged file is reopened.
pub fn publish_encrypted_operational_backup(
    root: &BackupOperationalRoot,
    file_name: &BackupArchiveFileName,
    ciphertext: &[u8],
    durability: DurabilityRequirement,
    reader: &dyn BackupEnvelopeReader,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    let name = format!("{}{ENCRYPTED_EXTENSION}", file_name.as_str());
    let target = root.path().join(&name);
    if name.len() > 255 {
        return Err(publication_error(
            BackupPublicationStage::ValidateDestination,
            target,
            false,
            "encrypted archive file name is too long",
        ));
    }
    let verify = |bytes: &[u8]| {
        reader
            .inspect(bytes)
            .map(|inspection| inspection.manifest().kind())
            .map_err(|error| match error {
                BackupEnvelopeError::InnerArchiveRejected(error) => {
                    verification_error(target.clone(), false, error)
                }
                error => publication_error(
                    BackupPublicationStage::VerifyTemporary,
                    target.clone(),
                    false,
                    error.to_string(),
                ),
            })
    };
    Publication {
        parent: root.path(),
        name: &name,
        destination: BackupDestinationKind::Operational,
        overwrite: ExportOverwrite::Refuse,
        durability,
        max_bytes: reader.max_ciphertext_bytes(),
    }
    .publish(ciphertext, verify)
}

/// Publishes one verified archive to an explicit user-selected destination.
pub fn export_backup(
    target: &BackupExportTarget,
    archive: &EncodedBackupArchive,
    overwrite: ExportOverwrite,
    options: BackupPublicationOptions,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    publish_plaintext(
        target.parent(),
        target.file_name().as_str(),
        archive,
        BackupDestinationKind::UserExport,
        overwrite,
        options,
    )
}

fn publish_plaintext(
    parent: &Path,
    name: &str,
    archive: &EncodedBackupArchive,
    destination: BackupDestinationKind,
    overwrite: ExportOverwrite,
    options: BackupPublicationOptions,
) -> Result<BackupPublicationReceipt, BackupPublicationError> {
    Publication {
        parent,
        name,
        destination,
        overwrite,
        durability: options.durability,
        max_bytes: options.archive_limits.max_archive_bytes(),
    }
    .publish(archive.bytes(), |bytes| {
        inspect_backup_archive(bytes, options.archive_limits)
            .map(|inspection| inspection.manifest().kind())
            .map_err(|error| verification_error(parent.join(name), false, error))
    })
}

pub(super) struct Publication<'a> {
    pub(super) parent: &'a Path,
    pub(super) name: &'a str,
    pub(super) destination: BackupDestinationKind,
    pub(super) overwrite: ExportOverwrite,
    pub(super) durability: DurabilityRequirement,
    pub(super) max_bytes: usize,
}

impl Publication<'_> {
    pub(super) fn publish(
        &self,
        bytes: &[u8],
        verify: impl Fn(&[u8]) -> Result<BackupKind, BackupPublicationError>,
    ) -> Result<BackupPublicationReceipt, BackupPublicationError> {
        let target = self.parent.join(self.name);
        if bytes.len() > self.max_bytes {
            return Err(verification_error(
                target,
                false,
                super::super::BackupArchiveError::ArchiveTooLarge {
                    limit: self.max_bytes,
                    observed: bytes.len(),
                },
            ));
        }
        let kind = verify(bytes)?;
        if (self.destination == BackupDestinationKind::UserExport)
            != (kind == BackupKind::UserExport)
        {
            return Err(publication_error(
                BackupPublicationStage::ValidateDestination,
                target,
                false,
                "archive kind does not match destination authority",
            ));
        }

        let root_durable = self.prepare_parent()?;
        let existed = target.symlink_metadata().is_ok();
        if existed && self.overwrite == ExportOverwrite::Refuse {
            return Err(destination_exists(target));
        }
        let mut temporary = create_temporary(self.parent, self.name, &target)?;
        if let Err(error) = temporary.write(bytes) {
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

        let expected = Sha256Digest::from_bytes(bytes);
        let checked = read_archive_bytes(temporary.path(), self.max_bytes)
            .map_err(|error| verification_error(target.clone(), false, error))
            .and_then(|reopened| {
                verify(&reopened)?;
                if Sha256Digest::from_bytes(&reopened) != expected {
                    return Err(publication_error(
                        BackupPublicationStage::VerifyTemporary,
                        target.clone(),
                        false,
                        "staged archive hash changed",
                    ));
                }
                Ok(())
            });
        if let Err(error) = checked {
            return Err(cleanup(error, temporary));
        }

        if let Err(error) = commit_name(temporary.path(), &target, self.overwrite) {
            let failure = if error.kind() == io::ErrorKind::AlreadyExists
                && self.overwrite == ExportOverwrite::Refuse
            {
                destination_exists(target)
            } else {
                publication_error(
                    BackupPublicationStage::Rename,
                    target,
                    false,
                    error.to_string(),
                )
            };
            return Err(cleanup(failure, temporary));
        }

        let synced = crate::dir_sync::sync_dir_path(self.parent);
        let durability = match (synced, root_durable, self.durability) {
            (Ok(()), true, _) => Durability::FileAndDirectorySynced,
            (_, _, DurabilityRequirement::Atomic) => Durability::FileSynced,
            (Err(error), _, DurabilityRequirement::Durable) => {
                return Err(publication_error(
                    BackupPublicationStage::SyncDirectory,
                    target,
                    true,
                    error.to_string(),
                ));
            }
            (Ok(()), false, DurabilityRequirement::Durable) => {
                return Err(publication_error(
                    BackupPublicationStage::SyncDirectory,
                    target,
                    true,
                    "operational root could not be synced",
                ));
            }
        };
        Ok(BackupPublicationReceipt {
            path: target,
            destination: self.destination,
            archive_sha256: expected,
            durability,
            replaced_existing: existed,
        })
    }

    fn prepare_parent(&self) -> Result<bool, BackupPublicationError> {
        if self.destination == BackupDestinationKind::Operational {
            return match crate::dir_sync::create_dir_all_synced(self.parent) {
                Ok(()) if self.parent.is_dir() => Ok(true),
                Ok(()) => Err(publication_error(
                    BackupPublicationStage::OpenParent,
                    self.parent.join(self.name),
                    false,
                    "publication parent exists but is not a directory",
                )),
                Err(_) if self.durability == DurabilityRequirement::Atomic => {
                    fs::create_dir_all(self.parent).map_err(|error| {
                        publication_error(
                            BackupPublicationStage::CreateRoot,
                            self.parent.to_path_buf(),
                            false,
                            error.to_string(),
                        )
                    })?;
                    Ok(false)
                }
                Err(error) => Err(publication_error(
                    BackupPublicationStage::CreateRoot,
                    self.parent.to_path_buf(),
                    false,
                    error.to_string(),
                )),
            };
        }
        if !self.parent.is_dir() {
            return Err(publication_error(
                BackupPublicationStage::ValidateDestination,
                self.parent.join(self.name),
                false,
                "export parent does not exist or is not a directory",
            ));
        }
        Ok(true)
    }
}

fn destination_exists(target: std::path::PathBuf) -> BackupPublicationError {
    publication_error(
        BackupPublicationStage::ValidateDestination,
        target,
        false,
        "destination already exists and overwrite was not authorized",
    )
}

fn commit_name(staging: &Path, target: &Path, overwrite: ExportOverwrite) -> io::Result<()> {
    match overwrite {
        ExportOverwrite::Refuse => rename_no_replace(staging, target),
        ExportOverwrite::Replace => fs::rename(staging, target),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_no_replace(staging: &Path, target: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};

    renameat_with(CWD, staging, CWD, target, RenameFlags::NOREPLACE).map_err(Into::into)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_no_replace(staging: &Path, target: &Path) -> io::Result<()> {
    fs::hard_link(staging, target)?;
    if let Err(error) = fs::remove_file(staging) {
        longhorn_core::report_best_effort_failure("config.backup.partial-cleanup", error);
    }
    Ok(())
}
