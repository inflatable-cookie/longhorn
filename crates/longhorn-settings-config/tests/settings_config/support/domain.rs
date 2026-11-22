use longhorn_config::{
    ConfigDomain, DomainDescriptor, DomainFilePath, DomainIssue, MigrationStep, StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Preferences {
    pub(crate) theme: Option<String>,
    pub(crate) volume: Option<u8>,
    pub(crate) locked: Option<String>,
    pub(crate) hidden: Option<String>,
    pub(crate) unsupported: Option<String>,
    pub(crate) secret: String,
    pub(crate) untouched: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: None,
            volume: None,
            locked: Some("locked".into()),
            hidden: Some("hidden".into()),
            unsupported: Some("unsupported".into()),
            secret: "secret-authority".into(),
            untouched: "other-domain-shape".into(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct PreferencesDomain {
    descriptor: DomainDescriptor,
}

impl PreferencesDomain {
    pub(crate) fn new(id: &str, path: &str) -> Self {
        Self::with_storage(id, path, StorageClass::UserConfig)
    }

    pub(crate) fn with_storage(id: &str, path: &str, storage_class: StorageClass) -> Self {
        Self::with_version(id, path, storage_class, 1)
    }

    pub(crate) fn with_version(
        id: &str,
        path: &str,
        storage_class: StorageClass,
        version: u32,
    ) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(id).unwrap(),
                SchemaVersion::new(version).unwrap(),
                storage_class,
                Some(DomainFilePath::new(path).unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for PreferencesDomain {
    type Value = Preferences;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        Preferences::default()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        serde_json::from_value(value)
            .map_err(|error| DomainIssue::new("preferences-decode", error.to_string()))
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        serde_json::to_value(value)
            .map_err(|error| DomainIssue::new("preferences-encode", error.to_string()))
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.theme.as_deref() == Some("domain-refused") {
            return Err(DomainIssue::new("preferences-theme", "invalid theme"));
        }
        if value.volume.is_some_and(|volume| volume > 100) {
            return Err(DomainIssue::new(
                "preferences-volume",
                "volume must not exceed 100",
            ));
        }
        if value.secret.is_empty() || value.untouched.is_empty() {
            return Err(DomainIssue::new(
                "preferences-required",
                "secret and untouched fields are required",
            ));
        }
        Ok(())
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version.get() > self.descriptor.schema_version().get() {
            return Err(DomainIssue::new(
                "preferences-schema",
                "unsupported schema version",
            ));
        }
        let decoded = self.decode(value.clone())?;
        self.validate(&decoded)
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        let to = self.descriptor.schema_version();
        Ok((from.get() == 1 && to.get() == 2).then_some(MigrationStep {
            schema_version: to,
            value,
        }))
    }
}
