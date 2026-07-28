use std::io::Cursor;

use longhorn_config::{
    BackupArchiveLimits, BackupKind, encode_backup_archive, inspect_backup_archive,
};
use zip::{CompressionMethod, DateTime, ZipArchive};

use super::{APP_ID, fixtures::snapshot};

#[test]
fn canonical_archive_is_deterministic_and_round_trips() {
    let snapshot = snapshot(
        "archive-a",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let first = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
    let second = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
    assert_eq!(first, second);

    let inspection = inspect_backup_archive(first.bytes(), BackupArchiveLimits::default()).unwrap();
    assert_eq!(inspection.manifest(), snapshot.manifest());
    assert_eq!(
        inspection.payloads()[0].bytes(),
        snapshot.payloads()[0].bytes()
    );
    assert_eq!(inspection.archive_sha256(), first.sha256());

    let mut zip = ZipArchive::new(Cursor::new(first.bytes())).unwrap();
    assert_eq!(zip.len(), 2);
    for (index, expected) in [
        "longhorn/manifest.json",
        "longhorn/domains/example.preferences.json",
    ]
    .iter()
    .enumerate()
    {
        let entry = zip.by_index(index).unwrap();
        assert_eq!(entry.name(), *expected);
        assert_eq!(entry.compression(), CompressionMethod::Deflated);
        assert_eq!(entry.last_modified(), Some(DateTime::default()));
        assert_eq!(entry.unix_mode(), Some(0o100600));
        assert!(entry.comment().is_empty());
        assert!(entry.extra_data().is_none_or(<[u8]>::is_empty));
    }
}
