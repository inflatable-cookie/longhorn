use longhorn_config::Sha256Digest;
use longhorn_surfaces::{
    LayoutDefinitionRegistry, LayoutLimits, LayoutSchemaDefinition, PanelDefinition,
};
use serde::{Deserialize, Serialize};

/// Deterministic digest of one complete validated layout definition registry.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct LayoutRegistryDigest(Sha256Digest);

impl LayoutRegistryDigest {
    /// Returns the lowercase hexadecimal SHA-256 value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryDigestMaterial<'registry> {
    limits: LayoutLimits,
    schemas: Vec<&'registry LayoutSchemaDefinition>,
    panel_definitions: Vec<&'registry PanelDefinition>,
}

/// Computes a canonical digest over limits, schemas, and panel definitions.
///
/// Registry construction has already canonicalized schema and definition
/// order, so equivalent insertion orders produce the same bytes.
pub fn compute_layout_registry_digest(
    registry: &LayoutDefinitionRegistry,
) -> Result<LayoutRegistryDigest, serde_json::Error> {
    let material = RegistryDigestMaterial {
        limits: registry.limits(),
        schemas: registry.schemas().collect(),
        panel_definitions: registry.panel_definitions().collect(),
    };
    serde_json::to_vec(&material)
        .map(|bytes| LayoutRegistryDigest(Sha256Digest::from_bytes(bytes.as_slice())))
}
