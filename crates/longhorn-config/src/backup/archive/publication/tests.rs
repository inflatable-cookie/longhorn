use std::{
    cell::Cell,
    fs,
    io::{Cursor, Write},
};

use serde_json::json;
use tempfile::TempDir;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::{
    BackupArchiveLimits, BackupKind, Durability, DurabilityRequirement,
    backup::archive::{
        EncodedBackupArchive,
        publication_types::{
            BackupArchiveFileName, BackupDestinationKind, BackupOperationalRoot,
            BackupPublicationOptions, BackupPublicationStage, ExportOverwrite,
        },
    },
    inspect_backup_archive,
};

use super::{
    publish::{Publication, publish_operational_backup},
    support::verification_error,
};

#[test]
fn failed_reopen_verification_cleans_partial_and_publishes_nothing() {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path().join("backups");
    let name = "failed.longhorn-backup";
    let archive = minimal_archive("operational");
    let publication = Publication {
        parent: &root,
        name,
        destination: BackupDestinationKind::Operational,
        overwrite: ExportOverwrite::Refuse,
        durability: DurabilityRequirement::Durable,
        max_bytes: BackupArchiveLimits::default().max_archive_bytes(),
    };
    let seen = Cell::new(false);
    let error = publication
        .publish(archive.bytes(), |bytes| {
            let bytes = if seen.replace(true) {
                &bytes[..bytes.len() - 1]
            } else {
                bytes
            };
            inspect_backup_archive(bytes, BackupArchiveLimits::default())
                .map(|inspection| inspection.manifest().kind())
                .map_err(|error| verification_error(root.join(name), false, error))
        })
        .unwrap_err();
    assert_eq!(error.stage, BackupPublicationStage::VerifyTemporary);
    assert!(!error.published);
    assert!(!root.join(name).exists());
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn exclusive_export_preserves_a_destination_created_after_staging() {
    let temporary = TempDir::new().unwrap();
    let name = "raced.longhorn-backup";
    let target = temporary.path().join(name);
    let archive = minimal_archive("user-export");
    let publication = Publication {
        parent: temporary.path(),
        name,
        destination: BackupDestinationKind::UserExport,
        overwrite: ExportOverwrite::Refuse,
        durability: DurabilityRequirement::Durable,
        max_bytes: BackupArchiveLimits::default().max_archive_bytes(),
    };
    let seen = Cell::new(false);
    let error = publication
        .publish(archive.bytes(), |_| {
            if seen.replace(true) {
                fs::write(&target, b"competing file").unwrap();
            }
            Ok(BackupKind::UserExport)
        })
        .unwrap_err();
    assert_eq!(error.stage, BackupPublicationStage::ValidateDestination);
    assert!(!error.published);
    assert_eq!(fs::read(&target).unwrap(), b"competing file");
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[test]
fn first_operational_publication_creates_a_nested_root_and_receipts_durability() {
    let temporary = TempDir::new().unwrap();
    let root = BackupOperationalRoot::new(temporary.path().join("new/data/backups")).unwrap();
    let name = BackupArchiveFileName::new("first.longhorn-backup").unwrap();
    let archive = minimal_archive("operational");
    let receipt = publish_operational_backup(
        &root,
        &name,
        &archive,
        BackupPublicationOptions::new(
            DurabilityRequirement::Durable,
            BackupArchiveLimits::default(),
        ),
    )
    .unwrap();
    assert_eq!(receipt.durability, Durability::FileAndDirectorySynced);
    assert_eq!(fs::read(receipt.path).unwrap(), archive.bytes());
}

#[cfg(unix)]
#[test]
fn durable_publication_refuses_a_new_root_when_its_parent_cannot_be_synced() {
    use std::os::unix::fs::PermissionsExt;

    let temporary = TempDir::new().unwrap();
    let parent = temporary.path().join("write-only");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o300)).unwrap();
    if fs::File::open(&parent).is_ok() {
        // A privileged runner can open this directory for syncing.
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        return;
    }
    let root = BackupOperationalRoot::new(parent.join("backups")).unwrap();
    let name = BackupArchiveFileName::new("first.longhorn-backup").unwrap();
    let archive = minimal_archive("operational");
    let result = publish_operational_backup(
        &root,
        &name,
        &archive,
        BackupPublicationOptions::new(
            DurabilityRequirement::Durable,
            BackupArchiveLimits::default(),
        ),
    );
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();

    let error = result.unwrap_err();
    assert_eq!(error.stage, BackupPublicationStage::CreateRoot);
    assert!(!error.published);
    assert!(!root.path().join(name.as_str()).exists());
    let receipt = publish_operational_backup(
        &root,
        &name,
        &archive,
        BackupPublicationOptions::new(
            DurabilityRequirement::Durable,
            BackupArchiveLimits::default(),
        ),
    )
    .unwrap();
    assert_eq!(receipt.durability, Durability::FileAndDirectorySynced);
}

fn minimal_archive(kind: &str) -> EncodedBackupArchive {
    let manifest = serde_json::to_vec(&json!({
        "format": "longhorn.config-backup",
        "formatVersion": 1,
        "archiveId": "publication-test",
        "kind": kind,
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
