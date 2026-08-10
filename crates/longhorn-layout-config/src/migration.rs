use longhorn_config::{DomainIssue, MigrationStep};
use longhorn_core::SchemaVersion;
use longhorn_surfaces::{LayoutDefinitionRegistry, LayoutDocument};
use serde_json::Value;

use crate::{LayoutRegistryDigest, PersistedLayoutDocument};

/// Current registry and codec authority exposed to a consumer migration hook.
#[derive(Clone, Copy, Debug)]
pub struct LayoutMigrationTarget<'target> {
    schema_version: SchemaVersion,
    registry: &'target LayoutDefinitionRegistry,
    registry_digest: &'target LayoutRegistryDigest,
}

impl<'target> LayoutMigrationTarget<'target> {
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

    /// Encodes a migrated document with the current registry digest.
    pub fn encode_current(self, document: LayoutDocument) -> Result<Value, DomainIssue> {
        serde_json::to_value(PersistedLayoutDocument::new(
            self.registry_digest.clone(),
            document,
        ))
        .map_err(|error| DomainIssue::new("layout-migration-encode", error.to_string()))
    }
}

/// Consumer-owned validation and one-step migration for older layout schemas.
///
/// A registry change must also bump the domain schema. The migration reaches
/// the current schema only by emitting the target registry digest.
pub trait LayoutMigration {
    /// Validates raw data at an older schema before migration.
    fn validate_raw(&self, schema_version: SchemaVersion, value: &Value)
    -> Result<(), DomainIssue>;

    /// Migrates one older schema or returns `None` when no step exists.
    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: LayoutMigrationTarget<'_>,
    ) -> Result<Option<MigrationStep>, DomainIssue>;
}

/// Explicit hook that supplies no older-schema migration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoLayoutMigration;

impl LayoutMigration for NoLayoutMigration {
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
        _target: LayoutMigrationTarget<'_>,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}
