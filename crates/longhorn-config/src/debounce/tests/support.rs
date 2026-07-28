use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use longhorn_core::{DomainId, SchemaVersion};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DebounceClock, DebouncePolicy,
    DebounceStrategy, DomainDescriptor, DomainFilePath, DomainIssue, DurabilityRequirement,
    MutationOptions, StorageClass, StorageRoots,
};

#[derive(Clone, Default)]
pub(super) struct FakeClock(Arc<AtomicU64>);

impl FakeClock {
    pub(super) fn set_millis(&self, value: u64) {
        self.0.store(value, Ordering::SeqCst);
    }
}

impl DebounceClock for FakeClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst))
    }
}

pub(super) struct Settings {
    name: String,
    enabled: bool,
}

pub(super) struct TestDomain {
    descriptor: DomainDescriptor,
}

impl TestDomain {
    pub(super) fn new(id: &str, path: &str) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(id).unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::UserConfig,
                Some(DomainFilePath::new(path).unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for TestDomain {
    type Value = Settings;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        Settings {
            name: "default".to_owned(),
            enabled: true,
        }
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        Ok(Settings {
            name: value["name"]
                .as_str()
                .ok_or_else(|| DomainIssue::new("name", "name is required"))?
                .to_owned(),
            enabled: value["enabled"]
                .as_bool()
                .ok_or_else(|| DomainIssue::new("enabled", "enabled is required"))?,
        })
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(json!({
            "name": value.name,
            "enabled": value.enabled,
        }))
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.name.is_empty() {
            Err(DomainIssue::new("empty-name", "name cannot be empty"))
        } else {
            Ok(())
        }
    }

    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if value["name"].is_string() && value["enabled"].is_boolean() {
            Ok(())
        } else {
            Err(DomainIssue::new("shape", "settings shape is invalid"))
        }
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<crate::MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

#[derive(Clone)]
pub(super) struct PatchIntent {
    name: Option<String>,
    enabled: Option<bool>,
}

impl PatchIntent {
    pub(super) fn name(value: &str) -> Self {
        Self {
            name: Some(value.to_owned()),
            enabled: None,
        }
    }

    pub(super) fn enabled(value: bool) -> Self {
        Self {
            name: None,
            enabled: Some(value),
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct PatchStrategy {
    pub(super) fail_once: Arc<AtomicBool>,
    pub(super) applications: Arc<AtomicUsize>,
}

impl DebounceStrategy<TestDomain> for PatchStrategy {
    type Intent = PatchIntent;

    fn coalesce(
        &self,
        previous: &Self::Intent,
        next: Self::Intent,
    ) -> Result<Self::Intent, DomainIssue> {
        if next.name.as_deref() == Some("reject") {
            return Err(DomainIssue::new("rejected", "coalescing rejected"));
        }
        Ok(PatchIntent {
            name: next.name.or_else(|| previous.name.clone()),
            enabled: next.enabled.or(previous.enabled),
        })
    }

    fn apply(&self, intent: &Self::Intent, value: &mut Settings) -> Result<(), DomainIssue> {
        self.applications.fetch_add(1, Ordering::SeqCst);
        if self.fail_once.swap(false, Ordering::SeqCst) {
            return Err(DomainIssue::new("injected", "injected apply failure"));
        }
        if let Some(name) = &intent.name {
            value.name.clone_from(name);
        }
        if let Some(enabled) = intent.enabled {
            value.enabled = enabled;
        }
        Ok(())
    }

    fn pending_weight(&self, intent: &Self::Intent) -> usize {
        intent.name.as_ref().map_or(0, String::len) + usize::from(intent.enabled.is_some())
    }
}

pub(super) fn fixture() -> (TempDir, ConfigStore) {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("config");
    let data = temp.path().join("data");
    for path in [
        &config,
        &data,
        &temp.path().join("cache"),
        &temp.path().join("runtime"),
        &temp.path().join("log"),
    ] {
        fs::create_dir_all(path).unwrap();
    }
    let roots = StorageRoots::new(
        &config,
        &data,
        temp.path().join("cache"),
        temp.path().join("runtime"),
        temp.path().join("log"),
    )
    .unwrap();
    let coordination = CoordinationAuthority::new(data).unwrap();
    (temp, ConfigStore::new(roots, coordination))
}

pub(super) fn policy(maximum: usize) -> DebouncePolicy {
    DebouncePolicy::new(
        Duration::from_millis(200),
        maximum,
        MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic),
    )
    .unwrap()
}

pub(super) fn target_path(temp: &TempDir, path: &str) -> PathBuf {
    temp.path().join("config").join(path)
}
