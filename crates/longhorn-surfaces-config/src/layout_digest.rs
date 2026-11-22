use longhorn_config::{Sha256Digest, Sha256DigestError};
use longhorn_surfaces::{
    LayoutDefinitionRegistry, LayoutSchemaDefinition, PanelDefinition, SurfaceLimits,
};
use serde::{Deserialize, Serialize};

/// Deterministic digest of one complete validated layout definition registry.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct LayoutRegistryDigest(Sha256Digest);

impl LayoutRegistryDigest {
    /// Validates an existing lowercase hexadecimal SHA-256 registry digest.
    ///
    /// # Errors
    ///
    /// Returns [`Sha256DigestError`] if `value` is not a 64-character lowercase
    /// hexadecimal SHA-256 digest.
    pub fn new(value: impl Into<String>) -> Result<Self, Sha256DigestError> {
        Sha256Digest::new(value).map(Self)
    }

    /// Wraps a verified [`Sha256Digest`] as a registry digest.
    #[must_use]
    pub const fn from_sha256(digest: Sha256Digest) -> Self {
        Self(digest)
    }

    /// Returns the lowercase hexadecimal SHA-256 value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryDigestMaterial<'registry> {
    limits: SurfaceLimits,
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
