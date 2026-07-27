use std::{fs, path::PathBuf};

use longhorn_config::{
    ConfigDomain, ConfigStore, DomainDescriptor, DomainFilePath, DomainIssue, DomainLocation,
    MigrationStep, StorageClass, StorageRoots,
};
use longhorn_core::{DomainId, SchemaVersion};
use serde::Deserialize;
use serde_json::{Value, json};
use tempfile::TempDir;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Preferences {
    pub(crate) name: String,
    pub(crate) enabled: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum MigrationBehavior {
    Complete,
    MissingSecond,
    WrongTarget,
    FailSecond,
}

pub(crate) struct PreferencesDomain {
    descriptor: DomainDescriptor,
    behavior: MigrationBehavior,
    invalid_default: bool,
}

impl PreferencesDomain {
    pub(crate) fn new(
        id: &str,
        class: StorageClass,
        path: Option<&str>,
        current_version: u32,
    ) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(id).unwrap(),
                SchemaVersion::new(current_version).unwrap(),
                class,
                path.map(|path| DomainFilePath::new(path).unwrap()),
            )
            .unwrap(),
            behavior: MigrationBehavior::Complete,
            invalid_default: false,
        }
    }

    pub(crate) fn with_behavior(mut self, behavior: MigrationBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    pub(crate) fn with_invalid_default(mut self) -> Self {
        self.invalid_default = true;
        self
    }
}

impl ConfigDomain for PreferencesDomain {
    type Value = Preferences;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        Preferences {
            name: if self.invalid_default {
                String::new()
            } else {
                "default".to_owned()
            },
            enabled: true,
        }
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        serde_json::from_value(value).map_err(|error| DomainIssue::new("decode", error.to_string()))
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
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        let object = value
            .as_object()
            .ok_or_else(|| DomainIssue::new("shape", "value must be an object"))?;

        let valid = match schema_version.get() {
            1 => object.get("label").is_some_and(Value::is_string),
            2 => object.get("name").is_some_and(Value::is_string),
            3 => {
                object.get("name").is_some_and(Value::is_string)
                    && object.get("enabled").is_some_and(Value::is_boolean)
            }
            _ => false,
        };

        if valid {
            Ok(())
        } else {
            Err(DomainIssue::new(
                "schema-shape",
                format!("invalid schema {schema_version} value"),
            ))
        }
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        mut value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        if from.get() == 2 && matches!(self.behavior, MigrationBehavior::MissingSecond) {
            return Ok(None);
        }
        if from.get() == 2 && matches!(self.behavior, MigrationBehavior::FailSecond) {
            return Err(DomainIssue::new(
                "migration-failed",
                "schema 2 migration failed",
            ));
        }

        let object = value
            .as_object_mut()
            .ok_or_else(|| DomainIssue::new("shape", "value must be an object"))?;

        let target = match from.get() {
            1 => {
                let label = object
                    .remove("label")
                    .ok_or_else(|| DomainIssue::new("label", "label is required"))?;
                object.insert("name".to_owned(), label);
                2
            }
            2 => {
                object.insert("enabled".to_owned(), Value::Bool(true));
                if matches!(self.behavior, MigrationBehavior::WrongTarget) {
                    2
                } else {
                    3
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(MigrationStep {
            schema_version: SchemaVersion::new(target).unwrap(),
            value,
        }))
    }
}

pub(crate) struct Fixture {
    pub(crate) temp: TempDir,
    pub(crate) roots: StorageRoots,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let config = root.join("config");
        let data = root.join("data");
        let cache = root.join("cache");
        let runtime = root.join("runtime");
        let log = root.join("log");
        let policy = root.join("policy");
        let workspace = root.join("workspace");
        let project = root.join("project");

        for path in [
            &config, &data, &cache, &runtime, &log, &policy, &workspace, &project,
        ] {
            fs::create_dir_all(path).unwrap();
        }

        let roots = StorageRoots::new(config, data, cache, runtime, log)
            .unwrap()
            .with_policy(policy)
            .unwrap()
            .with_workspace(workspace)
            .unwrap()
            .with_project(project)
            .unwrap();

        Self { temp, roots }
    }

    pub(crate) fn store(&self) -> ConfigStore {
        ConfigStore::new(self.roots.clone())
    }

    pub(crate) fn path_for(&self, domain: &PreferencesDomain) -> PathBuf {
        match self.roots.resolve(domain.descriptor()) {
            DomainLocation::File(file) => file.full_path().to_path_buf(),
            location => panic!("expected file location, found {location:?}"),
        }
    }

    pub(crate) fn write(&self, domain: &PreferencesDomain, bytes: &[u8]) -> PathBuf {
        let path = self.path_for(domain);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
}

pub(crate) fn config_domain() -> PreferencesDomain {
    PreferencesDomain::new(
        "example.preferences",
        StorageClass::UserConfig,
        Some("example/preferences.json"),
        3,
    )
}

pub(crate) fn document(domain: &str, schema_version: u32, value: Value) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "domain": domain,
        "schemaVersion": schema_version,
        "value": value,
    }))
    .unwrap()
}
