use longhorn_core::{DomainId, SchemaVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SerializedDocument {
    pub(super) domain: DomainId,
    pub(super) schema_version: SchemaVersion,
    pub(super) value: Value,
}

impl SerializedDocument {
    pub(super) fn new(domain: DomainId, schema_version: SchemaVersion, value: Value) -> Self {
        Self {
            domain,
            schema_version,
            value,
        }
    }
}
