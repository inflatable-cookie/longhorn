//! Renderer-safe storage and backup operation protocol.
//!
//! These types project exact host evidence. They deliberately exclude
//! filesystem capabilities, executable transition or retention plans,
//! encryption identities, and archive payloads.

mod backup;
mod projection;
mod restore;

pub use backup::*;
pub use restore::*;

use longhorn_core::ConfigRequestId;
use serde::{Deserialize, Deserializer, Serialize, de};

/// Current config-operations wire protocol version.
pub const CONFIG_OPERATIONS_PROTOCOL_VERSION: u16 = 1;

/// Exact config-operations protocol version accepted by this crate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct ConfigProtocolVersion(u16);

impl ConfigProtocolVersion {
    /// Current supported version.
    pub const CURRENT: Self = Self(CONFIG_OPERATIONS_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ConfigProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let actual = u16::deserialize(deserializer)?;
        if actual == CONFIG_OPERATIONS_PROTOCOL_VERSION {
            Ok(Self::CURRENT)
        } else {
            Err(de::Error::custom(format!(
                "unsupported config-operations protocol version {actual}; expected \
                 {CONFIG_OPERATIONS_PROTOCOL_VERSION}"
            )))
        }
    }
}

/// Monotonic host snapshot generation.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct ConfigGeneration(u64);

impl ConfigGeneration {
    /// Constructs a generation from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized generation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A host path could not be represented exactly on the UTF-8 wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigOperationProjectionError;

impl std::fmt::Display for ConfigOperationProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("config operation path is not valid UTF-8")
    }
}

impl std::error::Error for ConfigOperationProjectionError {}

/// Optional config-operation authority composed by the host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ConfigOperationCapability {
    /// Storage diagnostics are available.
    StorageDiagnostics,
    /// Storage profile transitions are available.
    StorageTransition,
    /// Backup inventory is available.
    BackupInventory,
    /// Backup capture and operational publication are available.
    BackupCreate,
    /// User-selected backup export is available.
    BackupExport,
    /// Host-owned retention policy is available.
    BackupRetention,
    /// An encryption provider is composed.
    BackupEncryption,
    /// Restore archive selection and inspection are available.
    RestoreInspection,
    /// Ordinary confirmation-bound restore execution is available.
    RestoreExecution,
    /// Explicit custom-adapter restore is available.
    RestoreAdapterExecution,
    /// Journaled restore recovery is available.
    RestoreRecovery,
}

/// Built-in storage profile identity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum StorageProfileId {
    /// Native lifecycle roots for each platform.
    PlatformNativeV1,
    /// One durable app root with typed child roots.
    UnifiedAppRootV1,
    /// One shared durable product root with typed child roots.
    SharedProductRootV1,
    /// One user-selected portable root with typed child roots.
    PortableV1,
}

/// Safe source label for the effective storage leaf.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum StorageLeafProvenanceProjection {
    /// Canonical application id supplied the leaf.
    CanonicalApplicationId,
    /// Explicit stable storage name supplied the leaf.
    StableStorageName,
}

/// One exact resolved storage root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageRootProjection {
    /// Stable root kind.
    pub kind: String,
    /// Exact resolved absolute path.
    pub path: String,
    /// Stable authority provenance.
    pub provenance: String,
}

/// Exact active storage layout diagnostics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageLayoutProjection {
    /// Active built-in profile.
    pub profile: StorageProfileId,
    /// Resolved target platform.
    pub platform: String,
    /// Canonical machine application identity.
    pub canonical_application_id: String,
    /// Effective app-specific directory leaf.
    pub effective_leaf: String,
    /// Source of the effective leaf.
    pub leaf_provenance: StorageLeafProvenanceProjection,
    /// Resolved roots in stable root-kind order.
    pub roots: Vec<StorageRootProjection>,
    /// Stable visible profile consequences.
    pub warnings: Vec<String>,
    /// Deterministic digest of this exact layout.
    pub layout_digest: String,
}

/// Bootstrap locator and recovery state without writable path authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum StorageBootstrapProjection {
    /// A profile was selected normally.
    Selected {
        /// Selection source.
        origin: String,
        /// Host-visible locator path, when a locator owns selection.
        locator_path: Option<String>,
        /// Transition that last committed selection.
        transition_id: Option<String>,
        /// Digest last committed by the locator.
        last_committed_layout_digest: Option<String>,
    },
    /// Bootstrap requires explicit recovery.
    RecoveryRequired {
        /// Stable recovery class.
        kind: String,
        /// Safe recovery detail.
        detail: String,
        /// Locator path involved in recovery.
        locator_path: String,
        /// Journal path involved in recovery.
        journal_path: String,
    },
}

/// Storage operations projected in one snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageOperationsProjection {
    /// Exact active layout.
    pub layout: StorageLayoutProjection,
    /// Locator or recovery state.
    pub bootstrap: StorageBootstrapProjection,
    /// Host-supported profile targets.
    pub available_profiles: Vec<StorageProfileId>,
}

/// One domain in a host-issued transition preview.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTransitionDomainProjection {
    /// Stable configuration domain id.
    pub domain_id: String,
    /// Storage lifecycle class.
    pub storage_class: String,
    /// Planned action.
    pub action: String,
    /// Exact source path when present.
    pub source_path: Option<String>,
    /// Exact target path when present.
    pub target_path: Option<String>,
    /// Source evidence digest when present.
    pub source_sha256: Option<String>,
}

/// One transition conflict that prevents execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTransitionConflictProjection {
    /// Stable conflict class.
    pub kind: String,
    /// Affected path when available.
    pub path: Option<String>,
    /// Safe conflict detail.
    pub detail: String,
}

/// Side-effect-free, host-issued transition preview.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTransitionPreviewProjection {
    /// Current layout digest.
    pub source_layout_digest: String,
    /// Resolved target layout digest.
    pub target_layout_digest: String,
    /// Target profile.
    pub target_profile: StorageProfileId,
    /// Known domain actions.
    pub domains: Vec<StorageTransitionDomainProjection>,
    /// Unregistered source paths retained by the host.
    pub unknown_source_paths: Vec<String>,
    /// Blocking or visible conflicts.
    pub conflicts: Vec<StorageTransitionConflictProjection>,
    /// Digest over exact inspected evidence.
    pub evidence_digest: String,
    /// Host-issued digest required for execution.
    pub confirmation_digest: String,
}

/// Successful committed transition receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTransitionReceiptProjection {
    /// Durable transition identity.
    pub transition_id: String,
    /// Stable terminal outcome.
    pub outcome: String,
    /// Committed target layout digest.
    pub target_layout_digest: String,
    /// Domains copied by the ordinary file path.
    pub copied_domain_ids: Vec<String>,
    /// Domains committed by custom adapters.
    pub custom_domain_ids: Vec<String>,
    /// Exact source paths retained after commit.
    pub retained_source_paths: Vec<String>,
    /// Digest over the exact committed receipt.
    pub receipt_digest: String,
}

/// Successful storage recovery receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageRecoveryReceiptProjection {
    /// Recovered transition identity when one existed.
    pub transition_id: Option<String>,
    /// Stable recovery outcome.
    pub outcome: String,
    /// Active layout digest after recovery.
    pub active_layout_digest: String,
    /// Safe recovery detail.
    pub detail: String,
}

/// Successful receipt-bound source cleanup.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageCleanupReceiptProjection {
    /// Transition whose receipt authorized cleanup.
    pub transition_id: String,
    /// Committed receipt digest rechecked by the host.
    pub transition_receipt_digest: String,
    /// Exact paths removed.
    pub removed_paths: Vec<String>,
}

/// Complete caller-authorized config operations snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConfigOperationsSnapshot {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Monotonic host generation.
    pub generation: ConfigGeneration,
    /// Operations composed for this caller.
    pub capabilities: Vec<ConfigOperationCapability>,
    /// Storage projection when authorized.
    pub storage: Option<StorageOperationsProjection>,
    /// Backup projection when authorized.
    pub backup: Option<BackupOperationsProjection>,
    /// Restore operation and recovery state when authorized.
    pub restore: Option<RestoreOperationsProjection>,
}

/// Loads one caller-authorized operations snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConfigSnapshotCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
}

/// Inspects one target profile. Portable root choice stays in the host picker.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTransitionInspectCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Target built-in profile.
    pub target_profile: StorageProfileId,
    /// Whether the host should include registered log domains.
    pub include_logs: bool,
}

/// Executes the currently matching host-retained transition preview.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTransitionExecuteCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Expected host generation.
    pub generation: ConfigGeneration,
    /// Digest issued by transition inspection.
    pub confirmation_digest: String,
}

/// Recovers an interrupted storage transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageRecoveryCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
}

/// Removes retained source paths authorized by one committed receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageCleanupCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Committed transition identity.
    pub transition_id: String,
    /// Digest of the exact committed transition receipt.
    pub transition_receipt_digest: String,
}

/// Stable domain rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ConfigOperationRejectionCode {
    /// Caller lacks the required capability.
    Unauthorized,
    /// Requested operation is not composed.
    Unsupported,
    /// Host authority changed since inspection.
    AuthorityChanged,
    /// Confirmation does not match exact host evidence.
    ConfirmationMismatch,
    /// A storage transition is already active.
    TransitionActive,
    /// Storage transition recovery must run first.
    RecoveryRequired,
    /// Transition inspection found blocking conflicts.
    Conflicts,
    /// Pending config publication blocks backup capture.
    PendingPublication,
    /// Required host-owned picker interaction was cancelled.
    SelectionCancelled,
    /// Archive is absent or no longer matches its digest.
    ArchiveChanged,
    /// Archive bytes are corrupt or fail strict inventory validation.
    ArchiveCorrupt,
    /// Archive format is newer than this host supports.
    ArchiveFutureVersion,
    /// Archive application or producer identity is incompatible.
    IdentityMismatch,
    /// Current evidence changed after restore confirmation.
    RestorePlanStale,
    /// A destructive restore is already active.
    RestoreActive,
    /// Custom-adapter authority changed after inspection.
    RestoreAdapterChanged,
    /// Backup inventory is incomplete, so pruning is unsafe.
    IncompleteInventory,
    /// Encryption provider requires host-owned interaction.
    EncryptionInteractionRequired,
    /// Host policy rejected the operation.
    PolicyBlocked,
}

/// Safe domain rejection without authority capabilities or secret material.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConfigOperationRejection {
    /// Stable rejection category.
    pub code: ConfigOperationRejectionCode,
    /// Safe diagnostic detail.
    pub detail: String,
    /// Fresh snapshot when authority can provide one.
    pub snapshot: Option<Box<ConfigOperationsSnapshot>>,
}

/// Outcome of transition inspection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum StorageTransitionInspectOutcome {
    /// Inspection produced an exact preview.
    Ready {
        /// Fresh host generation.
        generation: ConfigGeneration,
        /// Host-issued exact preview.
        preview: StorageTransitionPreviewProjection,
    },
    /// Inspection did not produce an executable preview.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}

/// Outcome of confirmed transition execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum StorageTransitionExecuteOutcome {
    /// Transition committed through locator-last publication.
    Committed {
        /// Exact durable receipt.
        receipt: StorageTransitionReceiptProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// No transition was published.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}

/// Outcome of explicit transition recovery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum StorageRecoveryOutcome {
    /// Recovery reached a safe terminal state.
    Recovered {
        /// Exact recovery evidence.
        receipt: StorageRecoveryReceiptProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// Recovery could not start.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}

/// Outcome of receipt-bound source cleanup.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum StorageCleanupOutcome {
    /// Exact authorized paths were removed.
    Applied {
        /// Exact cleanup receipt.
        receipt: StorageCleanupReceiptProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// No source path was removed.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}
