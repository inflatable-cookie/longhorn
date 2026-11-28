use std::{collections::BTreeSet, fs};

use longhorn_config::{
    BackupApplication, BackupArchiveFileName, BackupArchiveLimits, BackupKind,
    BackupOperationalRoot, BackupPublicationDurability, BackupPublicationMode,
    BackupPublicationOptions, BackupRetentionDiagnosticKind, BackupRetentionPolicy,
    DurabilityRequirement, Sha256Digest, apply_backup_retention, list_operational_backups,
    list_operational_backups_with_envelope, plan_backup_retention,
    publish_encrypted_operational_backup, publish_operational_backup,
};
use longhorn_config_age::{
    AgeBackupEnvelopeReader, AgeEnvelopeLimits, AgeIdentity, AgeIdentityRing,
    encrypt_export_to_recipients, encrypt_operational_backup,
};
use tempfile::TempDir;

use super::support::{APP_ID, encoded_archive};

#[test]
fn plaintext_retention_preserves_locked_encrypted_archives() {
    let archive = encoded_archive("locked-retention", BackupKind::Operational);
    let identity = AgeIdentity::generate();
    let encrypted = encrypt_export_to_recipients(
        &archive,
        &[identity.recipient()],
        AgeEnvelopeLimits::default(),
    )
    .unwrap();
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("locked.longhorn-backup.age");
    fs::write(&path, encrypted.bytes()).unwrap();
    let root = BackupOperationalRoot::new(temp.path()).unwrap();

    let listing = list_operational_backups(
        &root,
        &BackupApplication::new(APP_ID, "1.0.0").unwrap(),
        BackupArchiveLimits::default(),
        10,
    );
    assert!(listing.candidates().is_empty());
    assert!(listing.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind == BackupRetentionDiagnosticKind::Locked
            && diagnostic.path.as_deref() == Some(path.as_path())
    }));

    let policy = BackupRetentionPolicy::new(0, None, None, 10).unwrap();
    let plan = plan_backup_retention(
        &listing,
        policy,
        &BTreeSet::new(),
        &Sha256Digest::from_bytes(encrypted.bytes()),
    )
    .unwrap();
    assert!(plan.deletions().is_empty());
    assert!(path.exists());
}

#[test]
fn encrypted_publication_and_retention_require_verified_operational_evidence() {
    let temp = TempDir::new().unwrap();
    let root = BackupOperationalRoot::new(temp.path().join("data/backups")).unwrap();
    let limits = AgeEnvelopeLimits::default();
    let old_key = AgeIdentity::generate();
    let current = AgeIdentityRing::new(AgeIdentity::generate()).with_historical(old_key.clone());
    let reader = AgeBackupEnvelopeReader::new(&current, limits);
    let previous = AgeIdentityRing::new(old_key);
    let foreign = AgeIdentityRing::new(AgeIdentity::generate());

    let archive = |id, provider| {
        encrypt_operational_backup(
            provider,
            &encoded_archive(id, BackupKind::Operational),
            limits,
        )
        .unwrap()
    };
    let publish = |name, ciphertext: &[u8], mode| {
        publish_encrypted_operational_backup(
            &root,
            &BackupArchiveFileName::new(name).unwrap(),
            ciphertext,
            mode,
            &reader,
        )
    };
    let strict = BackupPublicationMode::Strict(DurabilityRequirement::Durable);
    let locked = archive("locked-operational", &foreign);
    assert!(publish("locked.longhorn-backup", locked.bytes(), strict).is_err());
    let export = encrypt_operational_backup(
        &current,
        &encoded_archive("user-export", BackupKind::UserExport),
        limits,
    )
    .unwrap();
    assert!(publish("export.longhorn-backup", export.bytes(), strict).is_err());
    assert!(!root.path().exists());

    let old = archive("old-operational", &previous);
    let new = archive("new-operational", &current);
    let old = publish("old.longhorn-backup", old.bytes(), strict).unwrap();
    let new = publish(
        "new.longhorn-backup",
        new.bytes(),
        BackupPublicationMode::PortableExclusiveCopy,
    )
    .unwrap();
    assert_eq!(
        new.durability,
        BackupPublicationDurability::PortableFileAndDirectorySynced
    );
    assert!(new.path.ends_with("new.longhorn-backup.age"));

    let plain = publish_operational_backup(
        &root,
        &BackupArchiveFileName::new("plain.longhorn-backup").unwrap(),
        &encoded_archive("plain-operational", BackupKind::Operational),
        BackupPublicationOptions::new(DurabilityRequirement::Durable, limits.archive_limits()),
    )
    .unwrap();
    let locked_path = root.path().join("locked.longhorn-backup.age");
    fs::write(&locked_path, locked.bytes()).unwrap();
    let app = BackupApplication::new(APP_ID, "1.0.0").unwrap();
    let listing = list_operational_backups_with_envelope(&root, &app, 10, &reader);
    assert_eq!(listing.candidates().len(), 3);
    assert!(listing.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind == BackupRetentionDiagnosticKind::Locked
            && diagnostic.path.as_deref() == Some(locked_path.as_path())
    }));
    let policy = BackupRetentionPolicy::new(0, None, None, 10).unwrap();
    let pins = BTreeSet::from([plain.archive_sha256]);
    let plan = plan_backup_retention(&listing, policy, &pins, &new.archive_sha256).unwrap();
    assert_eq!(plan.deletions().len(), 1);
    assert_eq!(plan.deletions()[0].archive_sha256, old.archive_sha256);

    let old_bytes = fs::read(&old.path).unwrap();
    fs::write(&old.path, b"replaced ciphertext").unwrap();
    assert!(apply_backup_retention(&plan, limits.archive_limits()).is_err());
    assert!(old.path.exists());
    fs::write(&old.path, old_bytes).unwrap();
    let applied = apply_backup_retention(&plan, limits.archive_limits()).unwrap();
    assert_eq!(applied.deleted, vec![old.path]);
    assert!(new.path.exists());
    assert!(plain.path.exists());
    assert!(locked_path.exists());
}
