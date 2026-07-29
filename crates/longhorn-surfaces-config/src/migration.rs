use longhorn_config::{DomainIssue, MigrationStep};
use longhorn_core::SchemaVersion;
use longhorn_surfaces::{SurfaceDocument, SurfaceLimits};
use serde_json::Value;

use crate::PersistedSurfaceDocument;

/// Current Surface codec authority exposed to a consumer migration hook.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMigrationTarget {
    schema_version: SchemaVersion,
    limits: SurfaceLimits,
}

impl SurfaceMigrationTarget {
    pub(crate) const fn new(schema_version: SchemaVersion, limits: SurfaceLimits) -> Self {
        Self {
            schema_version,
            limits,
        }
    }

    /// Returns the current configuration-domain schema.
    #[must_use]
    pub const fn schema_version(self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the current Surface validation limits.
    #[must_use]
    pub const fn limits(self) -> SurfaceLimits {
        self.limits
    }

    /// Encodes a migrated document in the current raw shape.
    pub fn encode_current(self, document: SurfaceDocument) -> Result<Value, DomainIssue> {
        serde_json::to_value(PersistedSurfaceDocument::new(document))
            .map_err(|error| DomainIssue::new("surface-migration-encode", error.to_string()))
    }
}

/// Consumer-owned validation and one-step migration for older Surface schemas.
pub trait SurfaceMigration {
    /// Validates raw data at an older schema before migration.
    fn validate_raw(&self, schema_version: SchemaVersion, value: &Value)
    -> Result<(), DomainIssue>;

    /// Migrates one older schema or returns `None` when no step exists.
    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: SurfaceMigrationTarget,
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
        _target: SurfaceMigrationTarget,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}
