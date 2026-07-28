use std::{collections::BTreeMap, error::Error, fmt};

use longhorn_core::DomainId;

use crate::{ConfigDomain, DomainDescriptor, StorageClass};

use super::BackupAdapter;

const MAX_POLICY_TEXT_BYTES: usize = 256;

/// Stable reason for excluding one selected domain from a backup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupExclusionReason(String);

impl BackupExclusionReason {
    /// Validates a stable machine-readable exclusion reason.
    pub fn new(value: impl Into<String>) -> Result<Self, BackupCatalogError> {
        let value = value.into();
        validate_policy_text("exclusion reason", &value)?;
        Ok(Self(value))
    }

    /// Returns the serialized exclusion reason.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn default_for(storage_class: StorageClass) -> Option<Self> {
        matches!(
            storage_class,
            StorageClass::Secret | StorageClass::Cache | StorageClass::Runtime | StorageClass::Log
        )
        .then(|| {
            Self(format!(
                "default-{}-exclusion",
                storage_class_name(storage_class)
            ))
        })
    }
}

/// Stable identifier for a consumer-owned custom backup adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterId(String);

impl BackupAdapterId {
    /// Validates a custom adapter identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, BackupCatalogError> {
        let value = value.into();
        validate_policy_text("adapter id", &value)?;
        if !value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_' | '-')
        }) {
            return Err(BackupCatalogError::InvalidText {
                field: "adapter id",
                detail: "must contain lowercase ASCII letters, digits, '.', '_', or '-'".into(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the serialized adapter identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Backup participation policy for one registered domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupPolicy {
    /// Capture the ordinary file-backed domain.
    Include,
    /// Record an explicit exclusion.
    Exclude(BackupExclusionReason),
    /// Delegate capture to a named consumer adapter.
    Custom(BackupAdapterId),
}

/// Explicit backup policy catalogue for registered domains.
///
/// Included entries retain a borrowed typed domain only for validation during
/// the coordinated capture call. The catalogue does not own product schemas.
pub struct BackupCatalog<'domains> {
    entries: BTreeMap<DomainId, CatalogEntry<'domains>>,
}

impl<'domains> BackupCatalog<'domains> {
    /// Constructs an empty policy catalogue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Includes an ordinary file-backed domain.
    pub fn include<D: ConfigDomain>(
        &mut self,
        domain: &'domains D,
    ) -> Result<(), BackupCatalogError> {
        self.insert(domain, BackupPolicy::Include)
    }

    /// Excludes a domain with a stable reason.
    pub fn exclude<D: ConfigDomain>(
        &mut self,
        domain: &'domains D,
        reason: BackupExclusionReason,
    ) -> Result<(), BackupCatalogError> {
        self.insert(domain, BackupPolicy::Exclude(reason))
    }

    /// Delegates a domain to a named custom adapter.
    pub fn custom<D: ConfigDomain>(
        &mut self,
        domain: &'domains D,
        adapter: &'domains dyn BackupAdapter,
    ) -> Result<(), BackupCatalogError> {
        self.insert_custom(domain, adapter)
    }

    fn insert<D: ConfigDomain>(
        &mut self,
        domain: &'domains D,
        policy: BackupPolicy,
    ) -> Result<(), BackupCatalogError> {
        let id = domain.descriptor().id().clone();
        if self.entries.contains_key(&id) {
            return Err(BackupCatalogError::DuplicateDomain { id });
        }
        self.entries.insert(
            id,
            CatalogEntry {
                descriptor: domain.descriptor().clone(),
                policy,
                domain,
                adapter: None,
            },
        );
        Ok(())
    }

    fn insert_custom<D: ConfigDomain>(
        &mut self,
        domain: &'domains D,
        adapter: &'domains dyn BackupAdapter,
    ) -> Result<(), BackupCatalogError> {
        let id = domain.descriptor().id().clone();
        if self.entries.contains_key(&id) {
            return Err(BackupCatalogError::DuplicateDomain { id });
        }
        self.entries.insert(
            id,
            CatalogEntry {
                descriptor: domain.descriptor().clone(),
                policy: BackupPolicy::Custom(adapter.id().clone()),
                domain,
                adapter: Some(adapter),
            },
        );
        Ok(())
    }

    pub(crate) fn decision(&self, descriptor: &DomainDescriptor) -> Option<CatalogDecision<'_>> {
        self.entries.get(descriptor.id()).map(|entry| {
            if entry.descriptor != *descriptor {
                CatalogDecision::DescriptorChanged
            } else {
                match &entry.policy {
                    BackupPolicy::Include => CatalogDecision::Include(entry.domain),
                    BackupPolicy::Exclude(reason) => CatalogDecision::Exclude(reason.clone()),
                    BackupPolicy::Custom(_) => CatalogDecision::Custom(
                        entry
                            .adapter
                            .expect("custom catalogue policy retains its adapter"),
                    ),
                }
            }
        })
    }

    pub(crate) fn descriptors(&self) -> impl Iterator<Item = &DomainDescriptor> {
        self.entries.values().map(|entry| &entry.descriptor)
    }
}

impl Default for BackupCatalog<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for BackupCatalog<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_map()
            .entries(self.entries.iter().map(|(id, entry)| (id, &entry.policy)))
            .finish()
    }
}

struct CatalogEntry<'domains> {
    descriptor: DomainDescriptor,
    policy: BackupPolicy,
    domain: &'domains dyn ErasedBackupDomain,
    adapter: Option<&'domains dyn BackupAdapter>,
}

pub(crate) enum CatalogDecision<'domains> {
    Include(&'domains dyn ErasedBackupDomain),
    Exclude(BackupExclusionReason),
    Custom(&'domains dyn BackupAdapter),
    DescriptorChanged,
}

pub(crate) trait ErasedBackupDomain {
    fn capture_source(
        &self,
        file: &crate::ResolvedFile,
        max_bytes: usize,
    ) -> Result<super::capture::CapturedSource, super::capture::SourceCaptureError>;

    fn prepare_restore_source(
        &self,
        bytes: &[u8],
        path: &std::path::Path,
    ) -> Result<super::restore::PreparedRestoreSource, super::restore::PrepareSourceError>;
}

impl<D: ConfigDomain> ErasedBackupDomain for D {
    fn capture_source(
        &self,
        file: &crate::ResolvedFile,
        max_bytes: usize,
    ) -> Result<super::capture::CapturedSource, super::capture::SourceCaptureError> {
        super::capture::capture_typed_source(self, file, max_bytes)
    }

    fn prepare_restore_source(
        &self,
        bytes: &[u8],
        path: &std::path::Path,
    ) -> Result<super::restore::PreparedRestoreSource, super::restore::PrepareSourceError> {
        super::restore::prepare_typed_source(self, bytes, path)
    }
}

/// Invalid backup catalogue declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupCatalogError {
    /// A domain received more than one policy decision.
    DuplicateDomain {
        /// Duplicate domain id.
        id: DomainId,
    },
    /// A bounded policy string was empty, too long, or malformed.
    InvalidText {
        /// Invalid field.
        field: &'static str,
        /// Validation detail.
        detail: String,
    },
}

impl fmt::Display for BackupCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDomain { id } => {
                write!(
                    formatter,
                    "backup policy for domain {id} is already declared"
                )
            }
            Self::InvalidText { field, detail } => {
                write!(formatter, "invalid backup {field}: {detail}")
            }
        }
    }
}

impl Error for BackupCatalogError {}

fn validate_policy_text(field: &'static str, value: &str) -> Result<(), BackupCatalogError> {
    if value.is_empty() {
        return Err(BackupCatalogError::InvalidText {
            field,
            detail: "cannot be empty".into(),
        });
    }
    if value.len() > MAX_POLICY_TEXT_BYTES {
        return Err(BackupCatalogError::InvalidText {
            field,
            detail: format!("cannot exceed {MAX_POLICY_TEXT_BYTES} bytes"),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(BackupCatalogError::InvalidText {
            field,
            detail: "cannot contain control characters".into(),
        });
    }
    Ok(())
}

const fn storage_class_name(storage_class: StorageClass) -> &'static str {
    match storage_class {
        StorageClass::Secret => "secret",
        StorageClass::Cache => "cache",
        StorageClass::Runtime => "runtime",
        StorageClass::Log => "log",
        StorageClass::Defaults
        | StorageClass::Policy
        | StorageClass::UserConfig
        | StorageClass::MachineState
        | StorageClass::WorkspaceLocal
        | StorageClass::ProjectShared => "domain",
    }
}
