use serde::{Deserialize, Serialize};

use crate::RecoveryKind;

use super::identity::deserialize_metadata;

pub(super) const ORDINARY_CONSISTENCY_GROUP: &str = "longhorn-config-store";

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
