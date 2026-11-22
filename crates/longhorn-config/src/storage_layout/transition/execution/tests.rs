use std::{fs, time::Duration};

use longhorn_core::{DomainId, SchemaVersion};
use serde_json::Value;
use tempfile::tempdir;

use crate::StorageRootsResolve;
use crate::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, MigrationStep, StorageClass, StorageTransitionExecutionOptions,
    StorageTransitionOutcome,
};
use longhorn_storage_layout::{
    PlatformDirectoryFacts, StorageIdentity, StorageLayoutRequest, StorageProfile,
    StorageProfileSelection, TargetPlatform, resolve_storage_bootstrap_paths,
    resolve_storage_layout,
};

use super::*;
use crate::storage_layout::transition::{
    StorageTransitionCatalog, StorageTransitionError, StorageTransitionRequest,
    inspect_storage_transition, plan_storage_transition,
};

struct Domain {
    descriptor: DomainDescriptor,
}

impl Domain {
    fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new("example.failure").unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::UserConfig,
                Some(DomainFilePath::new("failure.json").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for Domain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }
    fn default_value(&self) -> Self::Value {
        Value::Null
    }
    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        Ok(value)
    }
    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }
    fn validate(&self, _value: &Self::Value) -> Result<(), DomainIssue> {
        Ok(())
    }
    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        _value: &Value,
    ) -> Result<(), DomainIssue> {
        Ok(())
    }
    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

#[test]
fn failure_before_locator_recovers_verified_source_authority() {
    assert_eq!(
        run_failure(InjectedFailure::BeforeLocator),
        StorageTransitionOutcome::SourceRetained
    );
}

#[test]
fn failure_after_locator_recovers_verified_target_authority() {
    assert_eq!(
        run_failure(InjectedFailure::AfterLocator),
        StorageTransitionOutcome::TargetCommitted
    );
}

#[test]
fn source_retained_recovery_removes_partial_target_so_retry_succeeds() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.retry").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_root = temp.path().join("portable");
    let target_selection = StorageProfileSelection::portable(&portable_root).unwrap();
    let target = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_root),
    )
    .unwrap();
    for layout in [&source, &target] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let DomainLocation::File(source_file) = source.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let DomainLocation::File(target_file) = target.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    fs::write(source_file.full_path(), b"v1-source").unwrap();
    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_store = ConfigStore::new(
        target.storage_roots().clone(),
        CoordinationAuthority::new(target.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        target_selection,
        &catalog,
        bootstrap,
    );
    let preview = inspect_storage_transition(&request).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();
    let err = execute_inner(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("attempt-1", Duration::from_secs(2)).unwrap(),
        Some(InjectedFailure::BeforeLocator),
    )
    .unwrap_err();
    assert!(matches!(err, StorageTransitionError::RecoveryRequired(_)));
    assert!(target_file.full_path().is_file());

    let recovery = recover_storage_transition(&request, Duration::from_secs(2))
        .unwrap()
        .unwrap();
    assert_eq!(recovery.outcome(), StorageTransitionOutcome::SourceRetained);
    assert!(
        !target_file.full_path().exists(),
        "SourceRetained recovery must remove partial target copies"
    );

    // Source mutates after staying on SourceRetained; retrying must not fail with TargetOccupied.
    fs::write(source_file.full_path(), b"v2-source").unwrap();
    let retry_preview = inspect_storage_transition(&request).unwrap();
    assert!(retry_preview.conflicts().is_empty());
    let retry_plan = plan_storage_transition(&retry_preview).unwrap();
    let receipt = execute_storage_transition(
        &request,
        &retry_plan,
        retry_plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("attempt-2", Duration::from_secs(2)).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt.outcome(), StorageTransitionOutcome::TargetCommitted);
    assert_eq!(fs::read(target_file.full_path()).unwrap(), b"v2-source");
}

#[test]
fn occupied_ordinary_target_records_conflict_action_instead_of_copy() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.conflict").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_root = temp.path().join("portable");
    let target_selection = StorageProfileSelection::portable(&portable_root).unwrap();
    let target = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_root),
    )
    .unwrap();
    for layout in [&source, &target] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let DomainLocation::File(source_file) = source.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let DomainLocation::File(target_file) = target.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    fs::write(source_file.full_path(), b"source-data").unwrap();
    fs::write(target_file.full_path(), b"conflicting-target-data").unwrap();
    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_store = ConfigStore::new(
        target.storage_roots().clone(),
        CoordinationAuthority::new(target.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        target_selection,
        &catalog,
        bootstrap,
    );

    let preview = inspect_storage_transition(&request).unwrap();
    assert_eq!(preview.conflicts().len(), 1);
    assert_eq!(
        preview.domains()[0].action(),
        &crate::StorageTransitionAction::Conflict
    );
}

#[test]
fn source_retained_recovery_preserves_modified_target_and_requires_recovery() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.preserve").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_root = temp.path().join("portable");
    let target_selection = StorageProfileSelection::portable(&portable_root).unwrap();
    let target = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_root),
    )
    .unwrap();
    for layout in [&source, &target] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let DomainLocation::File(source_file) = source.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let DomainLocation::File(target_file) = target.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    fs::write(source_file.full_path(), b"v1-source").unwrap();
    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_store = ConfigStore::new(
        target.storage_roots().clone(),
        CoordinationAuthority::new(target.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        target_selection,
        &catalog,
        bootstrap.clone(),
    );
    let preview = inspect_storage_transition(&request).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();
    let _ = execute_inner(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("attempt-1", Duration::from_secs(2)).unwrap(),
        Some(InjectedFailure::BeforeLocator),
    )
    .unwrap_err();

    // Another writer replaces the target with same-length different bytes
    // (exercising the SHA-256 digest check, not just the length check).
    fs::write(target_file.full_path(), b"v2-source").unwrap();

    let err = recover_storage_transition(&request, Duration::from_secs(2)).unwrap_err();
    assert!(matches!(err, StorageTransitionError::RecoveryRequired(_)));
    assert_eq!(
        fs::read(target_file.full_path()).unwrap(),
        b"v2-source",
        "recovery must never delete a target file whose contents do not match the interrupted copy"
    );
    assert!(bootstrap.journal().is_file());
}

#[test]
fn source_retained_recovery_syncs_parent_on_retry_after_post_unlink_sync_failure() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.sync-retry").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_root = temp.path().join("portable");
    let target_selection = StorageProfileSelection::portable(&portable_root).unwrap();
    let target = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_root),
    )
    .unwrap();
    for layout in [&source, &target] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let DomainLocation::File(source_file) = source.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let DomainLocation::File(target_file) = target.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    fs::write(source_file.full_path(), b"v1-source").unwrap();
    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_store = ConfigStore::new(
        target.storage_roots().clone(),
        CoordinationAuthority::new(target.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        target_selection,
        &catalog,
        bootstrap.clone(),
    );
    let preview = inspect_storage_transition(&request).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();
    let _ = execute_inner(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("sync-retry", Duration::from_secs(2)).unwrap(),
        Some(InjectedFailure::BeforeLocator),
    )
    .unwrap_err();
    assert!(target_file.full_path().is_file());

    // First recovery unlinks the target file, then fails directory sync.
    let first_err = recover_inner(&request, Duration::from_secs(2), |_| {
        Err(std::io::Error::other("injected post-unlink sync failure"))
    })
    .unwrap_err();
    assert!(matches!(
        first_err,
        StorageTransitionError::Filesystem { .. }
    ));
    assert!(!target_file.full_path().exists());
    assert!(bootstrap.journal().is_file());

    // Retry sees NotFound, yet must still perform the destination parent sync
    // barrier before removing the journal.
    let sync_calls = AtomicUsize::new(0);
    let receipt = recover_inner(&request, Duration::from_secs(2), |dir| {
        sync_calls.fetch_add(1, Ordering::SeqCst);
        crate::dir_sync::sync_dir_handle(dir)
    })
    .unwrap()
    .unwrap();
    assert_eq!(receipt.outcome(), StorageTransitionOutcome::SourceRetained);
    assert!(
        sync_calls.load(Ordering::SeqCst) >= 1,
        "retry after post-unlink sync failure must still sync the destination parent"
    );
    assert!(!bootstrap.journal().exists());
}

#[test]
fn source_retained_recovery_skips_cleanup_for_prepared_journal() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.prepared").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_root = temp.path().join("portable");
    let target_selection = StorageProfileSelection::portable(&portable_root).unwrap();
    let target = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_root),
    )
    .unwrap();
    for layout in [&source, &target] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let DomainLocation::File(source_file) = source.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let DomainLocation::File(target_file) = target.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    fs::write(source_file.full_path(), b"v1-source").unwrap();
    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_store = ConfigStore::new(
        target.storage_roots().clone(),
        CoordinationAuthority::new(target.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        target_selection,
        &catalog,
        bootstrap.clone(),
    );
    let preview = inspect_storage_transition(&request).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();

    // Write a Prepared journal (before Publishing / any copy could start).
    let prepared = crate::storage_layout::transition::journal::TransitionJournal::from_plan(
        "prepared-only".into(),
        &plan,
    );
    crate::storage_layout::transition::journal::write(&bootstrap, &prepared).unwrap();

    // An independent actor places byte-identical content at the target path.
    fs::write(target_file.full_path(), b"v1-source").unwrap();

    let receipt = recover_storage_transition(&request, Duration::from_secs(2))
        .unwrap()
        .unwrap();
    assert_eq!(receipt.outcome(), StorageTransitionOutcome::SourceRetained);
    assert_eq!(
        fs::read(target_file.full_path()).unwrap(),
        b"v1-source",
        "Prepared recovery must not delete an independently placed target file"
    );
    assert!(!bootstrap.journal().exists());
}

#[test]
fn transition_rejects_oversized_legacy_ordinary_domain_before_publishing() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.oversized").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_root = temp.path().join("portable");
    let target_selection = StorageProfileSelection::portable(&portable_root).unwrap();
    let target = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_root),
    )
    .unwrap();
    for layout in [&source, &target] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let DomainLocation::File(source_file) = source.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let DomainLocation::File(target_file) = target.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let oversized = vec![b' '; crate::store::load::max_live_domain_bytes() + 1];
    fs::write(source_file.full_path(), &oversized).unwrap();

    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_store = ConfigStore::new(
        target.storage_roots().clone(),
        CoordinationAuthority::new(target.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        target_selection,
        &catalog,
        bootstrap.clone(),
    );
    let preview = inspect_storage_transition(&request).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();

    let err = execute_storage_transition(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("oversized-legacy", Duration::from_secs(2)).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(err, StorageTransitionError::BoundExceeded { .. }));
    assert!(!target_file.full_path().exists());
    assert!(!bootstrap.locator().exists());
    assert!(!bootstrap.journal().exists());
    assert_eq!(
        fs::read(source_file.full_path()).unwrap().len(),
        oversized.len()
    );
}

#[test]
fn source_retained_recovery_rejects_stale_journal_layout_identity() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.stale").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_a = temp.path().join("portable-a");
    let portable_b = temp.path().join("portable-b");
    let target_a = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_a.clone()),
    )
    .unwrap();
    let target_b = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_b.clone()),
    )
    .unwrap();
    for layout in [&source, &target_a, &target_b] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let DomainLocation::File(source_file) = source.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    let DomainLocation::File(target_a_file) = target_a.storage_roots().resolve(domain.descriptor())
    else {
        unreachable!()
    };
    fs::write(source_file.full_path(), b"v1-source").unwrap();
    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_a_store = ConfigStore::new(
        target_a.storage_roots().clone(),
        CoordinationAuthority::new(target_a.storage_roots().data()).unwrap(),
    );
    let mut target_b_store = ConfigStore::new(
        target_b.storage_roots().clone(),
        CoordinationAuthority::new(target_b.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_a_store.register(&domain).unwrap();
    target_b_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request_a = StorageTransitionRequest::new(
        &source_store,
        &target_a_store,
        &source,
        &target_a,
        StorageProfileSelection::portable(&portable_a).unwrap(),
        &catalog,
        bootstrap.clone(),
    );
    let preview = inspect_storage_transition(&request_a).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();
    let _ = execute_inner(
        &request_a,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("attempt-a", Duration::from_secs(2)).unwrap(),
        Some(InjectedFailure::BeforeLocator),
    )
    .unwrap_err();

    // Recovering against a different target layout must refuse and leave target_a intact.
    let request_b = StorageTransitionRequest::new(
        &source_store,
        &target_b_store,
        &source,
        &target_b,
        StorageProfileSelection::portable(&portable_b).unwrap(),
        &catalog,
        bootstrap,
    );
    let err = recover_storage_transition(&request_b, Duration::from_secs(2)).unwrap_err();
    assert!(matches!(err, StorageTransitionError::RecoveryRequired(_)));
    assert_eq!(fs::read(target_a_file.full_path()).unwrap(), b"v1-source");
}

fn run_failure(injected: InjectedFailure) -> StorageTransitionOutcome {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.failure").unwrap();
    let facts = PlatformDirectoryFacts::complete(
        TargetPlatform::Linux,
        temp.path().join("native/config"),
        temp.path().join("native/data"),
        temp.path().join("native/state"),
        temp.path().join("native/cache"),
        temp.path().join("native/log"),
        temp.path().join("native/runtime"),
    );
    let source =
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), facts.clone()))
            .unwrap();
    let portable_root = temp.path().join("portable");
    let target_selection = StorageProfileSelection::portable(&portable_root).unwrap();
    let target = resolve_storage_layout(
        &StorageLayoutRequest::new(identity.clone(), facts.clone())
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root(portable_root),
    )
    .unwrap();
    for layout in [&source, &target] {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
    }
    let domain = Domain::new();
    let source_path = match source.storage_roots().resolve(domain.descriptor()) {
        DomainLocation::File(file) => file.full_path().to_path_buf(),
        _ => unreachable!(),
    };
    fs::write(&source_path, b"authoritative-source").unwrap();
    let mut source_store = ConfigStore::new(
        source.storage_roots().clone(),
        CoordinationAuthority::new(source.storage_roots().data()).unwrap(),
    );
    let mut target_store = ConfigStore::new(
        target.storage_roots().clone(),
        CoordinationAuthority::new(target.storage_roots().data()).unwrap(),
    );
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog.include(&domain).unwrap();
    let bootstrap = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &source,
        &target,
        target_selection,
        &catalog,
        bootstrap.clone(),
    );
    let preview = inspect_storage_transition(&request).unwrap();
    let plan = plan_storage_transition(&preview).unwrap();
    let result = execute_inner(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("failure-transition", Duration::from_secs(2))
            .unwrap(),
        Some(injected),
    );
    assert!(matches!(
        result,
        Err(StorageTransitionError::RecoveryRequired(_))
    ));
    assert!(bootstrap.journal().is_file());
    let recovery = recover_storage_transition(&request, Duration::from_secs(2))
        .unwrap()
        .unwrap();
    assert!(!bootstrap.journal().exists());
    assert_eq!(fs::read(source_path).unwrap(), b"authoritative-source");
    recovery.outcome()
}

use crate::DomainLocation;
