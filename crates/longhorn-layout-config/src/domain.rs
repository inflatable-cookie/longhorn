use std::{error::Error, fmt};

use longhorn_config::{
    BackupCatalog, BackupCatalogError, BackupExclusionReason, ConfigDomain, DomainDescriptor,
    DomainIssue, MigrationStep,
};
use longhorn_core::SchemaVersion;
use longhorn_surfaces::{
    LayoutDefinitionRegistry, LayoutDocument, LayoutValidationCode,
    normalize_layout_document as normalize_document, validate_layout_document as validate_document,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    LayoutMigration, LayoutMigrationTarget, LayoutRegistryDigest, compute_layout_registry_digest,
};

/// Current raw value stored inside the generic configuration envelope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersistedLayoutDocument {
    registry_digest: LayoutRegistryDigest,
    document: LayoutDocument,
}

impl PersistedLayoutDocument {
    /// Constructs a current raw layout value.
    #[must_use]
    pub const fn new(registry_digest: LayoutRegistryDigest, document: LayoutDocument) -> Self {
        Self {
            registry_digest,
            document,
        }
    }

    /// Returns the definition registry digest that interprets the document.
    #[must_use]
    pub const fn registry_digest(&self) -> &LayoutRegistryDigest {
        &self.registry_digest
    }

    /// Returns the complete authoritative layout document.
    #[must_use]
    pub const fn document(&self) -> &LayoutDocument {
        &self.document
    }
}

/// Explicit ordinary-file backup participation for one layout domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutBackupPolicy {
    /// Include the registered layout domain in selected backups.
    Include,
    /// Preserve an explicit exclusion in backup evidence.
    Exclude(BackupExclusionReason),
}

/// One consumer-scoped authoritative layout configuration domain.
#[derive(Clone, Debug)]
pub struct RegisteredLayoutDomain<M> {
    descriptor: DomainDescriptor,
    default: LayoutDocument,
    registry: LayoutDefinitionRegistry,
    registry_digest: LayoutRegistryDigest,
    migration: M,
    backup_policy: LayoutBackupPolicy,
}

impl<M> RegisteredLayoutDomain<M>
where
    M: LayoutMigration,
{
    /// Constructs a domain from complete consumer-supplied authority.
    pub fn new(
        descriptor: DomainDescriptor,
        default: LayoutDocument,
        registry: LayoutDefinitionRegistry,
        migration: M,
        backup_policy: LayoutBackupPolicy,
    ) -> Result<Self, RegisteredLayoutDomainError> {
        let default = normalize_document(&registry, &default).map_err(|error| {
            RegisteredLayoutDomainError::InvalidDefault {
                code: error.code(),
                detail: error.detail().to_owned(),
            }
        })?;
        let registry_digest = compute_layout_registry_digest(&registry).map_err(|error| {
            RegisteredLayoutDomainError::RegistryEncoding {
                detail: error.to_string(),
            }
        })?;

        Ok(Self {
            descriptor,
            default,
            registry,
            registry_digest,
            migration,
            backup_policy,
        })
    }

    /// Returns the exact injected configuration descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    /// Returns the validated immutable definition registry.
    #[must_use]
    pub const fn registry(&self) -> &LayoutDefinitionRegistry {
        &self.registry
    }

    /// Returns the deterministic current registry digest.
    #[must_use]
    pub const fn registry_digest(&self) -> &LayoutRegistryDigest {
        &self.registry_digest
    }

    /// Returns the explicit backup participation policy.
    #[must_use]
    pub const fn backup_policy(&self) -> &LayoutBackupPolicy {
        &self.backup_policy
    }

    /// Adds this exact descriptor and policy to a backup catalogue.
    pub fn add_to_backup_catalog<'domain>(
        &'domain self,
        catalog: &mut BackupCatalog<'domain>,
    ) -> Result<(), BackupCatalogError> {
        match &self.backup_policy {
            LayoutBackupPolicy::Include => catalog.include(self),
            LayoutBackupPolicy::Exclude(reason) => catalog.exclude(self, reason.clone()),
        }
    }

    fn migration_target(&self) -> LayoutMigrationTarget<'_> {
        LayoutMigrationTarget::new(
            self.descriptor.schema_version(),
            &self.registry,
            &self.registry_digest,
        )
    }

    fn validate_persisted(&self, value: &PersistedLayoutDocument) -> Result<(), DomainIssue> {
        if value.registry_digest != self.registry_digest {
            return Err(DomainIssue::new(
                "layout-registry-mismatch",
                format!(
                    "stored registry digest {} does not match current digest {}",
                    value.registry_digest.as_str(),
                    self.registry_digest.as_str()
                ),
            ));
        }
        validate_document(&self.registry, &value.document).map_err(layout_validation_issue)
    }
}

impl<M> ConfigDomain for RegisteredLayoutDomain<M>
where
    M: LayoutMigration,
{
    type Value = LayoutDocument;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        self.default.clone()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        let persisted: PersistedLayoutDocument = serde_json::from_value(value)
            .map_err(|error| DomainIssue::new("layout-decode", error.to_string()))?;
        self.validate_persisted(&persisted)?;
        Ok(persisted.document)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        validate_document(&self.registry, value).map_err(layout_validation_issue)?;
        serde_json::to_value(PersistedLayoutDocument::new(
            self.registry_digest.clone(),
            value.clone(),
        ))
        .map_err(|error| DomainIssue::new("layout-encode", error.to_string()))
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        validate_document(&self.registry, value).map_err(layout_validation_issue)
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version == self.descriptor.schema_version() {
            let persisted: PersistedLayoutDocument = serde_json::from_value(value.clone())
                .map_err(|error| DomainIssue::new("layout-raw-shape", error.to_string()))?;
            self.validate_persisted(&persisted)
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

fn layout_validation_issue(error: longhorn_surfaces::LayoutValidationError) -> DomainIssue {
    DomainIssue::new(
        format!("layout-validation-{:?}", error.code()).to_ascii_lowercase(),
        error.detail(),
    )
}

/// Invalid registered layout domain construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegisteredLayoutDomainError {
    /// The compiled default could not be normalized and validated.
    InvalidDefault {
        /// Stable validation category.
        code: LayoutValidationCode,
        /// Validation detail.
        detail: String,
    },
    /// Canonical registry JSON could not be encoded for hashing.
    RegistryEncoding {
        /// Serializer detail.
        detail: String,
    },
}

impl fmt::Display for RegisteredLayoutDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDefault { code, detail } => {
                write!(formatter, "invalid layout default ({code:?}): {detail}")
            }
            Self::RegistryEncoding { detail } => {
                write!(formatter, "cannot encode layout registry: {detail}")
            }
        }
    }
}

impl Error for RegisteredLayoutDomainError {}
