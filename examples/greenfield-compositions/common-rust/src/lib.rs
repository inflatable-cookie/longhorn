use std::{fs, time::Duration};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, DurabilityRequirement, LoadOutcome, LoadedOrigin, MigrationStep, MutationOptions,
    PlatformDirectoryFact, PlatformDirectoryFacts, RootKind, StorageClass, StorageIdentity,
    StorageLayoutRequest, StorageProfile, StorageRoots, TargetPlatform, resolve_storage_layout,
};
use longhorn_core::{DomainId, SchemaVersion};
use serde_json::{Value, json};

pub fn run(shape: &str, application_id: &str) -> Value {
    let facts = PlatformDirectoryFacts::new(TargetPlatform::MacOs)
        .with(
            PlatformDirectoryFact::Config,
            "/platform/application-support",
        )
        .with(PlatformDirectoryFact::Data, "/platform/application-support")
        .with(
            PlatformDirectoryFact::State,
            "/platform/application-support",
        )
        .with(PlatformDirectoryFact::Cache, "/platform/caches")
        .with(PlatformDirectoryFact::Log, "/platform/logs")
        .with(PlatformDirectoryFact::Runtime, "/platform/runtime");
    let layout = resolve_storage_layout(
        &StorageLayoutRequest::new(StorageIdentity::new(application_id).unwrap(), facts)
            .with_profile(StorageProfile::PlatformNativeV1),
    )
    .unwrap();

    let temporary = tempfile::tempdir().unwrap();
    let roots = [
        "config", "data", "state", "cache", "runtime", "logs", "backups",
    ]
    .map(|name| temporary.path().join(name));
    for root in &roots {
        fs::create_dir_all(root).unwrap();
    }
    let storage_roots = StorageRoots::new(
        &roots[0], &roots[1], &roots[2], &roots[3], &roots[4], &roots[5], &roots[6],
    )
    .unwrap();
    let coordination = CoordinationAuthority::new(&roots[1]).unwrap();
    let mut store = ConfigStore::new(storage_roots, coordination);
    let domain = ExampleDomain::new(shape);
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(first) = store.load(&domain).unwrap() else {
        panic!("compiled default was not authoritative");
    };
    assert_eq!(first.origin, LoadedOrigin::Default);
    let receipt = store
        .mutate(
            &domain,
            MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic),
            |value| {
                value["enabled"] = Value::Bool(true);
                Ok(())
            },
        )
        .unwrap();
    let LoadOutcome::Ready(reloaded) = store.load(&domain).unwrap() else {
        panic!("published config was not readable");
    };
    assert_eq!(reloaded.origin, LoadedOrigin::File);
    assert_eq!(reloaded.value["enabled"], true);

    json!({
        "shape": shape,
        "applicationId": application_id,
        "storageProfile": "platform-native-v1",
        "configPath": layout.root(RootKind::Config).unwrap().path(),
        "cachePath": layout.root(RootKind::Cache).unwrap().path(),
        "firstLoad": "compiled-default",
        "mutation": "atomic-published",
        "mutationPathConfined": receipt.path.starts_with(&roots[0]),
        "reload": "file",
        "teardown": "temporary-roots-drop"
    })
}

struct ExampleDomain {
    descriptor: DomainDescriptor,
}

impl ExampleDomain {
    fn new(shape: &str) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(format!("greenfield.{shape}.preferences")).unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::UserConfig,
                Some(DomainFilePath::new("preferences.json").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for ExampleDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }
    fn default_value(&self) -> Self::Value {
        json!({ "enabled": false })
    }
    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        self.validate(&value)?;
        Ok(value)
    }
    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }
    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.get("enabled").is_some_and(Value::is_boolean) {
            Ok(())
        } else {
            Err(DomainIssue::new("enabled", "enabled must be boolean"))
        }
    }
    fn validate_raw(&self, version: SchemaVersion, value: &Value) -> Result<(), DomainIssue> {
        if version == self.descriptor.schema_version() {
            self.validate(value)
        } else {
            Err(DomainIssue::new("version", "unsupported schema version"))
        }
    }
    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}
