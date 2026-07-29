use std::{error::Error, fmt};

use longhorn_config::{
    BackupCatalog, BackupCatalogError, BackupExclusionReason, ConfigDomain, DomainDescriptor,
    DomainIssue, MigrationStep,
};
use longhorn_core::SchemaVersion;
use longhorn_surfaces::{
    SurfaceDocument, SurfaceLimits, SurfaceValidationCode, normalize_document, validate_document,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{SurfaceMigration, SurfaceMigrationTarget};

/// Current raw value stored inside the generic configuration envelope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersistedSurfaceDocument {
    document: SurfaceDocument,
}

impl PersistedSurfaceDocument {
    /// Constructs a current raw Surface value.
    #[must_use]
    pub const fn new(document: SurfaceDocument) -> Self {
        Self { document }
    }

    /// Returns the complete authoritative Surface document.
    #[must_use]
    pub const fn document(&self) -> &SurfaceDocument {
        &self.document
    }
}

/// Explicit ordinary-file backup participation for one Surface domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceBackupPolicy {
    /// Include the registered Surface domain in selected backups.
    Include,
    /// Preserve an explicit exclusion in backup evidence.
    Exclude(BackupExclusionReason),
}

/// One consumer-scoped authoritative Surface configuration domain.
#[derive(Clone, Debug)]
pub struct RegisteredSurfaceDomain<M> {
    descriptor: DomainDescriptor,
    default: SurfaceDocument,
    limits: SurfaceLimits,
    migration: M,
    backup_policy: SurfaceBackupPolicy,
}

impl<M> RegisteredSurfaceDomain<M>
where
    M: SurfaceMigration,
{
    /// Constructs a domain from complete consumer-supplied authority.
    pub fn new(
        descriptor: DomainDescriptor,
        default: SurfaceDocument,
        limits: SurfaceLimits,
        migration: M,
        backup_policy: SurfaceBackupPolicy,
    ) -> Result<Self, RegisteredSurfaceDomainError> {
        let default = normalize_document(limits, &default).map_err(|error| {
            RegisteredSurfaceDomainError::InvalidDefault {
                code: error.code(),
                detail: error.detail().to_owned(),
            }
        })?;
        Ok(Self {
            descriptor,
            default,
            limits,
            migration,
            backup_policy,
        })
    }

    /// Returns the exact injected configuration descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    /// Returns the configured Surface validation limits.
    #[must_use]
    pub const fn limits(&self) -> SurfaceLimits {
        self.limits
    }

    /// Returns the explicit backup participation policy.
    #[must_use]
    pub const fn backup_policy(&self) -> &SurfaceBackupPolicy {
        &self.backup_policy
    }

    /// Adds this exact descriptor and policy to a backup catalogue.
    pub fn add_to_backup_catalog<'domain>(
        &'domain self,
        catalog: &mut BackupCatalog<'domain>,
    ) -> Result<(), BackupCatalogError> {
        match &self.backup_policy {
            SurfaceBackupPolicy::Include => catalog.include(self),
            SurfaceBackupPolicy::Exclude(reason) => catalog.exclude(self, reason.clone()),
        }
    }

    fn migration_target(&self) -> SurfaceMigrationTarget {
        SurfaceMigrationTarget::new(self.descriptor.schema_version(), self.limits)
    }
}

impl<M> ConfigDomain for RegisteredSurfaceDomain<M>
where
    M: SurfaceMigration,
{
    type Value = SurfaceDocument;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        self.default.clone()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        let persisted: PersistedSurfaceDocument = serde_json::from_value(value)
            .map_err(|error| DomainIssue::new("surface-decode", error.to_string()))?;
        validate_document(self.limits, &persisted.document).map_err(surface_validation_issue)?;
        Ok(persisted.document)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        validate_document(self.limits, value).map_err(surface_validation_issue)?;
        serde_json::to_value(PersistedSurfaceDocument::new(value.clone()))
            .map_err(|error| DomainIssue::new("surface-encode", error.to_string()))
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        validate_document(self.limits, value).map_err(surface_validation_issue)
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version == self.descriptor.schema_version() {
            let persisted: PersistedSurfaceDocument = serde_json::from_value(value.clone())
                .map_err(|error| DomainIssue::new("surface-raw-shape", error.to_string()))?;
            validate_document(self.limits, &persisted.document).map_err(surface_validation_issue)
        } else {
            self.migration.validate_raw(schema_version, value)
        }
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        self.migration
            .migrate_one(from, value, self.migration_target())
    }
}

fn surface_validation_issue(error: longhorn_surfaces::SurfaceValidationError) -> DomainIssue {
    DomainIssue::new(
        format!("surface-validation-{:?}", error.code()).to_ascii_lowercase(),
        error.detail(),
    )
}

/// Invalid registered Surface domain construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegisteredSurfaceDomainError {
    /// The compiled default could not be normalized and validated.
    InvalidDefault {
        /// Stable validation category.
        code: SurfaceValidationCode,
        /// Validation detail.
        detail: String,
    },
}

impl fmt::Display for RegisteredSurfaceDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDefault { code, detail } => {
                write!(formatter, "invalid Surface default ({code:?}): {detail}")
            }
        }
    }
}

impl Error for RegisteredSurfaceDomainError {}
