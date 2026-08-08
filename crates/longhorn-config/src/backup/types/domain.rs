use longhorn_core::{DomainId, SchemaVersion};
use serde::{Deserialize, Serialize};

use crate::StorageClass;

use super::{
    consistency::ORDINARY_CONSISTENCY_GROUP, identity::deserialize_metadata, BackupPayloadManifest,
    BackupSourceIssue, BackupSourceState,
};

const ORDINARY_ADAPTER: &str = "longhorn-json-v1";

/// Manifest evidence for one included ordinary or custom domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifestDomain {
    domain: DomainId,
    storage_class: StorageClass,
    #[serde(deserialize_with = "deserialize_metadata")]
    consistency_group: String,
    #[serde(deserialize_with = "deserialize_metadata")]
    adapter: String,
    state: BackupSourceState,
    source_schema_version: Option<SchemaVersion>,
    source_issue: Option<BackupSourceIssue>,
    payloads: Vec<BackupPayloadManifest>,
}

impl BackupManifestDomain {
    /// Returns the registered domain id.
    #[must_use]
    pub fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the domain storage class.
    #[must_use]
    pub const fn storage_class(&self) -> StorageClass {
        self.storage_class
    }

    /// Returns the manifest consistency group id.
    #[must_use]
    pub fn consistency_group(&self) -> &str {
        &self.consistency_group
    }

    /// Returns the capture adapter id.
    #[must_use]
    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    /// Returns the persisted source state.
    #[must_use]
    pub const fn state(&self) -> BackupSourceState {
        self.state
    }

    /// Returns the source envelope schema when readable.
    #[must_use]
    pub const fn source_schema_version(&self) -> Option<SchemaVersion> {
        self.source_schema_version
    }

    /// Returns why readable source is non-restorable.
    #[must_use]
    pub const fn source_issue(&self) -> Option<BackupSourceIssue> {
        self.source_issue
    }

    /// Returns ordered exact payload evidence.
    #[must_use]
    pub fn payloads(&self) -> &[BackupPayloadManifest] {
        &self.payloads
    }

    pub(crate) fn absent(descriptor: &crate::DomainDescriptor) -> Self {
        Self {
            domain: descriptor.id().clone(),
            storage_class: descriptor.storage_class(),
            consistency_group: ORDINARY_CONSISTENCY_GROUP.into(),
            adapter: ORDINARY_ADAPTER.into(),
            state: BackupSourceState::Absent,
            source_schema_version: None,
            source_issue: None,
            payloads: Vec::new(),
        }
    }

    pub(crate) fn with_source(
        descriptor: &crate::DomainDescriptor,
        state: BackupSourceState,
        source_schema_version: Option<SchemaVersion>,
        source_issue: Option<BackupSourceIssue>,
        payload: BackupPayloadManifest,
    ) -> Self {
        Self {
            domain: descriptor.id().clone(),
            storage_class: descriptor.storage_class(),
            consistency_group: ORDINARY_CONSISTENCY_GROUP.into(),
            adapter: ORDINARY_ADAPTER.into(),
            state,
            source_schema_version,
            source_issue,
            payloads: vec![payload],
        }
    }

    pub(crate) fn custom_absent(
        descriptor: &crate::DomainDescriptor,
        consistency_group: &str,
        adapter: &crate::BackupAdapterId,
    ) -> Self {
        Self {
            domain: descriptor.id().clone(),
            storage_class: descriptor.storage_class(),
            consistency_group: consistency_group.into(),
            adapter: adapter.as_str().into(),
            state: BackupSourceState::Absent,
            source_schema_version: None,
            source_issue: None,
            payloads: Vec::new(),
        }
    }

    pub(crate) fn custom_present(
        descriptor: &crate::DomainDescriptor,
        consistency_group: &str,
        adapter: &crate::BackupAdapterId,
        source_schema_version: SchemaVersion,
        payloads: Vec<BackupPayloadManifest>,
    ) -> Self {
        Self {
            domain: descriptor.id().clone(),
            storage_class: descriptor.storage_class(),
            consistency_group: consistency_group.into(),
            adapter: adapter.as_str().into(),
            state: BackupSourceState::Present,
            source_schema_version: Some(source_schema_version),
            source_issue: None,
            payloads,
        }
    }
}

/// Manifest record for one explicitly excluded selected domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupExclusion {
    domain: DomainId,
    storage_class: StorageClass,
    #[serde(deserialize_with = "deserialize_metadata")]
    reason: String,
}

impl BackupExclusion {
    pub(crate) fn new(
        descriptor: &crate::DomainDescriptor,
        reason: &crate::BackupExclusionReason,
    ) -> Self {
        Self {
            domain: descriptor.id().clone(),
            storage_class: descriptor.storage_class(),
            reason: reason.as_str().into(),
        }
    }

    /// Returns the excluded domain.
    #[must_use]
    pub fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the excluded domain's storage class.
    #[must_use]
    pub const fn storage_class(&self) -> StorageClass {
        self.storage_class
    }

    /// Returns the stable exclusion reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}
