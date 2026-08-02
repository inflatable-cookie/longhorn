mod evidence;
mod identity;
mod snapshot;

use longhorn_core::{DomainId, SchemaVersion};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{RecoveryKind, StorageClass};

pub use evidence::{
    BackupPayloadManifest, BackupPayloadPath, BackupPayloadPathError, Sha256Digest,
    Sha256DigestError,
};
pub use identity::{
    BackupApplication, BackupCaptureOptions, BackupKind, BackupLimits, BackupLimitsError,
    BackupMetadata, BackupMetadataError, BackupProducer, BackupScope, BackupScopeError,
};
pub use snapshot::{
    BackupAdapterCaptureReceipt, BackupCaptureReceipt, BackupSnapshot, BackupSnapshotPayload,
};

pub(crate) use identity::{UtcTimestamp, parse_utc_timestamp};
use identity::{deserialize_metadata, deserialize_utc_timestamp};

const BACKUP_FORMAT: &str = "longhorn.config-backup";
const BACKUP_FORMAT_VERSION: u32 = 1;
const ORDINARY_ADAPTER: &str = "longhorn-json-v1";
const ORDINARY_CONSISTENCY_GROUP: &str = "longhorn-config-store";

/// Consistency mode claimed by one manifest group.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupConsistencyMode {
    /// Bounded capture under the Longhorn store coordinator.
    CoordinatedBounded,
    /// Immutable snapshot created by an external transaction authority.
    ExternalSnapshot,
}

/// One independently consistent group in a backup manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupConsistencyGroup {
    #[serde(deserialize_with = "deserialize_metadata")]
    id: String,
    mode: BackupConsistencyMode,
    #[serde(deserialize_with = "deserialize_metadata")]
    authority: String,
}

impl BackupConsistencyGroup {
    pub(crate) fn ordinary() -> Self {
        Self {
            id: ORDINARY_CONSISTENCY_GROUP.into(),
            mode: BackupConsistencyMode::CoordinatedBounded,
            authority: "longhorn-config-store-coordinator".into(),
        }
    }

    pub(crate) fn external(group: &crate::backup::BackupAdapterConsistencyGroup) -> Self {
        Self {
            id: group.id().into(),
            mode: BackupConsistencyMode::ExternalSnapshot,
            authority: group.authority().into(),
        }
    }

    /// Returns the stable group id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the group's consistency mode.
    #[must_use]
    pub const fn mode(&self) -> BackupConsistencyMode {
        self.mode
    }

    /// Returns the declared transaction authority.
    #[must_use]
    pub fn authority(&self) -> &str {
        &self.authority
    }
}

/// Captured source state for one selected domain.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupSourceState {
    /// Exact valid ordinary source bytes are present.
    Present,
    /// No persisted source exists.
    Absent,
    /// Readable source is preserved but is not ordinarily restorable.
    SourcePreserved,
}

/// Stable reason why readable source was preserved but is not restorable.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupSourceIssue {
    /// JSON envelope was malformed.
    CorruptDocument,
    /// Envelope named another domain.
    DomainMismatch,
    /// Schema is newer than the registered domain.
    FutureSchema,
    /// Raw or decoded value failed validation.
    InvalidValue,
    /// Required migration was absent.
    MissingMigration,
    /// Migration returned an invalid target.
    InvalidMigrationStep,
    /// Consumer migration code failed.
    MigrationFailed,
    /// Current raw value could not be decoded.
    DecodeFailed,
}

impl BackupSourceIssue {
    pub(crate) fn from_recovery(kind: RecoveryKind) -> Option<Self> {
        match kind {
            RecoveryKind::CorruptDocument => Some(Self::CorruptDocument),
            RecoveryKind::DomainMismatch => Some(Self::DomainMismatch),
            RecoveryKind::FutureSchema => Some(Self::FutureSchema),
            RecoveryKind::InvalidValue => Some(Self::InvalidValue),
            RecoveryKind::MissingMigration => Some(Self::MissingMigration),
            RecoveryKind::InvalidMigrationStep => Some(Self::InvalidMigrationStep),
            RecoveryKind::MigrationFailed => Some(Self::MigrationFailed),
            RecoveryKind::DecodeFailed => Some(Self::DecodeFailed),
            RecoveryKind::ReadFailed | RecoveryKind::InvalidDefault => None,
        }
    }
}

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
        reason: &super::BackupExclusionReason,
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

/// Strict version-1 backup manifest model.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifest {
    #[serde(deserialize_with = "deserialize_format")]
    format: String,
    #[serde(deserialize_with = "deserialize_format_version")]
    format_version: u32,
    #[serde(deserialize_with = "deserialize_metadata")]
    archive_id: String,
    kind: BackupKind,
    #[serde(deserialize_with = "deserialize_utc_timestamp")]
    created_at: String,
    application: BackupApplication,
    producer: BackupProducer,
    consistency_groups: Vec<BackupConsistencyGroup>,
    domains: Vec<BackupManifestDomain>,
    exclusions: Vec<BackupExclusion>,
}

impl BackupManifest {
    pub(crate) fn new(
        metadata: BackupMetadata,
        consistency_groups: Vec<BackupConsistencyGroup>,
        domains: Vec<BackupManifestDomain>,
        exclusions: Vec<BackupExclusion>,
    ) -> Self {
        Self {
            format: BACKUP_FORMAT.into(),
            format_version: BACKUP_FORMAT_VERSION,
            archive_id: metadata.archive_id,
            kind: metadata.kind,
            created_at: metadata.created_at,
            application: metadata.application,
            producer: metadata.producer,
            consistency_groups,
            domains,
            exclusions,
        }
    }

    /// Returns the fixed format id.
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Returns the fixed manifest format version.
    #[must_use]
    pub const fn format_version(&self) -> u32 {
        self.format_version
    }

    /// Returns the caller-supplied archive id.
    #[must_use]
    pub fn archive_id(&self) -> &str {
        &self.archive_id
    }

    /// Returns the backup kind.
    #[must_use]
    pub const fn kind(&self) -> BackupKind {
        self.kind
    }

    /// Returns the caller-supplied UTC creation time.
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Returns application identity.
    #[must_use]
    pub fn application(&self) -> &BackupApplication {
        &self.application
    }

    /// Returns producer identity.
    #[must_use]
    pub fn producer(&self) -> &BackupProducer {
        &self.producer
    }

    /// Returns declared consistency groups.
    #[must_use]
    pub fn consistency_groups(&self) -> &[BackupConsistencyGroup] {
        &self.consistency_groups
    }

    /// Returns included domains in stable domain-id order.
    #[must_use]
    pub fn domains(&self) -> &[BackupManifestDomain] {
        &self.domains
    }

    /// Returns exclusions in stable domain-id order.
    #[must_use]
    pub fn exclusions(&self) -> &[BackupExclusion] {
        &self.exclusions
    }

    pub(crate) fn with_kind(&self, kind: BackupKind) -> Self {
        let mut manifest = self.clone();
        manifest.kind = kind;
        manifest
    }
}

fn deserialize_format<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value == BACKUP_FORMAT {
        Ok(value)
    } else {
        Err(de::Error::custom(format!(
            "unsupported backup format {value}"
        )))
    }
}

fn deserialize_format_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = u32::deserialize(deserializer)?;
    if value == BACKUP_FORMAT_VERSION {
        Ok(value)
    } else {
        Err(de::Error::custom(format!(
            "unsupported backup format version {value}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn strict_manifest_rejects_unknown_format_version_and_fields() {
        let base = json!({
            "format": BACKUP_FORMAT,
            "formatVersion": 1,
            "archiveId": "archive-1",
            "kind": "operational",
            "createdAt": "2026-07-28T12:00:00Z",
            "application": {"id": "com.example.app", "version": "1.0.0"},
            "producer": {"name": "longhorn", "version": "0.1.0"},
            "consistencyGroups": [],
            "domains": [],
            "exclusions": []
        });
        assert!(serde_json::from_value::<BackupManifest>(base.clone()).is_ok());

        let mut future = base.clone();
        future["formatVersion"] = json!(2);
        assert!(serde_json::from_value::<BackupManifest>(future).is_err());

        let mut unknown = base;
        unknown["surprise"] = json!(true);
        assert!(serde_json::from_value::<BackupManifest>(unknown).is_err());

        let mut empty_metadata = json!({
            "format": BACKUP_FORMAT,
            "formatVersion": 1,
            "archiveId": "",
            "kind": "operational",
            "createdAt": "2026-07-28T12:00:00Z",
            "application": {"id": "com.example.app", "version": "1.0.0"},
            "producer": {"name": "longhorn", "version": "0.1.0"},
            "consistencyGroups": [],
            "domains": [],
            "exclusions": []
        });
        assert!(serde_json::from_value::<BackupManifest>(empty_metadata.clone()).is_err());
        empty_metadata["archiveId"] = json!("archive-1");
        empty_metadata["application"]["id"] = json!("");
        assert!(serde_json::from_value::<BackupManifest>(empty_metadata).is_err());
    }
}
