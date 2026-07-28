use std::{collections::BTreeSet, fs};

use longhorn_config::{
    BackupApplication, BackupArchiveLimits, BackupKind, BackupOperationalRoot,
    BackupRetentionDiagnosticKind, BackupRetentionPolicy, Sha256Digest, list_operational_backups,
    plan_backup_retention,
};
use longhorn_config_age::{AgeEnvelopeLimits, AgeIdentity, encrypt_export_to_recipients};
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
