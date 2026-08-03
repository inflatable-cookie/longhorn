use super::*;

use longhorn_config::{
    BackupArchiveFileName, BackupCaptureOptions, BackupKind, BackupLimits, BackupMetadata,
    BackupOperationalRoot, CoordinationFailureKind, DurabilityRequirement, LoadOutcome,
    MigrationRewriteOptions, MutationError, MutationOptions, RestoreExecutionOptions,
    RestoreOperationState, RestoreSafetyBackupOptions,
};

fn safety(
    fixture: &Fixture,
    kind: BackupKind,
    archive_id: &str,
    file_name: &str,
) -> RestoreSafetyBackupOptions {
    let (application, producer) = identities();
    RestoreSafetyBackupOptions::new(
        BackupMetadata::new(
            archive_id,
            kind,
            "2026-07-28T13:00:00Z",
            application,
            producer,
        )
        .unwrap(),
        BackupOperationalRoot::new(fixture.temp.path().join("backups")).unwrap(),
        BackupArchiveFileName::new(file_name).unwrap(),
        BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        BackupArchiveLimits::default(),
    )
}

#[test]
fn execution_publishes_verified_target_and_retains_exact_safety_backup() {
    let donor = Fixture::new();
    let donor_domain = config_domain();
    let archive_bytes = document(
        "example.preferences",
        3,
        json!({"name": "archive", "enabled": true}),
    );
    donor.write(&donor_domain, &archive_bytes);
    let mut donor_store = donor.store();
    donor_store.register(&donor_domain).unwrap();
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&donor_domain).unwrap();
    let archive = archive(&donor_store, &donor_catalog);

    let target = Fixture::new();
    let target_domain = config_domain();
    let old = document(
        "example.preferences",
        3,
        json!({"name": "old", "enabled": false}),
    );
    target.write(&target_domain, &old);
    let mut store = target.store();
    store.register(&target_domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&target_domain).unwrap();
    let (application, producer) = identities();
    let inspection = store.inspect_restore(&catalog, &archive, &application, &producer);
    let plan = store
        .plan_restore(
            &inspection,
            &choices([(
                target_domain.descriptor().id().clone(),
                RestoreConflictChoice::UseArchive,
            )]),
        )
        .unwrap();
    let staging = store
        .prepare_restore(
            &catalog,
            &archive,
            &inspection,
            &plan,
            RestorePrepareOptions::new(Duration::from_secs(2)),
        )
        .unwrap();
    let receipt = store
        .execute_restore(
            &catalog,
            staging,
            RestoreExecutionOptions::new(
                Duration::from_secs(2),
                safety(
                    &target,
                    BackupKind::PreRestore,
                    "pre-restore-1",
                    "pre-restore-1.longhorn-backup",
                ),
            ),
        )
        .unwrap();

    assert_eq!(
        fs::read(target.path_for(&target_domain)).unwrap(),
        archive_bytes
    );
    assert_eq!(
        receipt.restored(),
        &[target_domain.descriptor().id().clone()]
    );
    assert_eq!(
        receipt.safety_backup().durability,
        longhorn_config::Durability::FileAndDirectorySynced
    );
    let safety_bytes = fs::read(&receipt.safety_backup().path).unwrap();
    let safety_archive =
        inspect_backup_archive(&safety_bytes, BackupArchiveLimits::default()).unwrap();
    assert_eq!(safety_archive.payloads()[0].bytes(), old);
    assert_eq!(
        store.restore_operation_state(),
        RestoreOperationState::Inactive
    );
    assert_eq!(store.restore_safety_pin().unwrap(), None);
}

#[test]
fn destructive_migration_requires_and_records_pre_migration_backup() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let old = document("example.preferences", 1, json!({"label": "legacy"}));
    fixture.write(&domain, &old);
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();

    let receipt = store
        .rewrite_migrated_domain(
            &catalog,
            &domain,
            MigrationRewriteOptions::new(
                Duration::from_secs(2),
                safety(
                    &fixture,
                    BackupKind::PreMigration,
                    "pre-migration-1",
                    "pre-migration-1.longhorn-backup",
                ),
            ),
        )
        .unwrap();

    assert_eq!(receipt.from().get(), 1);
    assert_eq!(receipt.to().get(), 3);
    let rewritten: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.path_for(&domain)).unwrap()).unwrap();
    assert_eq!(rewritten["schemaVersion"], 3);
    assert_eq!(rewritten["value"]["name"], "legacy");
    assert_eq!(rewritten["value"]["enabled"], true);
    let safety_bytes = fs::read(&receipt.safety_backup().path).unwrap();
    let safety_archive =
        inspect_backup_archive(&safety_bytes, BackupArchiveLimits::default()).unwrap();
    assert_eq!(safety_archive.payloads()[0].bytes(), old);
}

#[test]
fn coordinated_load_set_holds_one_generation_guard_across_member_reads() {
    let fixture = Fixture::new();
    let alpha = PreferencesDomain::new(
        "a.preferences",
        StorageClass::UserConfig,
        Some("coordinated/a.json"),
        3,
    );
    let beta = PreferencesDomain::new(
        "b.preferences",
        StorageClass::MachineState,
        Some("coordinated/b.json"),
        3,
    );
    for domain in [&alpha, &beta] {
        fixture.write(
            domain,
            &document(
                domain.descriptor().id().as_str(),
                3,
                json!({"name": domain.descriptor().id().as_str(), "enabled": true}),
            ),
        );
    }
    let mut reader = fixture.store();
    reader.register(&alpha).unwrap();
    reader.register(&beta).unwrap();
    let mut competing_writer = fixture.store();
    competing_writer.register(&alpha).unwrap();

    reader
        .with_coordinated_load_set(Duration::from_secs(1), |set| {
            assert!(matches!(set.load(&alpha).unwrap(), LoadOutcome::Ready(_)));
            let blocked = competing_writer
                .mutate(
                    &alpha,
                    MutationOptions::new(Duration::ZERO, DurabilityRequirement::Durable),
                    |value| {
                        value.name = "mixed".into();
                        Ok(())
                    },
                )
                .unwrap_err();
            assert!(matches!(
                blocked,
                MutationError::Coordination(failure)
                    if failure.kind == CoordinationFailureKind::Busy
            ));
            assert!(matches!(set.load(&beta).unwrap(), LoadOutcome::Ready(_)));
        })
        .unwrap();
}

#[test]
fn terminal_journal_self_heals_on_bare_load_while_active_phases_still_block() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let bytes = document(
        "example.preferences",
        3,
        json!({"name": "kept", "enabled": true}),
    );
    fixture.write(&domain, &bytes);
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let journal_dir = fixture.temp.path().join("data/.longhorn/restore");
    fs::create_dir_all(&journal_dir).unwrap();
    let digest = "a".repeat(64);
    let journal = |phase: &str| {
        format!(
            concat!(
                "{{\"version\":1,\"operationId\":\"crash-sim\",",
                "\"planDigest\":\"{digest}\",\"safetyPath\":\"{path}\",",
                "\"safetySha256\":\"{digest}\",\"phase\":\"{phase}\",\"entries\":[]}}"
            ),
            digest = "DIGEST",
            path = "PATH",
            phase = "PHASE"
        )
        .replace("DIGEST", &digest)
        .replace(
            "PATH",
            fixture.temp.path().join("safety.zip").to_str().unwrap(),
        )
        .replace("PHASE", phase)
    };

    // A crash mid-apply keeps blocking bare loads.
    fs::write(journal_dir.join("journal.json"), journal("applying")).unwrap();
    assert_eq!(
        store.restore_operation_state(),
        RestoreOperationState::Active
    );
    assert!(matches!(
        store.load(&domain).unwrap(),
        LoadOutcome::Unavailable(_)
    ));
    assert!(journal_dir.join("journal.json").exists());

    // A crash after completion self-heals and returns data.
    fs::write(journal_dir.join("journal.json"), journal("succeeded")).unwrap();
    let loaded = store.load(&domain).unwrap();
    assert!(matches!(loaded, LoadOutcome::Ready(_)));
    assert!(!journal_dir.join("journal.json").exists());
    assert_eq!(
        store.restore_operation_state(),
        RestoreOperationState::Inactive
    );

    // Same for a rolled-back terminal phase.
    fs::write(journal_dir.join("journal.json"), journal("rolled-back")).unwrap();
    assert!(matches!(
        store.load(&domain).unwrap(),
        LoadOutcome::Ready(_)
    ));
    assert!(!journal_dir.join("journal.json").exists());
}
