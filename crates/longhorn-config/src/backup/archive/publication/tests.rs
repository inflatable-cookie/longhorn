use std::{
    cell::Cell,
    fs,
    io::{self, Cursor, Write},
};

use serde_json::json;
use tempfile::TempDir;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::{
    BackupArchiveLimits, BackupKind, BackupPublicationDurability, BackupPublicationMode,
    DurabilityRequirement, Sha256Digest,
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
    commit::commit_no_replace,
    publish::{
        Publication, create_destination, publish_operational_backup, verify_portable_destination,
        write_destination,
    },
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
        mode: BackupPublicationMode::Strict(DurabilityRequirement::Durable),
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
        mode: BackupPublicationMode::Strict(DurabilityRequirement::Durable),
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
    assert_eq!(
        receipt.durability,
        BackupPublicationDurability::FileAndDirectorySynced
    );
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
    assert_eq!(
        receipt.durability,
        BackupPublicationDurability::FileAndDirectorySynced
    );
}

#[test]
fn unavailable_atomic_primitives_require_explicit_portable_publication() {
    let temporary = TempDir::new().unwrap();
    let archive = minimal_archive("user-export");
    let path = temporary.path().join("portable.longhorn-backup");
    let mut publisher = Publication {
        parent: temporary.path(),
        name: "portable.longhorn-backup",
        destination: BackupDestinationKind::UserExport,
        overwrite: ExportOverwrite::Refuse,
        mode: BackupPublicationMode::Strict(DurabilityRequirement::Durable),
        max_bytes: BackupArchiveLimits::default().max_archive_bytes(),
    };
    let verify = |bytes: &[u8]| {
        inspect_backup_archive(bytes, BackupArchiveLimits::default())
            .map(|archive| archive.manifest().kind())
            .map_err(|error| verification_error(path.clone(), false, error))
    };
    let error = publisher
        .publish_with_commit(archive.bytes(), verify, |staging, target, _| {
            commit_no_replace(
                staging,
                target,
                |_, _| Err(io::ErrorKind::Unsupported.into()),
                |_, _| Err(io::ErrorKind::Unsupported.into()),
            )
        })
        .unwrap_err();
    assert_eq!(
        error.stage,
        BackupPublicationStage::AtomicNoReplaceUnsupported
    );
    assert!(!error.published);
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 0);

    publisher.mode = BackupPublicationMode::PortableExclusiveCopy;
    let receipt = publisher
        .publish_with_commit(archive.bytes(), verify, |_, _, _| {
            panic!("portable mode used rename")
        })
        .unwrap();
    assert_eq!(
        receipt.durability,
        BackupPublicationDurability::PortableFileAndDirectorySynced
    );
    assert!(!receipt.replaced_existing);
    assert_eq!(fs::read(&path).unwrap(), archive.bytes());

    let error = publisher.publish(archive.bytes(), verify).unwrap_err();
    assert_eq!(error.stage, BackupPublicationStage::ValidateDestination);
    assert_eq!(fs::read(&path).unwrap(), archive.bytes());
}

#[test]
fn unsupported_rename_uses_an_atomic_link_only_to_an_unused_name() {
    let temporary = TempDir::new().unwrap();
    let staging = temporary.path().join("staged");
    let target = temporary.path().join("destination");
    fs::write(&staging, b"staged bytes").unwrap();
    fs::write(&target, b"competing file").unwrap();
    let publish = || {
        commit_no_replace(
            &staging,
            &target,
            |_, _| Err(io::ErrorKind::Unsupported.into()),
            |from, to| fs::hard_link(from, to),
        )
    };
    assert!(
        matches!(publish(), Err(super::commit::CommitError::Io(error))
        if error.kind() == io::ErrorKind::AlreadyExists)
    );
    assert_eq!(fs::read(&target).unwrap(), b"competing file");
    assert!(staging.is_file());

    fs::remove_file(&target).unwrap();
    publish().unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"staged bytes");
    assert!(!staging.exists());
}

#[cfg(unix)]
#[test]
fn portable_copy_cannot_overwrite_a_file_that_replaces_its_open_destination() {
    let temporary = TempDir::new().unwrap();
    let path = temporary.path().join("destination");
    let original = temporary.path().join("displaced");
    let file = create_destination(&path).unwrap();
    fs::rename(&path, &original).unwrap();
    fs::write(&path, b"competing file").unwrap();

    write_destination(file, &path, b"archive data").unwrap();
    let error =
        verify_portable_destination(&path, 1024, &Sha256Digest::from_bytes(b"archive data"))
            .unwrap_err();
    assert_eq!(error.stage, BackupPublicationStage::VerifyDestination);
    assert_eq!(fs::read(path).unwrap(), b"competing file");
    assert_eq!(fs::read(original).unwrap(), b"archive data");
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "run with TMPDIR on a disposable exFAT volume"]
fn exfat_requires_portable_publication_for_export_and_operational_backup() {
    use crate::{
        BackupExportTarget, BackupPublicationError, BackupPublicationReceipt, export_backup,
    };

    let temp = TempDir::new().unwrap();
    let strict = BackupPublicationOptions::new(
        DurabilityRequirement::Durable,
        BackupArchiveLimits::default(),
    );
    let portable = strict.with_mode(BackupPublicationMode::PortableExclusiveCopy);
    let export = BackupExportTarget::new(
        temp.path(),
        BackupArchiveFileName::new("export.longhorn-backup").unwrap(),
    )
    .unwrap();
    let export_archive = minimal_archive("user-export");
    let publish_export =
        |options| export_backup(&export, &export_archive, ExportOverwrite::Refuse, options);
    let root = BackupOperationalRoot::new(temp.path().join("backups")).unwrap();
    let name = BackupArchiveFileName::new("operational.longhorn-backup").unwrap();
    let operational_archive = minimal_archive("operational");
    let publish_operational =
        |options| publish_operational_backup(&root, &name, &operational_archive, options);

    type Publish<'a> = dyn Fn(BackupPublicationOptions) -> Result<BackupPublicationReceipt, BackupPublicationError>
        + 'a;
    let cases: [(&Publish<'_>, &[u8]); 2] = [
        (&publish_export, export_archive.bytes()),
        (&publish_operational, operational_archive.bytes()),
    ];
    for (publish, bytes) in cases {
        let error = publish(strict).unwrap_err();
        assert_eq!(
            error.stage,
            BackupPublicationStage::AtomicNoReplaceUnsupported
        );
        assert!(!error.published);
        assert!(!error.path.exists());

        let receipt = publish(portable).unwrap();
        assert_eq!(
            receipt.durability,
            BackupPublicationDurability::PortableFileAndDirectorySynced
        );
        assert!(!receipt.replaced_existing);
        assert_eq!(fs::read(&receipt.path).unwrap(), bytes);

        fs::write(&receipt.path, b"independent file").unwrap();
        let error = publish(portable).unwrap_err();
        assert_eq!(error.stage, BackupPublicationStage::ValidateDestination);
        assert_eq!(fs::read(receipt.path).unwrap(), b"independent file");
    }
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
