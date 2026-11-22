use longhorn_config::{DomainIssue, MigrationStep};
use longhorn_core::SchemaVersion;
use longhorn_surfaces::{
    LayoutDefinitionRegistry, SurfaceDocument, SurfaceLimits, normalize_document,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{LayoutRegistryDigest, PersistedSurfaceDocument};

/// Current registry and codec authority exposed to a consumer migration hook.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMigrationTarget<'target> {
    schema_version: SchemaVersion,
    registry: &'target LayoutDefinitionRegistry,
    registry_digest: &'target LayoutRegistryDigest,
}

impl<'target> SurfaceMigrationTarget<'target> {
    pub(crate) const fn new(
        schema_version: SchemaVersion,
        registry: &'target LayoutDefinitionRegistry,
        registry_digest: &'target LayoutRegistryDigest,
    ) -> Self {
        Self {
            schema_version,
            registry,
            registry_digest,
        }
    }

    /// Returns the current configuration-domain schema.
    #[must_use]
    pub const fn schema_version(self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the current validated layout definition registry.
    #[must_use]
    pub const fn registry(self) -> &'target LayoutDefinitionRegistry {
        self.registry
    }

    /// Returns the current deterministic registry digest.
    #[must_use]
    pub const fn registry_digest(self) -> &'target LayoutRegistryDigest {
        self.registry_digest
    }

    /// Returns the current Surface validation limits.
    #[must_use]
    pub const fn limits(self) -> SurfaceLimits {
        self.registry.limits()
    }

    /// Encodes a migrated document in the current raw shape with the current registry digest.
    pub fn encode_current(self, document: SurfaceDocument) -> Result<Value, DomainIssue> {
        serde_json::to_value(PersistedSurfaceDocument::new(
            self.registry_digest.clone(),
            document,
        ))
        .map_err(|error| DomainIssue::new("surface-migration-encode", error.to_string()))
    }
}

/// Consumer-owned validation and one-step migration for older Surface schemas.
///
/// A registry change must also bump the domain schema. The migration reaches
/// the current schema only by emitting the target registry digest.
pub trait SurfaceMigration {
    /// Validates raw data at an older schema before migration.
    fn validate_raw(&self, schema_version: SchemaVersion, value: &Value)
    -> Result<(), DomainIssue>;

    /// Migrates one older schema or returns `None` when no step exists.
    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: SurfaceMigrationTarget<'_>,
    ) -> Result<Option<MigrationStep>, DomainIssue>;
}

/// Explicit hook that supplies no older-schema migration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSurfaceMigration;

impl SurfaceMigration for NoSurfaceMigration {
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
        _target: SurfaceMigrationTarget<'_>,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

/// Built-in one-step migration from pre-unification persisted Surface/layout files.
///
/// Use [`Self::digestless`] for the former digest-less Surface wrapper
/// (`{"document": <SurfaceDocument>}`) or [`Self::with_expected_digest`] for
/// the former layout wrapper (`{"registryDigest": <digest>, "document": <SurfaceDocument>}`).
///
/// Digest-bearing migrations verify that the stored `registryDigest` equals
/// the expected prior-schema digest before re-encoding with the current
/// registry digest; any missing or mismatched source digest fails closed into
/// recovery. Digest-less migrations reject files that carry a `registryDigest`.
///
/// The inner [`SurfaceDocument`] is normalized and validated against the current
/// [`LayoutDefinitionRegistry`] before re-encoding. Documents that fail
/// whole-document validation (for example, when a schema change requires a
/// consumer product decision) return a typed [`DomainIssue`] so the store
/// enters recovery and leaves the original bytes untouched on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacySurfaceFormatMigration {
    from_version: SchemaVersion,
    expected_registry_digest: Option<LayoutRegistryDigest>,
}

impl LegacySurfaceFormatMigration {
    /// Constructs a migration from a pre-unification digest-less Surface file
    /// (`{"document": <SurfaceDocument>}`) at `from_version`.
    #[must_use]
    pub const fn digestless(from_version: SchemaVersion) -> Self {
        Self {
            from_version,
            expected_registry_digest: None,
        }
    }

    /// Constructs a migration from a pre-unification digest-bearing layout file
    /// (`{"registryDigest": <digest>, "document": <SurfaceDocument>}`) at
    /// `from_version`, requiring the stored digest to match `expected_registry_digest`.
    #[must_use]
    pub const fn with_expected_digest(
        from_version: SchemaVersion,
        expected_registry_digest: LayoutRegistryDigest,
    ) -> Self {
        Self {
            from_version,
            expected_registry_digest: Some(expected_registry_digest),
        }
    }

    /// Returns the legacy schema version accepted by this migration.
    #[must_use]
    pub const fn from_version(&self) -> SchemaVersion {
        self.from_version
    }

    /// Returns the expected legacy registry digest, or `None` for digest-less files.
    #[must_use]
    pub const fn expected_registry_digest(&self) -> Option<&LayoutRegistryDigest> {
        self.expected_registry_digest.as_ref()
    }

    fn decode_and_verify(&self, value: Value) -> Result<SurfaceDocument, DomainIssue> {
        match &self.expected_registry_digest {
            None => {
                let legacy: LegacyDigestlessSurfaceDocument = serde_json::from_value(value)
                    .map_err(|error| {
                        DomainIssue::new("legacy-surface-raw-shape", error.to_string())
                    })?;
                Ok(legacy.document)
            }
            Some(expected) => {
                let legacy: PersistedSurfaceDocument =
                    serde_json::from_value(value).map_err(|error| {
                        DomainIssue::new("legacy-surface-raw-shape", error.to_string())
                    })?;
                if legacy.registry_digest() != expected {
                    return Err(DomainIssue::new(
                        "legacy-surface-registry-mismatch",
                        format!(
                            "stored legacy registry digest {} does not match expected digest {}",
                            legacy.registry_digest().as_str(),
                            expected.as_str()
                        ),
                    ));
                }
                Ok(legacy.document().clone())
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyDigestlessSurfaceDocument {
    document: SurfaceDocument,
}

impl SurfaceMigration for LegacySurfaceFormatMigration {
    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version != self.from_version {
            return Err(DomainIssue::new(
                "legacy-surface-schema",
                format!(
                    "expected legacy schema version {}, found {}",
                    self.from_version.get(),
                    schema_version.get()
                ),
            ));
        }
        self.decode_and_verify(value.clone()).map(|_| ())
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: SurfaceMigrationTarget<'_>,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        if from != self.from_version {
            return Ok(None);
        }
        let document = self.decode_and_verify(value)?;
        let normalized = normalize_document(target.registry(), &document).map_err(|error| {
            DomainIssue::new(
                format!("surface-validation-{:?}", error.code()).to_ascii_lowercase(),
                error.detail(),
            )
        })?;
        Ok(Some(MigrationStep {
            schema_version: target.schema_version(),
            value: target.encode_current(normalized)?,
        }))
    }
}
