use longhorn_core::{DomainId, SchemaVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SerializedDocument {
    pub(crate) domain: DomainId,
    pub(crate) schema_version: SchemaVersion,
    pub(crate) value: Value,
}

impl SerializedDocument {
    pub(crate) fn new(domain: DomainId, schema_version: SchemaVersion, value: Value) -> Self {
        Self {
            domain,
            schema_version,
            value,
        }
    }
}
