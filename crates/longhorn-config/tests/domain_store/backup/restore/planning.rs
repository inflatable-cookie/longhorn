use super::*;

#[test]
fn planning_requires_exact_choices_and_derives_all_action_shapes() {
    let cases = [
        (None, Some(3), RestoreAction::Create),
        (Some(3), Some(3), RestoreAction::Replace),
        (Some(3), None, RestoreAction::Delete),
        (None, Some(1), RestoreAction::Migrate),
    ];

    for (current_schema, archive_schema, expected) in cases {
        let donor = Fixture::new();
        let donor_domain = config_domain();
        if let Some(schema) = archive_schema {
            let value = if schema == 1 {
                json!({"label": "archive"})
            } else {
                json!({"name": "archive", "enabled": true})
            };
            donor.write(
                &donor_domain,
                &document("example.preferences", schema, value),
            );
        }
        let mut donor_store = donor.store();
        donor_store.register(&donor_domain).unwrap();
        let mut donor_catalog = BackupCatalog::new();
        donor_catalog.include(&donor_domain).unwrap();
        let archive = archive(&donor_store, &donor_catalog);

        let target = Fixture::new();
        let target_domain = config_domain();
        if let Some(schema) = current_schema {
            target.write(
                &target_domain,
                &document(
                    "example.preferences",
                    schema,
                    json!({"name": "current", "enabled": false}),
                ),
            );
        }
        let mut target_store = target.store();
        target_store.register(&target_domain).unwrap();
        let mut target_catalog = BackupCatalog::new();
        target_catalog.include(&target_domain).unwrap();
        let (application, producer) = identities();
        let inspection =
            target_store.inspect_restore(&target_catalog, &archive, &application, &producer);
        let domain = DomainId::new("example.preferences").unwrap();

        assert!(matches!(
            target_store.plan_restore(&inspection, &RestoreChoices::new()),
            Err(RestorePlanError::MissingChoices { .. })
        ));
        let unexpected = choices([
            (domain.clone(), RestoreConflictChoice::UseArchive),
            (
                DomainId::new("example.unknown").unwrap(),
                RestoreConflictChoice::KeepCurrent,
            ),
        ]);
        assert!(matches!(
            target_store.plan_restore(&inspection, &unexpected),
            Err(RestorePlanError::UnexpectedChoices { .. })
        ));

        let plan = target_store
            .plan_restore(
                &inspection,
                &choices([(domain, RestoreConflictChoice::UseArchive)]),
            )
            .unwrap();
        assert_eq!(plan.entries()[0].action(), Some(expected));
        assert_eq!(plan.receipt().actions(expected), 1);
    }
}

#[test]
fn plan_digest_binds_conflict_choice_action_and_current_evidence() {
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
                3,
                json!({"name": "archive", "enabled": true}),
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
    let mut target_store = target.store();
    target_store.register(&alpha).unwrap();
    target_store.register(&beta).unwrap();
    let mut target_catalog = BackupCatalog::new();
    target_catalog.include(&alpha).unwrap();
    target_catalog.include(&beta).unwrap();
    let (application, producer) = identities();
    let inspection =
        target_store.inspect_restore(&target_catalog, &archive, &application, &producer);
    let both = choices([
        (
            alpha.descriptor().id().clone(),
            RestoreConflictChoice::UseArchive,
        ),
        (
            beta.descriptor().id().clone(),
            RestoreConflictChoice::UseArchive,
        ),
    ]);
    let skip_beta = choices([
        (
            alpha.descriptor().id().clone(),
            RestoreConflictChoice::UseArchive,
        ),
        (
            beta.descriptor().id().clone(),
            RestoreConflictChoice::KeepCurrent,
        ),
    ]);
    let plan = target_store.plan_restore(&inspection, &both).unwrap();
    let repeated = target_store.plan_restore(&inspection, &both).unwrap();
    let conflict_changed = target_store.plan_restore(&inspection, &skip_beta).unwrap();
    assert_eq!(plan.digest(), repeated.digest());
    assert_ne!(plan.digest(), conflict_changed.digest());

    let changed_donor = Fixture::new();
    for domain in [&alpha, &beta] {
        changed_donor.write(
            domain,
            &document(
                domain.descriptor().id().as_str(),
                3,
                json!({"name": "different-archive", "enabled": true}),
            ),
        );
    }
    let mut changed_donor_store = changed_donor.store();
    changed_donor_store.register(&alpha).unwrap();
    changed_donor_store.register(&beta).unwrap();
    let changed_archive = super::archive(&changed_donor_store, &donor_catalog);
    let changed_inspection =
        target_store.inspect_restore(&target_catalog, &changed_archive, &application, &producer);
    let archive_changed = target_store
        .plan_restore(&changed_inspection, &both)
        .unwrap();
    assert_ne!(plan.digest(), archive_changed.digest());

    target.write(
        &alpha,
        &document(
            "a.preferences",
            3,
            json!({"name": "archive", "enabled": true}),
        ),
    );
    let evidence_changed = target_store.plan_restore(&inspection, &both).unwrap();
    assert_ne!(plan.digest(), evidence_changed.digest());
    assert_eq!(
        evidence_changed.entries()[0].action(),
        Some(RestoreAction::Unchanged)
    );
}
