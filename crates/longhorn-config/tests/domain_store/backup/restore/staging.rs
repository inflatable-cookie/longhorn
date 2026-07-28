use super::*;

#[test]
fn preparation_rejects_stale_current_state_under_coordination() {
    let donor = Fixture::new();
    let domain = config_domain();
    donor.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": "archive", "enabled": true}),
        ),
    );
    let mut donor_store = donor.store();
    donor_store.register(&domain).unwrap();
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&domain).unwrap();
    let archive = archive(&donor_store, &donor_catalog);

    let target = Fixture::new();
    let mut target_store = target.store();
    target_store.register(&domain).unwrap();
    let mut target_catalog = BackupCatalog::new();
    target_catalog.include(&domain).unwrap();
    let (application, producer) = identities();
    let inspection =
        target_store.inspect_restore(&target_catalog, &archive, &application, &producer);
    let plan = target_store
        .plan_restore(
            &inspection,
            &choices([(
                domain.descriptor().id().clone(),
                RestoreConflictChoice::UseArchive,
            )]),
        )
        .unwrap();
    let current = document(
        "example.preferences",
        3,
        json!({"name": "changed-after-preview", "enabled": false}),
    );
    target.write(&domain, &current);

    assert!(matches!(
        target_store.prepare_restore(
            &target_catalog,
            &archive,
            &inspection,
            &plan,
            RestorePrepareOptions::new(Duration::from_secs(2)),
        ),
        Err(RestorePrepareError::StaleCurrent { .. })
    ));
    assert_eq!(fs::read(target.path_for(&domain)).unwrap(), current);
}

#[test]
fn staging_is_complete_current_schema_all_or_nothing_and_never_publishes() {
    let donor = Fixture::new();
    let alpha = PreferencesDomain::new(
        "a.preferences",
        StorageClass::UserConfig,
        Some("restore/a.json"),
        3,
    );
    let beta = PreferencesDomain::new(
        "b.preferences",
        StorageClass::UserConfig,
        Some("restore/b.json"),
        3,
    );
    for domain in [&alpha, &beta] {
        donor.write(
            domain,
            &document(
                domain.descriptor().id().as_str(),
                1,
                json!({"label": domain.descriptor().id().as_str()}),
            ),
        );
    }
    let mut donor_store = donor.store();
    donor_store.register(&alpha).unwrap();
    donor_store.register(&beta).unwrap();
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&alpha).unwrap();
    donor_catalog.include(&beta).unwrap();
    let archive = archive(&donor_store, &donor_catalog);

    let target = Fixture::new();
    let complete_alpha = PreferencesDomain::new(
        "a.preferences",
        StorageClass::UserConfig,
        Some("restore/a.json"),
        3,
    );
    let complete_beta = PreferencesDomain::new(
        "b.preferences",
        StorageClass::UserConfig,
        Some("restore/b.json"),
        3,
    );
    let mut target_store = target.store();
    target_store.register(&complete_alpha).unwrap();
    target_store.register(&complete_beta).unwrap();
    let mut complete_catalog = BackupCatalog::new();
    complete_catalog.include(&complete_alpha).unwrap();
    complete_catalog.include(&complete_beta).unwrap();
    let (application, producer) = identities();
    let inspection =
        target_store.inspect_restore(&complete_catalog, &archive, &application, &producer);
    let plan = target_store
        .plan_restore(
            &inspection,
            &choices([
                (
                    complete_alpha.descriptor().id().clone(),
                    RestoreConflictChoice::UseArchive,
                ),
                (
                    complete_beta.descriptor().id().clone(),
                    RestoreConflictChoice::UseArchive,
                ),
            ]),
        )
        .unwrap();

    let failing_beta = PreferencesDomain::new(
        "b.preferences",
        StorageClass::UserConfig,
        Some("restore/b.json"),
        3,
    )
    .with_behavior(MigrationBehavior::MissingSecond);
    let mut failing_catalog = BackupCatalog::new();
    failing_catalog.include(&complete_alpha).unwrap();
    failing_catalog.include(&failing_beta).unwrap();
    assert!(matches!(
        target_store.prepare_restore(
            &failing_catalog,
            &archive,
            &inspection,
            &plan,
            RestorePrepareOptions::new(Duration::from_secs(2)),
        ),
        Err(RestorePrepareError::DomainStagingFailed { .. })
    ));
    assert!(!target.path_for(&complete_alpha).exists());
    assert!(!target.path_for(&complete_beta).exists());

    let staging = target_store
        .prepare_restore(
            &complete_catalog,
            &archive,
            &inspection,
            &plan,
            RestorePrepareOptions::new(Duration::from_secs(2)),
        )
        .unwrap();
    assert_eq!(staging.receipt().selected(), 2);
    assert_eq!(staging.receipt().documents(), 2);
    assert_eq!(staging.receipt().deletions(), 0);
    assert!(staging.receipt().total_document_bytes() > 0);
    assert_eq!(staging.plan_digest(), plan.digest());
    assert!(!target.path_for(&complete_alpha).exists());
    assert!(!target.path_for(&complete_beta).exists());
}

#[test]
fn unchanged_targets_stage_without_requesting_publication() {
    let donor = Fixture::new();
    let domain = config_domain();
    let bytes = document(
        "example.preferences",
        3,
        json!({"name": "same", "enabled": true}),
    );
    donor.write(&domain, &bytes);
    let mut donor_store = donor.store();
    donor_store.register(&domain).unwrap();
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&domain).unwrap();
    let archive = archive(&donor_store, &donor_catalog);

    let target = Fixture::new();
    target.write(&domain, &bytes);
    let mut target_store = target.store();
    target_store.register(&domain).unwrap();
    let mut target_catalog = BackupCatalog::new();
    target_catalog.include(&domain).unwrap();
    let (application, producer) = identities();
    let inspection =
        target_store.inspect_restore(&target_catalog, &archive, &application, &producer);
    let plan = target_store
        .plan_restore(
            &inspection,
            &choices([(
                domain.descriptor().id().clone(),
                RestoreConflictChoice::UseArchive,
            )]),
        )
        .unwrap();
    assert_eq!(plan.entries()[0].action(), Some(RestoreAction::Unchanged));
    let staging = target_store
        .prepare_restore(
            &target_catalog,
            &archive,
            &inspection,
            &plan,
            RestorePrepareOptions::new(Duration::from_secs(2)),
        )
        .unwrap();
    assert_eq!(staging.receipt().unchanged(), 1);
    assert_eq!(fs::read(target.path_for(&domain)).unwrap(), bytes);
}
