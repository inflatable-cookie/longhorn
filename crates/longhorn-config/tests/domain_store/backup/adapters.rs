use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use fs4::FileExt;
use longhorn_config::{
    BackupAdapter, BackupAdapterCapabilities, BackupAdapterCapture, BackupAdapterCaptureMode,
    BackupAdapterCaptureRequest, BackupAdapterConsistencyGroup, BackupAdapterError,
    BackupAdapterGroupedApplyRequest, BackupAdapterGroupedRestore,
    BackupAdapterGroupedStageRequest, BackupAdapterGroupedVerifyRequest, BackupAdapterId,
    BackupAdapterInspectRequest, BackupAdapterPayload, BackupAdapterRelativePath,
    BackupAdapterRestoreOutcome, BackupAdapterRestoreParticipation, BackupAdapterRestorePreview,
    BackupAdapterRestoreRequest, BackupAdapterRestoreStage, BackupAdapterStateEvidence,
    BackupApplication, BackupArchiveFileName, BackupArchiveLimits, BackupCatalog,
    BackupConsistencyMode, BackupKind, BackupLimits, BackupMetadata, BackupOperationalRoot,
    BackupProducer, BackupScope, BackupSourceState, ConfigDomain, CoordinationAuthority,
    DomainDescriptor, DomainFilePath, DomainIssue, MigrationStep, RestoreAdapterError,
    RestoreAdapterRequirement, RestoreChoices, RestoreConflictChoice, RestoreDomainCompatibility,
    RestoreExecutionOptions, RestorePrepareOptions, RestoreSafetyBackupOptions, Sha256Digest,
    StorageClass, encode_backup_archive, inspect_backup_archive,
};
use longhorn_core::{DomainId, SchemaVersion};
use rusqlite::{Connection, MAIN_DB, OpenFlags, params};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

use crate::common::{Fixture, PreferencesDomain, document};
use support::{
    OpaqueDomain, SqliteAdapter, StaticAdapter, adapter_failure, database_value, safety_options,
    seed_wal_database, semantic_digest, sqlite_sidecar,
};

#[test]
fn sqlite_external_snapshot_captures_wal_state_and_restores_only_with_explicit_authority() {
    let source_database = tempdir().unwrap();
    let source_path = source_database.path().join("library.sqlite3");
    let source = seed_wal_database(&source_path, "captured-from-wal");
    let wal_path = sqlite_sidecar(&source_path, "-wal");
    let wal_before = fs::read(&wal_path).expect("live WAL exists");

    let donor = Fixture::new();
    let settings = PreferencesDomain::new(
        "soundcheck.settings-window",
        StorageClass::UserConfig,
        Some("soundcheck/settings.json"),
        3,
    );
    donor.write(
        &settings,
        &document(
            "soundcheck.settings-window",
            3,
            json!({"name": "soundcheck", "enabled": true}),
        ),
    );
    let database_domain = OpaqueDomain::new(
        "soundcheck.library",
        StorageClass::UserConfig,
        "soundcheck/library-authority.json",
        &["database"],
    );
    let source_adapter = SqliteAdapter::new(
        source_path.clone(),
        BackupAdapterRestoreParticipation::Separate,
        Some(donor.coordination.clone()),
    );
    let mut donor_store = donor.store();
    donor_store.register(&settings).unwrap();
    donor_store.register(&database_domain).unwrap();
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&settings).unwrap();
    donor_catalog
        .custom(&database_domain, &source_adapter)
        .unwrap();

    let snapshot = super::capture(&donor_store, &donor_catalog, &BackupScope::AllRegistered)
        .expect("mixed capture");
    assert!(
        source_adapter
            .captured_outside_longhorn_guard
            .load(Ordering::SeqCst)
    );
    assert_eq!(fs::read(&wal_path).unwrap(), wal_before);
    assert_eq!(snapshot.receipt().custom_domains(), 1);
    assert_eq!(snapshot.receipt().external_consistency_groups(), 1);
    assert_eq!(snapshot.adapter_receipts().len(), 1);
    assert_eq!(
        snapshot.adapter_receipts()[0].consistency_mode(),
        BackupConsistencyMode::ExternalSnapshot
    );
    let groups = snapshot.manifest().consistency_groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].id(), "longhorn-config-store");
    assert_eq!(groups[1].id(), "soundcheck-sqlite");
    assert_eq!(groups[1].authority(), "sqlite-online-backup-api");
    let database_manifest = snapshot
        .manifest()
        .domains()
        .iter()
        .find(|domain| domain.domain() == database_domain.descriptor().id())
        .unwrap();
    assert_eq!(database_manifest.payloads().len(), 1);
    assert_eq!(
        database_manifest.payloads()[0].path().as_str(),
        "longhorn/adapters/soundcheck.library/library.sqlite3"
    );

    let encoded =
        encode_backup_archive(&snapshot, BackupArchiveLimits::default()).expect("encode mixed");
    let archive = inspect_backup_archive(encoded.bytes(), BackupArchiveLimits::default())
        .expect("inspect mixed");
    let target_database = tempdir().unwrap();
    let target_path = target_database.path().join("library.sqlite3");
    let target_connection = seed_wal_database(&target_path, "keep-before-confirmation");
    drop(target_connection);
    let target = Fixture::new();
    let target_settings = PreferencesDomain::new(
        "soundcheck.settings-window",
        StorageClass::UserConfig,
        Some("soundcheck/settings.json"),
        3,
    );
    let target_database_domain = OpaqueDomain::new(
        "soundcheck.library",
        StorageClass::UserConfig,
        "soundcheck/library-authority.json",
        &["database"],
    );
    let target_adapter = SqliteAdapter::new(
        target_path.clone(),
        BackupAdapterRestoreParticipation::Separate,
        None,
    );
    let mut target_store = target.store();
    target_store.register(&target_settings).unwrap();
    target_store.register(&target_database_domain).unwrap();
    let mut target_catalog = BackupCatalog::new();
    target_catalog.include(&target_settings).unwrap();
    target_catalog
        .custom(&target_database_domain, &target_adapter)
        .unwrap();
    let application = BackupApplication::new("com.example.desktop", "9").unwrap();
    let producer = BackupProducer::new("longhorn-config", "9").unwrap();
    let inspection =
        target_store.inspect_restore(&target_catalog, &archive, &application, &producer);
    assert_eq!(inspection.receipt().restorable(), 1);
    assert_eq!(inspection.receipt().adapter_restorable(), 1);
    let custom = inspection
        .domains()
        .iter()
        .find(|domain| domain.domain() == target_database_domain.descriptor().id())
        .unwrap();
    assert!(matches!(
        custom.compatibility(),
        RestoreDomainCompatibility::CustomAdapterReady {
            participation: BackupAdapterRestoreParticipation::Separate,
            ..
        }
    ));
    let confirmation = inspection
        .adapter_confirmation(target_database_domain.descriptor().id())
        .unwrap()
        .clone();

    assert!(matches!(
        target_store.execute_adapter_restore(
            &target_catalog,
            &archive,
            &inspection,
            target_database_domain.descriptor().id(),
            &confirmation,
            RestoreAdapterRequirement::FailureAtomic,
        ),
        Err(RestoreAdapterError::FailureAtomicRequired { .. })
    ));
    assert_eq!(database_value(&target_path), "keep-before-confirmation");

    let receipt = target_store
        .execute_adapter_restore(
            &target_catalog,
            &archive,
            &inspection,
            target_database_domain.descriptor().id(),
            &confirmation,
            RestoreAdapterRequirement::AllowSeparate,
        )
        .expect("explicit separate restore");
    assert_eq!(
        receipt.participation(),
        &BackupAdapterRestoreParticipation::Separate
    );
    let BackupAdapterRestoreOutcome::Verified { evidence } = receipt.outcome() else {
        panic!("separate SQLite restore did not verify its target");
    };
    assert_eq!(evidence, &semantic_digest(&target_path).unwrap());
    assert_eq!(database_value(&target_path), "captured-from-wal");
    drop(source);
}

#[test]
fn loophole_soundcheck_and_split_shell_fixtures_round_trip_without_library_schemas() {
    let source = Fixture::new();
    let loophole = OpaqueDomain::new(
        "loophole.machine-window-layout",
        StorageClass::MachineState,
        "loophole/machine-window-layout.json",
        &["machineId", "windowInstances", "surfaceIds"],
    );
    let soundcheck = OpaqueDomain::new(
        "soundcheck.settings-window",
        StorageClass::UserConfig,
        "soundcheck/settings-window.json",
        &["agentReviewModel", "availabilityTargetId", "mainWindow"],
    );
    let split-shell = OpaqueDomain::new(
        "split-shell.workspace-presentation",
        StorageClass::WorkspaceLocal,
        "split-shell/workspace-presentation.json",
        &[
            "workspaceRoot",
            "navigationRatio",
            "expandedNodeIds",
            "selectedNodeId",
        ],
    );
    let fixtures = [
        (
            &loophole,
            json!({
                "machineId": "machine-a",
                "windowInstances": [{"windowId": "main", "displayFallbackIds": ["display-b"]}],
                "surfaceIds": ["surface-main", "surface-mixer"]
            }),
        ),
        (
            &soundcheck,
            json!({
                "agentReviewModel": "gpt-5.4-mini",
                "availabilityTargetId": "portable",
                "mainWindow": {"x": 30, "y": 40, "width": 1280, "height": 800}
            }),
        ),
        (
            &split-shell,
            json!({
                "workspaceRoot": "/fixture/private-consumer",
                "navigationRatio": 0.24,
                "expandedNodeIds": ["pathway:acca"],
                "selectedNodeId": "module:e3"
            }),
        ),
    ];
    let mut expected = Vec::new();
    for (domain, value) in &fixtures {
        let bytes = document(domain.descriptor().id().as_str(), 1, value.clone());
        let path = domain.path(&source);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, &bytes).unwrap();
        expected.push((domain.descriptor().id().clone(), bytes));
    }
    let mut source_store = source.store();
    let mut source_catalog = BackupCatalog::new();
    for (domain, _) in &fixtures {
        source_store.register(*domain).unwrap();
        source_catalog.include(*domain).unwrap();
    }
    let snapshot =
        super::capture(&source_store, &source_catalog, &BackupScope::AllRegistered).unwrap();
    let encoded = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
    let archive = inspect_backup_archive(encoded.bytes(), BackupArchiveLimits::default()).unwrap();

    let target = Fixture::new();
    let target_loophole = OpaqueDomain::new(
        "loophole.machine-window-layout",
        StorageClass::MachineState,
        "loophole/machine-window-layout.json",
        &["machineId", "windowInstances", "surfaceIds"],
    );
    let target_soundcheck = OpaqueDomain::new(
        "soundcheck.settings-window",
        StorageClass::UserConfig,
        "soundcheck/settings-window.json",
        &["agentReviewModel", "availabilityTargetId", "mainWindow"],
    );
    let target_split_shell = OpaqueDomain::new(
        "split-shell.workspace-presentation",
        StorageClass::WorkspaceLocal,
        "split-shell/workspace-presentation.json",
        &[
            "workspaceRoot",
            "navigationRatio",
            "expandedNodeIds",
            "selectedNodeId",
        ],
    );
    let target_domains = [&target_loophole, &target_soundcheck, &target_split_shell];
    let mut target_store = target.store();
    let mut target_catalog = BackupCatalog::new();
    for domain in target_domains {
        target_store.register(domain).unwrap();
        target_catalog.include(domain).unwrap();
    }
    let application = BackupApplication::new("com.example.desktop", "9").unwrap();
    let producer = BackupProducer::new("longhorn-config", "9").unwrap();
    let inspection =
        target_store.inspect_restore(&target_catalog, &archive, &application, &producer);
    assert_eq!(inspection.receipt().restorable(), 3);
    let mut choices = RestoreChoices::new();
    for domain in target_domains {
        choices
            .choose(
                domain.descriptor().id().clone(),
                RestoreConflictChoice::UseArchive,
            )
            .unwrap();
    }
    let plan = target_store.plan_restore(&inspection, &choices).unwrap();
    let staging = target_store
        .prepare_restore(
            &target_catalog,
            &archive,
            &inspection,
            &plan,
            RestorePrepareOptions::new(Duration::from_secs(2)),
        )
        .unwrap();
    target_store
        .execute_restore(
            &target_catalog,
            staging,
            RestoreExecutionOptions::new(Duration::from_secs(2), safety_options(&target)),
        )
        .unwrap();
    for domain in target_domains {
        let expected = expected
            .iter()
            .find(|(id, _)| id == domain.descriptor().id())
            .unwrap();
        assert_eq!(fs::read(domain.path(&target)).unwrap(), expected.1);
    }
}

mod adapter_payloads;
#[path = "adapters/grouped.rs"]
mod grouped;
mod support;
