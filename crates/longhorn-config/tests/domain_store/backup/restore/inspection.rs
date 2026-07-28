use super::*;

#[test]
fn inspection_reports_identity_unknown_absent_preserved_excluded_and_unavailable() {
    let donor = Fixture::new();
    let present = config_domain();
    let absent = PreferencesDomain::new(
        "example.absent",
        StorageClass::MachineState,
        Some("restore/absent.json"),
        3,
    );
    let preserved = PreferencesDomain::new(
        "example.preserved",
        StorageClass::UserConfig,
        Some("restore/preserved.json"),
        3,
    );
    let excluded = PreferencesDomain::new(
        "example.excluded",
        StorageClass::MachineState,
        Some("restore/excluded.json"),
        3,
    );
    let policy = PreferencesDomain::new(
        "example.policy",
        StorageClass::Policy,
        Some("restore/policy.json"),
        3,
    );
    donor.write(
        &present,
        &document(
            "example.preferences",
            3,
            json!({"name": "present", "enabled": true}),
        ),
    );
    donor.write(&preserved, b"{broken");
    donor.write(
        &policy,
        &document(
            "example.policy",
            3,
            json!({"name": "policy", "enabled": true}),
        ),
    );
    let mut donor_store = donor.store();
    for domain in [&present, &absent, &preserved, &excluded, &policy] {
        donor_store.register(domain).unwrap();
    }
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&present).unwrap();
    donor_catalog.include(&absent).unwrap();
    donor_catalog.include(&preserved).unwrap();
    donor_catalog
        .exclude(
            &excluded,
            BackupExclusionReason::new("not-portable").unwrap(),
        )
        .unwrap();
    donor_catalog.include(&policy).unwrap();
    let archive = archive(&donor_store, &donor_catalog);

    let target = Fixture::new();
    let mut target_store = target.store();
    for domain in [&absent, &preserved, &excluded, &policy] {
        target_store.register(domain).unwrap();
    }
    let mut target_catalog = BackupCatalog::new();
    target_catalog.include(&absent).unwrap();
    target_catalog.include(&preserved).unwrap();
    target_catalog.include(&policy).unwrap();
    let wrong_application = BackupApplication::new("com.other.desktop", "1").unwrap();
    let wrong_producer = BackupProducer::new("other-producer", "1").unwrap();

    let inspection = target_store.inspect_restore(
        &target_catalog,
        &archive,
        &wrong_application,
        &wrong_producer,
    );

    assert!(!inspection.identity().is_compatible());
    assert_eq!(inspection.domains().len(), 4);
    assert_eq!(inspection.receipt().manifest_domains(), 4);
    assert_eq!(inspection.receipt().exclusions(), 1);
    assert_eq!(inspection.receipt().restorable(), 1);
    assert_eq!(inspection.receipt().blocked(), 3);
    let report = |id: &str| {
        inspection
            .domains()
            .iter()
            .find(|domain| domain.domain().as_str() == id)
            .unwrap()
    };
    assert!(matches!(
        report("example.absent").compatibility(),
        RestoreDomainCompatibility::Ready
    ));
    assert!(matches!(
        report("example.policy").compatibility(),
        RestoreDomainCompatibility::TargetUnavailable { .. }
    ));
    assert!(matches!(
        report("example.preserved").compatibility(),
        RestoreDomainCompatibility::SourcePreserved {
            issue: BackupSourceIssue::CorruptDocument
        }
    ));
    assert!(matches!(
        report("example.preferences").compatibility(),
        RestoreDomainCompatibility::UnknownDomain
    ));
    assert_eq!(inspection.exclusions().len(), 1);
    assert_eq!(
        inspection.exclusions()[0].exclusion().domain().as_str(),
        "example.excluded"
    );
    assert!(inspection.exclusions()[0].is_registered());
    assert!(!target.path_for(&absent).exists());
    assert_eq!(
        fs::read(target.path_for(&preserved)).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn inspection_distinguishes_migration_and_missing_migration_without_writes() {
    let donor = Fixture::new();
    let source = config_domain();
    donor.write(
        &source,
        &document("example.preferences", 1, json!({"label": "legacy"})),
    );
    let mut donor_store = donor.store();
    donor_store.register(&source).unwrap();
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&source).unwrap();
    let archive = archive(&donor_store, &donor_catalog);

    let target = Fixture::new();
    let complete = config_domain();
    let mut complete_store = target.store();
    complete_store.register(&complete).unwrap();
    let mut complete_catalog = BackupCatalog::new();
    complete_catalog.include(&complete).unwrap();
    let (application, producer) = identities();
    let compatible =
        complete_store.inspect_restore(&complete_catalog, &archive, &application, &producer);
    assert!(matches!(
        compatible.domains()[0].compatibility(),
        RestoreDomainCompatibility::MigrationRequired { from, to }
            if from.get() == 1 && to.get() == 3
    ));
    assert_eq!(compatible.receipt().migrations(), 1);

    let missing = config_domain().with_behavior(MigrationBehavior::MissingSecond);
    let mut missing_store = target.store();
    missing_store.register(&missing).unwrap();
    let mut missing_catalog = BackupCatalog::new();
    missing_catalog.include(&missing).unwrap();
    let rejected =
        missing_store.inspect_restore(&missing_catalog, &archive, &application, &producer);
    assert!(matches!(
        rejected.domains()[0].compatibility(),
        RestoreDomainCompatibility::SourceRejected {
            issue: BackupSourceIssue::MissingMigration
        }
    ));
    assert!(!target.path_for(&complete).exists());
}
