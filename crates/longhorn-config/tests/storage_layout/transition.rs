use std::{fs, time::Duration};

use longhorn_config::{
    ConfigDomain, LegacyStorageCandidate, StorageBootstrapOrigin, StorageBootstrapState,
    StorageClass, StorageFileEvidence, StorageRoots, StorageTransitionAction,
    StorageTransitionCatalog, StorageTransitionCleanupPlan, StorageTransitionConflictKind,
    StorageTransitionError, StorageTransitionExclusion, StorageTransitionExecutionOptions,
    StorageTransitionLimits, StorageTransitionOutcome, StorageTransitionPlanError,
    StorageTransitionRequest, apply_storage_transition_cleanup, discover_legacy_storage,
    execute_storage_transition, inspect_storage_bootstrap, inspect_storage_transition,
    plan_storage_transition,
};

use crate::transition_support::{TestDomain, TransitionFixture};

#[test]
fn ordinary_transition_is_locator_last_and_retains_only_registered_source_paths() {
    let fixture = TransitionFixture::new();
    let settings = TestDomain::new(
        "example.settings",
        StorageClass::UserConfig,
        "settings.json",
    );
    let cache = TestDomain::new("example.cache", StorageClass::Cache, "cache.json");
    let runtime = TestDomain::new("example.runtime", StorageClass::Runtime, "runtime.json");
    let logs = TestDomain::new("example.logs", StorageClass::Log, "events.json");
    let secret = TestDomain::external("example.secret", StorageClass::Secret);
    let source_bytes = br#"{"schemaVersion":1,"value":"source"}"#;
    for (domain, bytes) in [
        (&settings, source_bytes.as_slice()),
        (&cache, b"cache".as_slice()),
        (&runtime, b"runtime".as_slice()),
        (&logs, b"log".as_slice()),
    ] {
        let path = domain.path(&fixture.source);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let unknown = fixture.source.storage_roots().config().join("unknown.dat");
    fs::write(&unknown, b"preserve-me").unwrap();

    let mut source_store = fixture.store(&fixture.source);
    let mut target_store = fixture.store(&fixture.target);
    for domain in [&settings, &cache, &runtime, &logs, &secret] {
        source_store.register(domain).unwrap();
        target_store.register(domain).unwrap();
    }
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&settings).unwrap();
    catalog.include(&logs).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &fixture.source,
        &fixture.target,
        fixture.target_selection.clone(),
        &catalog,
        fixture.bootstrap(),
    );

    let logs_preview = inspect_storage_transition(
        &StorageTransitionRequest::new(
            &source_store,
            &target_store,
            &fixture.source,
            &fixture.target,
            fixture.target_selection.clone(),
            &catalog,
            fixture.bootstrap(),
        )
        .with_logs(true),
    )
    .unwrap();
    assert!(matches!(
        logs_preview
            .domains()
            .iter()
            .find(|entry| entry.domain() == logs.descriptor().id())
            .unwrap()
            .action(),
        StorageTransitionAction::CopyOrdinary
    ));

    let preview = inspect_storage_transition(&request).unwrap();
    assert!(preview.conflicts().is_empty());
    assert_eq!(preview.source_unknown().len(), 1);
    assert_eq!(preview.source_unknown()[0].path(), unknown);
    assert!(matches!(
        preview
            .domains()
            .iter()
            .find(|entry| entry.domain() == settings.descriptor().id())
            .unwrap()
            .action(),
        StorageTransitionAction::CopyOrdinary
    ));
    assert!(matches!(
        preview
            .domains()
            .iter()
            .find(|entry| entry.domain() == cache.descriptor().id())
            .unwrap()
            .action(),
        StorageTransitionAction::Excluded(StorageTransitionExclusion::CacheRebuilt)
    ));
    assert!(matches!(
        preview
            .domains()
            .iter()
            .find(|entry| entry.domain() == runtime.descriptor().id())
            .unwrap()
            .action(),
        StorageTransitionAction::Excluded(StorageTransitionExclusion::RuntimeDiscarded)
    ));
    assert!(matches!(
        preview
            .domains()
            .iter()
            .find(|entry| entry.domain() == logs.descriptor().id())
            .unwrap()
            .action(),
        StorageTransitionAction::Excluded(StorageTransitionExclusion::LogsNotSelected)
    ));
    assert!(matches!(
        preview
            .domains()
            .iter()
            .find(|entry| entry.domain() == secret.descriptor().id())
            .unwrap()
            .action(),
        StorageTransitionAction::Excluded(StorageTransitionExclusion::SecretExternal)
    ));
    let plan = plan_storage_transition(&preview).unwrap();

    fs::write(settings.path(&fixture.source), b"changed-after-plan").unwrap();
    let stale = execute_storage_transition(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("transition-stale", Duration::from_secs(2)).unwrap(),
    )
    .unwrap_err();
    assert!(
        matches!(stale, StorageTransitionError::StalePlan),
        "{stale:?}"
    );
    fs::write(settings.path(&fixture.source), source_bytes).unwrap();

    let preview = inspect_storage_transition(&request).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();
    let receipt = execute_storage_transition(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("transition-ordinary", Duration::from_secs(2))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt.outcome(), StorageTransitionOutcome::TargetCommitted);
    assert_eq!(
        receipt.copied_domains(),
        [settings.descriptor().id().clone()]
    );
    assert_eq!(
        fs::read(settings.path(&fixture.target)).unwrap(),
        source_bytes
    );
    assert_eq!(
        fs::read(settings.path(&fixture.source)).unwrap(),
        source_bytes
    );
    assert_eq!(fs::read(&unknown).unwrap(), b"preserve-me");
    assert!(!cache.path(&fixture.target).exists());
    assert!(!runtime.path(&fixture.target).exists());
    assert!(!logs.path(&fixture.target).exists());

    let StorageBootstrapState::Selected(selected) =
        inspect_storage_bootstrap(&fixture.identity, &fixture.facts, None).unwrap()
    else {
        panic!("committed locator did not select target");
    };
    assert_eq!(selected.origin(), StorageBootstrapOrigin::Locator);
    assert_eq!(selected.selection(), &fixture.target_selection);
    assert_eq!(
        selected.last_committed_layout_digest(),
        Some(fixture.target.digest())
    );
    let cleanup = StorageTransitionCleanupPlan::from_receipt(&receipt).unwrap();
    assert_eq!(cleanup.paths(), [settings.path(&fixture.source)]);
    assert!(!cleanup.paths().contains(&unknown));
    let cleaned =
        apply_storage_transition_cleanup(&request, &receipt, &cleanup, Duration::from_secs(2))
            .unwrap();
    assert_eq!(cleaned.deleted_paths(), [settings.path(&fixture.source)]);
    assert!(!settings.path(&fixture.source).exists());
    assert_eq!(fs::read(&unknown).unwrap(), b"preserve-me");

    let repeated =
        apply_storage_transition_cleanup(&request, &receipt, &cleanup, Duration::from_secs(2))
            .unwrap();
    assert_eq!(
        repeated.already_absent_paths(),
        [settings.path(&fixture.source)]
    );
}

#[test]
fn target_state_and_overlapping_roots_block_before_mutation() {
    let fixture = TransitionFixture::new();
    let domain = TestDomain::new(
        "example.settings",
        StorageClass::UserConfig,
        "settings.json",
    );
    for (layout, bytes) in [
        (&fixture.source, b"source".as_slice()),
        (&fixture.target, b"target".as_slice()),
    ] {
        let path = domain.path(layout);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let mut source = fixture.store(&fixture.source);
    let mut target = fixture.store(&fixture.target);
    source.register(&domain).unwrap();
    target.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let request = StorageTransitionRequest::new(
        &source,
        &target,
        &fixture.source,
        &fixture.target,
        fixture.target_selection.clone(),
        &catalog,
        fixture.bootstrap(),
    );
    let preview = inspect_storage_transition(&request).unwrap();
    assert!(
        preview
            .conflicts()
            .iter()
            .any(|conflict| { conflict.kind() == StorageTransitionConflictKind::TargetOccupied })
    );
    assert_eq!(
        plan_storage_transition(&preview),
        Err(StorageTransitionPlanError::Conflicts { count: 1 })
    );
    assert_eq!(fs::read(domain.path(&fixture.target)).unwrap(), b"target");

    let target_unknown = fixture.target.storage_roots().config().join("unknown.bin");
    fs::write(&target_unknown, b"target-unknown").unwrap();
    let unknown_preview = inspect_storage_transition(&request).unwrap();
    assert!(unknown_preview.conflicts().iter().any(|conflict| {
        conflict.kind() == StorageTransitionConflictKind::UnknownTargetFile
            && conflict.path() == Some(target_unknown.as_path())
    }));

    let nested_root = fixture
        .source
        .storage_roots()
        .config()
        .join("nested-portable");
    let nested_selection =
        longhorn_config::StorageProfileSelection::portable(&nested_root).unwrap();
    let nested_layout = longhorn_config::resolve_storage_layout(
        &longhorn_config::StorageLayoutRequest::new(
            fixture.identity.clone(),
            fixture.facts.clone(),
        )
        .with_profile(longhorn_config::StorageProfile::PortableV1)
        .with_portable_root(nested_root),
    )
    .unwrap();
    let mut nested_store = fixture.store(&nested_layout);
    nested_store.register(&domain).unwrap();
    let nested_request = StorageTransitionRequest::new(
        &source,
        &nested_store,
        &fixture.source,
        &nested_layout,
        nested_selection,
        &catalog,
        fixture.bootstrap(),
    );
    let nested = inspect_storage_transition(&nested_request).unwrap();
    assert!(
        nested
            .conflicts()
            .iter()
            .any(|conflict| { conflict.kind() == StorageTransitionConflictKind::OverlappingRoots })
    );
}

#[test]
fn shared_product_target_is_visible_in_transition_evidence() {
    let fixture = TransitionFixture::new();
    let identity = longhorn_config::StorageIdentity::new("com.inflatablecookie.loophole")
        .unwrap()
        .with_storage_name("Loophole")
        .unwrap();
    let facts = fixture.facts.clone().with(
        longhorn_config::PlatformDirectoryFact::SharedData,
        fixture.temp.path().join("shared-product-data"),
    );
    let source = longhorn_config::resolve_storage_layout(
        &longhorn_config::StorageLayoutRequest::new(identity.clone(), facts.clone()),
    )
    .unwrap();
    let target = longhorn_config::resolve_storage_layout(
        &longhorn_config::StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(longhorn_config::StorageProfile::SharedProductRootV1),
    )
    .unwrap();
    let domain = TestDomain::new(
        "loophole.app-settings",
        StorageClass::UserConfig,
        "app/settings.json",
    );
    let mut source_store = fixture.store(&source);
    let mut target_store = fixture.store(&target);
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let selection = longhorn_config::StorageProfileSelection::shared_product();
    let bootstrap = longhorn_config::resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        selection.clone(),
        &catalog,
        bootstrap,
    );

    let preview = inspect_storage_transition(&request).unwrap();
    assert!(preview.conflicts().is_empty());
    assert_eq!(preview.target_selection(), &selection);
    assert_eq!(
        preview.target_layout_digest(),
        target.digest(),
        "transition evidence must bind the selected shared-product layout"
    );
    assert!(target.storage_roots().config().ends_with("Loophole/config"));
}

#[test]
fn same_layout_adoption_allows_derived_workspace_under_state() {
    let fixture = TransitionFixture::new();
    let identity = longhorn_config::StorageIdentity::new("com.inflatablecookie.loophole")
        .unwrap()
        .with_storage_name("Loophole")
        .unwrap();
    let facts = fixture.facts.clone().with(
        longhorn_config::PlatformDirectoryFact::SharedData,
        fixture.temp.path().join("shared-product-data"),
    );
    let layout = longhorn_config::resolve_storage_layout(
        &longhorn_config::StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(longhorn_config::StorageProfile::SharedProductRootV1),
    )
    .unwrap();
    let domain = TestDomain::new(
        "loophole.app-settings",
        StorageClass::UserConfig,
        "app-settings.json",
    );
    let mut source_store = fixture.store(&layout);
    let mut target_store = fixture.store(&layout);
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let retained = layout
        .storage_roots()
        .data()
        .join("large-retained-product-data.bin");
    fs::create_dir_all(retained.parent().unwrap()).unwrap();
    fs::write(&retained, b"not transition input").unwrap();
    let bootstrap = longhorn_config::resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &layout,
        &layout,
        longhorn_config::StorageProfileSelection::shared_product(),
        &catalog,
        bootstrap,
    );

    let preview = inspect_storage_transition(&request).unwrap();
    assert!(preview.conflicts().is_empty());
    assert!(
        preview.source_unknown().is_empty(),
        "same-layout profile adoption must not inventory unrelated retained data"
    );
    assert_eq!(
        preview.domains()[0].action(),
        &StorageTransitionAction::SameAuthority
    );
}

#[test]
fn declared_legacy_candidates_are_discovered_read_only_with_unknowns_preserved() {
    let fixture = TransitionFixture::new();
    let domains = [
        TestDomain::new(
            "loophole.machine-window-layout",
            StorageClass::MachineState,
            "loophole/state.json",
        ),
        TestDomain::new(
            "soundcheck.settings-window",
            StorageClass::UserConfig,
            "soundcheck/settings.json",
        ),
        TestDomain::new(
            "nucleus.workspace-state",
            StorageClass::MachineState,
            "nucleus/state.json",
        ),
        TestDomain::new(
            "bovine.workspace-presentation",
            StorageClass::UserConfig,
            "bovine/workspace.json",
        ),
    ];
    let mut registry = fixture.store(&fixture.source);
    for domain in &domains {
        registry.register(domain).unwrap();
    }
    let names = ["loophole", "soundcheck", "nucleus", "bovine"];
    let mut candidates = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let root = fixture.temp.path().join("legacy").join(name);
        let roots = legacy_roots(&root);
        let candidate = LegacyStorageCandidate::new(*name, roots.clone()).unwrap();
        let path = match roots.resolve(domains[index].descriptor()) {
            longhorn_config::DomainLocation::File(file) => file.full_path().to_path_buf(),
            _ => unreachable!(),
        };
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, name.as_bytes()).unwrap();
        fs::write(path.parent().unwrap().join("unknown.bin"), b"unknown").unwrap();
        candidates.push(candidate);
    }
    let discoveries =
        discover_legacy_storage(&registry, &candidates, StorageTransitionLimits::default())
            .unwrap();
    assert_eq!(
        discoveries
            .iter()
            .map(|entry| entry.candidate_id())
            .collect::<Vec<_>>(),
        names
    );
    for (index, discovery) in discoveries.iter().enumerate() {
        let found = discovery
            .domains()
            .iter()
            .find(|entry| entry.domain() == domains[index].descriptor().id())
            .unwrap();
        assert!(matches!(
            found.source_evidence(),
            Some(StorageFileEvidence::Present { .. })
        ));
        assert_eq!(discovery.unknown_files().len(), 1);
        assert!(found.source_path().unwrap().exists());
    }
}

fn legacy_roots(root: &std::path::Path) -> StorageRoots {
    StorageRoots::new(
        root.join("config"),
        root.join("data"),
        root.join("state"),
        root.join("cache"),
        root.join("runtime"),
        root.join("logs"),
        root.join("backups"),
    )
    .unwrap()
    .with_workspace(root.join("workspaces"))
    .unwrap()
}
