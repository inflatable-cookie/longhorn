use std::{fs, time::Duration};

use longhorn_config::{
    BackupApplication, BackupArchiveLimits, BackupCaptureOptions, BackupCatalog, BackupKind,
    BackupLimits, BackupMetadata, BackupProducer, BackupScope, ConfigDomain, ConfigStore,
    CoordinationAuthority, DomainDescriptor, DomainFilePath, DomainIssue, MigrationStep,
    StorageClass, StorageRoots, encode_backup_archive,
};
use longhorn_core::{DomainId, SchemaVersion};
use serde_json::{Value, json};
use tempfile::TempDir;

pub(crate) const APP_ID: &str = "com.example.encrypted";
pub(crate) const DOMAIN_ID: &str = "example.private-preferences";

struct TestDomain {
    descriptor: DomainDescriptor,
}

impl TestDomain {
    fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(DOMAIN_ID).unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::UserConfig,
                Some(DomainFilePath::new("example/private-preferences.json").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for TestDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        json!({"theme": "default"})
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        Ok(value)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.is_object() {
            Ok(())
        } else {
            Err(DomainIssue::new("shape", "value must be an object"))
        }
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version == SchemaVersion::new(1).unwrap() {
            self.validate(value)
        } else {
            Err(DomainIssue::new("schema", "unsupported schema"))
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

pub(crate) fn encoded_archive(
    archive_id: &str,
    kind: BackupKind,
) -> longhorn_config::EncodedBackupArchive {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("config");
    let data = temp.path().join("data");
    let state = temp.path().join("state");
    let cache = temp.path().join("cache");
    let runtime = temp.path().join("runtime");
    let log = temp.path().join("log");
    let backup = temp.path().join("backup");
    for root in [&config, &data, &state, &cache, &runtime, &log] {
        fs::create_dir_all(root).unwrap();
    }
    let roots = StorageRoots::new(&config, &data, &state, &cache, &runtime, &log, &backup).unwrap();
    let coordination = CoordinationAuthority::new(&data).unwrap();
    let domain = TestDomain::new();
    let source = config.join("example/private-preferences.json");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(
        source,
        serde_json::to_vec(&json!({
            "domain": DOMAIN_ID,
            "schemaVersion": 1,
            "value": {"theme": "midnight"}
        }))
        .unwrap(),
    )
    .unwrap();

    let mut store = ConfigStore::new(roots, coordination);
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();
    let snapshot = store
        .capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            BackupMetadata::new(
                archive_id,
                kind,
                "2026-07-28T12:00:00Z",
                BackupApplication::new(APP_ID, "1.0.0").unwrap(),
                BackupProducer::new("longhorn-config-age", "0.1.0").unwrap(),
            )
            .unwrap(),
            BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        )
        .unwrap();
    encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap()
}
