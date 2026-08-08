use std::{
    fs,
    io::{Cursor, Write},
};

use serde_json::json;
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, CompressionMethod, DateTime, ZipWriter};

use crate::{
    backup::archive::{
        publication_types::{
            BackupArchiveFileName, BackupOperationalRoot, BackupPublicationOptions,
            BackupPublicationStage,
        },
        EncodedBackupArchive,
    },
    BackupArchiveLimits, DurabilityRequirement,
};

use super::publish::publish_corrupt_staging_for_test;

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
