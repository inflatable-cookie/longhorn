use std::fs;

use longhorn_config::{
    BackupArchiveFileName, BackupArchiveLimits, BackupExportTarget, BackupKind,
    BackupOperationalRoot, ExportOverwrite, encode_backup_archive, export_backup,
    publish_operational_backup,
};

use crate::common::Fixture;

use super::{
    APP_ID,
    fixtures::{publication_options, snapshot},
};

#[test]
fn publication_separates_operational_and_export_authority() {
    let fixture = Fixture::new();
    let root = fixture.temp.path().join("backups");
    let operational = snapshot(
        "archive-operational",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let archive = encode_backup_archive(&operational, BackupArchiveLimits::default()).unwrap();
    let root = BackupOperationalRoot::new(&root).unwrap();
    let name = BackupArchiveFileName::new("operational.longhorn-backup").unwrap();
    let options = publication_options();
    let receipt = publish_operational_backup(&root, &name, &archive, options).unwrap();
    assert_eq!(receipt.archive_sha256, *archive.sha256());
    assert!(receipt.path.exists());

    let export_dir = fixture.temp.path().join("exports");
    fs::create_dir(&export_dir).unwrap();
    let export_snapshot = snapshot(
        "archive-export",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::UserExport,
    );
    let export_archive =
        encode_backup_archive(&export_snapshot, BackupArchiveLimits::default()).unwrap();
    assert!(
        publish_operational_backup(
            &root,
            &BackupArchiveFileName::new("wrong-kind.longhorn-backup").unwrap(),
            &export_archive,
            options,
        )
        .is_err()
    );
    let export = BackupExportTarget::new(
        &export_dir,
        BackupArchiveFileName::new("selected.longhorn-backup").unwrap(),
    )
    .unwrap();
    assert!(export_backup(&export, &archive, ExportOverwrite::Refuse, options).is_err());
    export_backup(&export, &export_archive, ExportOverwrite::Refuse, options).unwrap();
    assert!(export_backup(&export, &export_archive, ExportOverwrite::Refuse, options).is_err());
    let replaced =
        export_backup(&export, &export_archive, ExportOverwrite::Replace, options).unwrap();
    assert!(replaced.replaced_existing);
    assert_eq!(
        fs::read(export_dir.join("selected.longhorn-backup")).unwrap(),
        export_archive.bytes()
    );
}
