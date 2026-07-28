use std::{fs, path::PathBuf};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, MigrationStep, PlatformDirectoryFacts, ResolvedStorageLayout, StorageClass,
    StorageIdentity, StorageLayoutRequest, StorageProfile, StorageProfileSelection, TargetPlatform,
    resolve_storage_layout,
};
use longhorn_core::{DomainId, SchemaVersion};
use serde_json::Value;
use tempfile::TempDir;

pub(crate) struct TransitionFixture {
    pub(crate) temp: TempDir,
    pub(crate) identity: StorageIdentity,
    pub(crate) facts: PlatformDirectoryFacts,
    pub(crate) source: ResolvedStorageLayout,
    pub(crate) target: ResolvedStorageLayout,
    pub(crate) target_selection: StorageProfileSelection,
}

impl TransitionFixture {
    pub(crate) fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let identity = StorageIdentity::new("com.example.transition").unwrap();
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
        Self {
            temp,
            identity,
            facts,
            source,
            target,
            target_selection,
        }
    }

    pub(crate) fn store(&self, layout: &ResolvedStorageLayout) -> ConfigStore {
        for root in layout.diagnostic().roots() {
            fs::create_dir_all(root.path()).unwrap();
        }
        let authority_root = layout.storage_roots().data();
        fs::create_dir_all(authority_root).unwrap();
        ConfigStore::new(
            layout.storage_roots().clone(),
            CoordinationAuthority::new(authority_root).unwrap(),
        )
    }

    pub(crate) fn bootstrap(&self) -> longhorn_config::StorageBootstrapPaths {
        longhorn_config::resolve_storage_bootstrap_paths(&self.identity, &self.facts).unwrap()
    }
}

pub(crate) struct TestDomain {
    descriptor: DomainDescriptor,
}

impl TestDomain {
    pub(crate) fn new(id: &str, class: StorageClass, path: &str) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(id).unwrap(),
                SchemaVersion::new(1).unwrap(),
                class,
                Some(DomainFilePath::new(path).unwrap()),
            )
            .unwrap(),
        }
    }

    pub(crate) fn external(id: &str, class: StorageClass) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(id).unwrap(),
                SchemaVersion::new(1).unwrap(),
                class,
                None,
            )
            .unwrap(),
        }
    }

    pub(crate) fn path(&self, layout: &ResolvedStorageLayout) -> PathBuf {
        match layout.storage_roots().resolve(&self.descriptor) {
            longhorn_config::DomainLocation::File(file) => file.full_path().to_path_buf(),
            other => panic!("expected file, got {other:?}"),
        }
    }
}

impl ConfigDomain for TestDomain {
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
