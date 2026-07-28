#[path = "archive/canonical.rs"]
mod canonical;
#[path = "archive/fixtures.rs"]
mod fixtures;
#[path = "archive/publication.rs"]
mod publication;

use fixtures::*;
use std::{collections::BTreeSet, fs, time::Duration};

use longhorn_config::{
    BackupApplication, BackupArchiveError, BackupArchiveLimits, BackupKind, BackupOperationalRoot,
    BackupRetentionDiagnosticKind, BackupRetentionPolicy, MilestoneRetention,
    apply_backup_retention, encode_backup_archive, inspect_backup_archive,
    list_operational_backups, plan_backup_retention,
};
use serde_json::json;
use zip::CompressionMethod;

use crate::common::Fixture;

const APP_ID: &str = "com.example.desktop";

#[test]
fn reader_accepts_stored_and_rejects_other_compression() {
    let snapshot = snapshot(
        "archive-stored",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let stored = archive_with(
        serde_json::to_value(snapshot.manifest()).unwrap(),
        &[(
            "longhorn/domains/example.preferences.json",
            snapshot.payloads()[0].bytes(),
        )],
        CompressionMethod::Stored,
    );
    inspect_backup_archive(&stored, BackupArchiveLimits::default()).unwrap();

    let mut unsupported = stored;
    patch_compression_method(&mut unsupported, 12);
    let error = inspect_backup_archive(&unsupported, BackupArchiveLimits::default()).unwrap_err();
    assert!(
        matches!(error, BackupArchiveError::UnsupportedCompression { .. }),
        "{error:?}"
    );
}

#[test]
fn strict_manifest_inventory_and_checksums_fail_safe() {
    let snapshot = snapshot(
        "archive-strict",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let manifest = serde_json::to_value(snapshot.manifest()).unwrap();
    let path = "longhorn/domains/example.preferences.json";
    let payload = snapshot.payloads()[0].bytes();

    let mut future = manifest.clone();
    future["formatVersion"] = json!(2);
    let future = archive_with(future, &[(path, payload)], CompressionMethod::Stored);
    assert!(matches!(
        inspect_backup_archive(&future, BackupArchiveLimits::default()),
        Err(BackupArchiveError::UnsupportedFormatVersion { .. })
    ));

    let mut unknown = manifest.clone();
    unknown["surprise"] = json!(true);
    let unknown = archive_with(unknown, &[(path, payload)], CompressionMethod::Stored);
    assert!(matches!(
        inspect_backup_archive(&unknown, BackupArchiveLimits::default()),
        Err(BackupArchiveError::ManifestJson { .. })
    ));

    let undeclared = archive_with(
        manifest.clone(),
        &[(path, payload), ("longhorn/unexpected.bin", b"x")],
        CompressionMethod::Stored,
    );
    assert!(matches!(
        inspect_backup_archive(&undeclared, BackupArchiveLimits::default()),
        Err(BackupArchiveError::UndeclaredEntry { .. })
    ));

    let mut damaged = payload.to_vec();
    damaged[0] ^= 1;
    let damaged = archive_with(manifest, &[(path, &damaged)], CompressionMethod::Stored);
    assert!(matches!(
        inspect_backup_archive(&damaged, BackupArchiveLimits::default()),
        Err(BackupArchiveError::ChecksumMismatch { .. })
    ));
}

#[test]
fn unsafe_duplicate_and_bomb_shaped_entries_are_rejected_in_memory() {
    let snapshot = snapshot(
        "archive-unsafe",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let manifest = serde_json::to_value(snapshot.manifest()).unwrap();
    let payload = snapshot.payloads()[0].bytes();
    let traversal = archive_with(
        manifest.clone(),
        &[("../escape", b"x")],
        CompressionMethod::Stored,
    );
    assert!(matches!(
        inspect_backup_archive(&traversal, BackupArchiveLimits::default()),
        Err(BackupArchiveError::InvalidEntryName { .. })
    ));

    let mut duplicate = archive_with(
        manifest.clone(),
        &[("longhorn/a.json", payload), ("longhorn/b.json", payload)],
        CompressionMethod::Stored,
    );
    replace_all_same_length(&mut duplicate, b"longhorn/b.json", b"longhorn/a.json");
    let duplicate_error =
        inspect_backup_archive(&duplicate, BackupArchiveLimits::default()).unwrap_err();
    assert!(
        matches!(duplicate_error, BackupArchiveError::DuplicateEntry { .. }),
        "{duplicate_error:?}"
    );

    let compressed = archive_with(
        manifest,
        &[("longhorn/domains/example.preferences.json", payload)],
        CompressionMethod::Deflated,
    );
    let tight =
        BackupArchiveLimits::new(1024 * 1024, 10, 512, 1024 * 1024, 1024 * 1024, 1).unwrap();
    assert!(matches!(
        inspect_backup_archive(&compressed, tight),
        Err(BackupArchiveError::CompressionRatio { .. })
    ));
}

#[test]
fn absolute_nul_directory_symlink_and_finite_limits_are_rejected() {
    let snapshot = snapshot(
        "archive-shapes",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let manifest = serde_json::to_value(snapshot.manifest()).unwrap();
    for path in ["/absolute.json", "longhorn/../escape.json"] {
        let archive = archive_with(manifest.clone(), &[(path, b"x")], CompressionMethod::Stored);
        assert!(matches!(
            inspect_backup_archive(&archive, BackupArchiveLimits::default()),
            Err(BackupArchiveError::InvalidEntryName { .. })
        ));
    }

    let mut nul = archive_with(
        manifest.clone(),
        &[("longhorn/a.json", b"x")],
        CompressionMethod::Stored,
    );
    replace_all_same_length(&mut nul, b"longhorn/a.json", b"longhorn/\0.json");
    assert!(matches!(
        inspect_backup_archive(&nul, BackupArchiveLimits::default()),
        Err(BackupArchiveError::InvalidEntryName { .. })
    ));

    for archive in [
        archive_with_directory(manifest.clone()),
        archive_with_symlink(manifest.clone()),
        archive_with_device(manifest.clone()),
    ] {
        assert!(matches!(
            inspect_backup_archive(&archive, BackupArchiveLimits::default()),
            Err(BackupArchiveError::InvalidEntryName { .. }
                | BackupArchiveError::NonRegularEntry { .. }
                | BackupArchiveError::NonCanonicalMetadata { .. })
        ));
    }

    let encoded = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
    let entry_count = BackupArchiveLimits::new(
        encoded.bytes().len() + 1,
        1,
        512,
        1024 * 1024,
        1024 * 1024,
        200,
    )
    .unwrap();
    assert!(matches!(
        inspect_backup_archive(encoded.bytes(), entry_count),
        Err(BackupArchiveError::TooManyEntries { .. })
    ));
    let per_entry =
        BackupArchiveLimits::new(encoded.bytes().len() + 1, 10, 512, 32, 1024, 200).unwrap();
    assert!(matches!(
        inspect_backup_archive(encoded.bytes(), per_entry),
        Err(BackupArchiveError::EntryTooLarge { .. })
    ));
    let manifest_bytes = serde_json::to_vec(snapshot.manifest()).unwrap();
    let payload_bytes = snapshot.payloads()[0].bytes();
    let total_limit = manifest_bytes.len().max(payload_bytes.len()) + 1;
    assert!(manifest_bytes.len() + payload_bytes.len() > total_limit);
    let aggregate = BackupArchiveLimits::new(
        encoded.bytes().len() + 1,
        10,
        512,
        total_limit,
        total_limit,
        200,
    )
    .unwrap();
    assert!(matches!(
        inspect_backup_archive(encoded.bytes(), aggregate),
        Err(BackupArchiveError::TotalTooLarge { .. })
    ));
}

#[test]
fn listing_preserves_unproven_files_and_retention_rechecks_bytes() {
    let fixture = Fixture::new();
    let root_path = fixture.temp.path().join("backups");
    fs::create_dir(&root_path).unwrap();
    let root = BackupOperationalRoot::new(&root_path).unwrap();
    let app = BackupApplication::new(APP_ID, "9.9.9").unwrap();

    let archives = [
        ("archive-a", "2000-07-28T12:00:00Z", APP_ID),
        ("archive-b", "2000-07-27T12:00:00Z", APP_ID),
        ("archive-c", "2000-07-20T12:00:00Z", APP_ID),
        ("archive-foreign", "2000-07-29T12:00:00Z", "other.app"),
    ]
    .map(|(id, time, app_id)| {
        let snapshot = snapshot(id, time, app_id, BackupKind::Operational);
        encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap()
    });
    for (index, archive) in archives.iter().enumerate() {
        fs::write(
            root_path.join(format!("{index}.longhorn-backup")),
            archive.bytes(),
        )
        .unwrap();
    }
    fs::write(root_path.join("damaged.longhorn-backup"), b"not a zip").unwrap();
    let valid_manifest = serde_json::to_value(
        snapshot(
            "future",
            "2026-07-28T12:00:00Z",
            APP_ID,
            BackupKind::Operational,
        )
        .manifest(),
    )
    .unwrap();
    let mut future_manifest = valid_manifest;
    future_manifest["formatVersion"] = json!(2);
    fs::write(
        root_path.join("future.longhorn-backup"),
        archive_with(future_manifest, &[], CompressionMethod::Stored),
    )
    .unwrap();
    fs::write(root_path.join("locked.longhorn-backup.age"), b"age bytes").unwrap();
    fs::write(root_path.join("notes.txt"), b"keep me").unwrap();

    let listing = list_operational_backups(&root, &app, BackupArchiveLimits::default(), 100);
    assert!(listing.is_complete());
    assert_eq!(listing.candidates().len(), 3);
    let kinds = listing
        .diagnostics()
        .iter()
        .map(|item| item.kind)
        .collect::<BTreeSet<_>>();
    assert!(kinds.contains(&BackupRetentionDiagnosticKind::ForeignApplication));
    assert!(kinds.contains(&BackupRetentionDiagnosticKind::Corrupt));
    assert!(kinds.contains(&BackupRetentionDiagnosticKind::UnknownFormat));
    assert!(kinds.contains(&BackupRetentionDiagnosticKind::Locked));
    assert!(kinds.contains(&BackupRetentionDiagnosticKind::Unmanaged));

    let policy = BackupRetentionPolicy::new(
        1,
        Some(Duration::from_secs(2 * 24 * 60 * 60)),
        Some(MilestoneRetention::new(Duration::from_secs(7 * 24 * 60 * 60), 1).unwrap()),
        100,
    )
    .unwrap();
    let mut pins = BTreeSet::new();
    pins.insert(archives[1].sha256().clone());
    let plan = plan_backup_retention(&listing, policy, &pins, archives[0].sha256()).unwrap();
    assert_eq!(plan.deletions().len(), 1);

    fs::write(&plan.deletions()[0].path, b"changed after planning").unwrap();
    assert!(apply_backup_retention(&plan, BackupArchiveLimits::default()).is_err());
    fs::write(&plan.deletions()[0].path, archives[2].bytes()).unwrap();
    let receipt = apply_backup_retention(&plan, BackupArchiveLimits::default()).unwrap();
    assert_eq!(receipt.deleted, [plan.deletions()[0].path.clone()]);
    assert!(root_path.join("damaged.longhorn-backup").exists());
    assert!(root_path.join("locked.longhorn-backup.age").exists());
    assert!(root_path.join("notes.txt").exists());
}

#[test]
fn equal_times_and_clock_regression_are_deterministic() {
    let fixture = Fixture::new();
    let root_path = fixture.temp.path().join("backups");
    fs::create_dir(&root_path).unwrap();
    let root = BackupOperationalRoot::new(&root_path).unwrap();
    let app = BackupApplication::new(APP_ID, "1").unwrap();
    let older_new = snapshot(
        "archive-new",
        "2026-07-27T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let same_a = snapshot(
        "archive-a",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let same_z = snapshot(
        "archive-z",
        "2026-07-28T12:00:00Z",
        APP_ID,
        BackupKind::Operational,
    );
    let archives = [&older_new, &same_a, &same_z]
        .map(|snapshot| encode_backup_archive(snapshot, BackupArchiveLimits::default()).unwrap());
    for (index, archive) in archives.iter().enumerate() {
        fs::write(
            root_path.join(format!("{index}.longhorn-backup")),
            archive.bytes(),
        )
        .unwrap();
    }
    let listing = list_operational_backups(&root, &app, BackupArchiveLimits::default(), 10);
    assert_eq!(listing.candidates()[0].archive_id(), "archive-z");
    assert_eq!(listing.candidates()[1].archive_id(), "archive-a");
    let policy = BackupRetentionPolicy::new(1, None, None, 10).unwrap();
    let plan =
        plan_backup_retention(&listing, policy, &BTreeSet::new(), archives[0].sha256()).unwrap();
    assert!(
        plan.diagnostics()
            .iter()
            .any(|item| { item.kind == BackupRetentionDiagnosticKind::ClockRegression })
    );
    assert!(plan.retained().contains_key(archives[0].sha256()));
}

#[test]
fn milestone_buckets_keep_one_deterministic_representative() {
    let fixture = Fixture::new();
    let root_path = fixture.temp.path().join("backups");
    fs::create_dir(&root_path).unwrap();
    let root = BackupOperationalRoot::new(&root_path).unwrap();
    let app = BackupApplication::new(APP_ID, "1").unwrap();
    let archives = [
        ("archive-0", "2000-07-28T12:00:00Z"),
        ("archive-1", "2000-07-21T12:00:00Z"),
        ("archive-2", "2000-07-14T12:00:00Z"),
        ("archive-3", "2000-07-07T12:00:00Z"),
    ]
    .map(|(id, time)| {
        encode_backup_archive(
            &snapshot(id, time, APP_ID, BackupKind::Operational),
            BackupArchiveLimits::default(),
        )
        .unwrap()
    });
    for (index, archive) in archives.iter().enumerate() {
        fs::write(
            root_path.join(format!("{index}.longhorn-backup")),
            archive.bytes(),
        )
        .unwrap();
    }
    let listing = list_operational_backups(&root, &app, BackupArchiveLimits::default(), 10);
    let policy = BackupRetentionPolicy::new(
        0,
        None,
        Some(MilestoneRetention::new(Duration::from_secs(7 * 24 * 60 * 60), 3).unwrap()),
        10,
    )
    .unwrap();
    let plan =
        plan_backup_retention(&listing, policy, &BTreeSet::new(), archives[0].sha256()).unwrap();
    assert_eq!(plan.deletions().len(), 1);
    assert_eq!(plan.deletions()[0].archive_sha256, *archives[3].sha256());
    for archive in &archives[..3] {
        assert!(plan.retained().contains_key(archive.sha256()));
    }
}

#[test]
fn incomplete_listing_never_produces_a_prune_plan() {
    let fixture = Fixture::new();
    let root_path = fixture.temp.path().join("backups");
    fs::create_dir(&root_path).unwrap();
    let root = BackupOperationalRoot::new(&root_path).unwrap();
    let archive = encode_backup_archive(
        &snapshot(
            "archive-a",
            "2026-07-28T12:00:00Z",
            APP_ID,
            BackupKind::Operational,
        ),
        BackupArchiveLimits::default(),
    )
    .unwrap();
    fs::write(root_path.join("a.longhorn-backup"), archive.bytes()).unwrap();
    fs::write(root_path.join("unmanaged"), b"preserve").unwrap();
    let app = BackupApplication::new(APP_ID, "1").unwrap();
    let listing = list_operational_backups(&root, &app, BackupArchiveLimits::default(), 1);
    assert!(!listing.is_complete());
    let policy = BackupRetentionPolicy::new(0, None, None, 1).unwrap();
    assert!(plan_backup_retention(&listing, policy, &BTreeSet::new(), archive.sha256()).is_err());
}
