use std::{fs, time::Duration};

use longhorn_core::{DomainId, SchemaVersion};
use serde_json::Value;
use tempfile::tempdir;

use crate::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, MigrationStep, PlatformDirectoryFacts, StorageClass, StorageIdentity,
    StorageLayoutRequest, StorageProfile, StorageProfileSelection,
    StorageTransitionExecutionOptions, StorageTransitionOutcome, TargetPlatform,
    resolve_storage_bootstrap_paths, resolve_storage_layout,
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
