use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{BackupKind, Sha256Digest, atomic_file::PrivateOpenOptions};

use super::super::{
    BackupEnvelopeError, BackupEnvelopeReader, EncodedBackupArchive,
    envelope::ENCRYPTED_EXTENSION,
    inspect_backup_archive,
    publication_types::{
        BackupArchiveFileName, BackupDestinationKind, BackupExportTarget, BackupOperationalRoot,
        BackupPublicationDurability, BackupPublicationError, BackupPublicationMode,
        BackupPublicationOptions, BackupPublicationReceipt, BackupPublicationStage,
        ExportOverwrite,
    },
};
use super::commit::{CommitError, commit_name};
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
    mode: BackupPublicationMode,
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
        mode,
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
        mode: options.mode,
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
    pub(super) mode: BackupPublicationMode,
    pub(super) max_bytes: usize,
}

impl Publication<'_> {
    pub(super) fn publish(
        &self,
        bytes: &[u8],
        verify: impl Fn(&[u8]) -> Result<BackupKind, BackupPublicationError>,
    ) -> Result<BackupPublicationReceipt, BackupPublicationError> {
        self.publish_with_commit(bytes, verify, commit_name)
    }

    pub(super) fn publish_with_commit(
        &self,
        bytes: &[u8],
        verify: impl Fn(&[u8]) -> Result<BackupKind, BackupPublicationError>,
        commit: impl FnOnce(&Path, &Path, ExportOverwrite) -> Result<(), CommitError>,
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

        if self.mode == BackupPublicationMode::PortableExclusiveCopy
            && self.overwrite == ExportOverwrite::Replace
        {
            return Err(publication_error(
                BackupPublicationStage::ValidateDestination,
                target,
                false,
                "portable exclusive copy requires an unused destination",
            ));
        }

        let root_durable = self.prepare_parent()?;
        let existed = target.symlink_metadata().is_ok();
        if existed && self.overwrite == ExportOverwrite::Refuse {
            return Err(destination_exists(target));
        }
        let expected = Sha256Digest::from_bytes(bytes);
        match self.mode {
            BackupPublicationMode::Strict(_) => {
                self.publish_staged(bytes, &target, &expected, verify, commit)?;
            }
            BackupPublicationMode::PortableExclusiveCopy => {
                let file = create_destination(&target)?;
                write_destination(file, &target, bytes)?;
                verify_portable_destination(&target, self.max_bytes, &expected)?;
            }
        }
        let durability = self.sync_publication(&target, root_durable)?;
        Ok(BackupPublicationReceipt {
            path: target,
            destination: self.destination,
            archive_sha256: expected,
            durability,
            replaced_existing: existed,
        })
    }

    fn publish_staged(
        &self,
        bytes: &[u8],
        target: &Path,
        expected: &Sha256Digest,
        verify: impl Fn(&[u8]) -> Result<BackupKind, BackupPublicationError>,
        commit: impl FnOnce(&Path, &Path, ExportOverwrite) -> Result<(), CommitError>,
    ) -> Result<(), BackupPublicationError> {
        let mut temporary = create_temporary(self.parent, self.name, target)?;
        if let Err(error) = temporary.write(bytes) {
            return Err(cleanup(
                publication_error(
                    BackupPublicationStage::WriteTemporary,
                    target.to_path_buf(),
                    false,
                    error.to_string(),
                ),
                temporary,
            ));
        }
        temporary.close();

        let checked = read_archive_bytes(temporary.path(), self.max_bytes)
            .map_err(|error| verification_error(target.to_path_buf(), false, error))
            .and_then(|reopened| {
                verify(&reopened)?;
                if &Sha256Digest::from_bytes(&reopened) != expected {
                    return Err(publication_error(
                        BackupPublicationStage::VerifyTemporary,
                        target.to_path_buf(),
                        false,
                        "staged archive hash changed",
                    ));
                }
                Ok(())
            });
        if let Err(error) = checked {
            return Err(cleanup(error, temporary));
        }

        commit(temporary.path(), target, self.overwrite)
            .map_err(|error| cleanup(commit_failure(target, self.overwrite, error), temporary))
    }

    fn sync_publication(
        &self,
        target: &Path,
        root_durable: bool,
    ) -> Result<BackupPublicationDurability, BackupPublicationError> {
        let synced = crate::dir_sync::sync_dir_path(self.parent);
        match (synced, root_durable) {
            (Ok(()), true) => Ok(match self.mode {
                BackupPublicationMode::Strict(_) => {
                    BackupPublicationDurability::FileAndDirectorySynced
                }
                BackupPublicationMode::PortableExclusiveCopy => {
                    BackupPublicationDurability::PortableFileAndDirectorySynced
                }
            }),
            _ if !self.mode.requires_directory_sync() => {
                Ok(BackupPublicationDurability::FileSynced)
            }
            (result, _) => Err(publication_error(
                BackupPublicationStage::SyncDirectory,
                target.to_path_buf(),
                true,
                result.err().map_or_else(
                    || "operational root could not be synced".into(),
                    |error| error.to_string(),
                ),
            )),
        }
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
                Err(_) if !self.mode.requires_directory_sync() => {
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

fn destination_exists(target: PathBuf) -> BackupPublicationError {
    publication_error(
        BackupPublicationStage::ValidateDestination,
        target,
        false,
        "destination already exists and overwrite was not authorized",
    )
}

fn commit_failure(
    target: &Path,
    overwrite: ExportOverwrite,
    error: CommitError,
) -> BackupPublicationError {
    let (stage, detail) = match error {
        CommitError::AtomicNoReplaceUnavailable(error) => (
            BackupPublicationStage::AtomicNoReplaceUnsupported,
            format!(
                "atomic no-replace rename unavailable and hard-link publication failed: {error}"
            ),
        ),
        CommitError::Io(error)
            if error.kind() == io::ErrorKind::AlreadyExists
                && overwrite == ExportOverwrite::Refuse =>
        {
            return destination_exists(target.to_path_buf());
        }
        CommitError::Io(error) => (BackupPublicationStage::Rename, error.to_string()),
    };
    publication_error(stage, target.to_path_buf(), false, detail)
}

pub(super) fn create_destination(target: &Path) -> Result<File, BackupPublicationError> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true).set_private_mode();
    options.open(target).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            destination_exists(target.to_path_buf())
        } else {
            publication_error(
                BackupPublicationStage::CreateDestination,
                target.to_path_buf(),
                false,
                error.to_string(),
            )
        }
    })
}

pub(super) fn write_destination(
    mut file: File,
    target: &Path,
    bytes: &[u8],
) -> Result<(), BackupPublicationError> {
    // The name can be unlinked and reused while we hold the descriptor.
    // Keep using the descriptor and leave any failed copy for the caller to inspect.
    file.write_all(bytes).map_err(|error| {
        publication_error(
            BackupPublicationStage::WriteDestination,
            target.to_path_buf(),
            false,
            format!("portable copy may be incomplete: {error}"),
        )
    })?;
    file.sync_all().map_err(|error| {
        publication_error(
            BackupPublicationStage::SyncDestination,
            target.to_path_buf(),
            false,
            format!("portable copy could not be synced: {error}"),
        )
    })
}

pub(super) fn verify_portable_destination(
    target: &Path,
    limit: usize,
    expected: &Sha256Digest,
) -> Result<(), BackupPublicationError> {
    let failed = |detail: String| {
        publication_error(
            BackupPublicationStage::VerifyDestination,
            target.to_path_buf(),
            false,
            detail,
        )
    };
    let metadata = target
        .symlink_metadata()
        .map_err(|error| failed(error.to_string()))?;
    if !metadata.is_file() {
        return Err(failed(
            "portable destination is no longer a regular file".into(),
        ));
    }
    let bytes = read_archive_bytes(target, limit).map_err(|error| failed(error.to_string()))?;
    if &Sha256Digest::from_bytes(&bytes) != expected {
        return Err(failed(
            "portable destination no longer matches the verified archive".into(),
        ));
    }
    Ok(())
}
