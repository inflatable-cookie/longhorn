use super::{
    ConfigGeneration, ConfigOperationRejection, ConfigOperationsSnapshot, ConfigProtocolVersion,
};
use longhorn_core::ConfigRequestId;
use serde::{Deserialize, Serialize};

/// One inspected operational backup archive.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupArchiveProjection {
    /// Exact root-level archive path.
    pub path: String,
    /// Manifest archive id.
    pub archive_id: String,
    /// Strict UTC creation time.
    pub created_at: String,
    /// Backup kind.
    pub kind: String,
    /// Digest over exact published archive bytes.
    pub archive_sha256: String,
}

/// Safe inventory classification.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BackupInventoryEntryState {
    /// Valid inspected same-app archive.
    Valid,
    /// Encrypted archive cannot currently be inspected.
    Locked,
    /// Plaintext or decrypted archive is malformed.
    Corrupt,
    /// Archive belongs to another application.
    Foreign,
    /// Archive format or ownership is unknown.
    Unknown,
    /// Candidate could not be read.
    Unreadable,
    /// Entry is outside Longhorn archive management.
    Unmanaged,
}

/// Preserved inventory entry not eligible for automatic deletion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupInventoryEntry {
    /// Root entry path.
    pub path: Option<String>,
    /// Safe inventory state.
    pub state: BackupInventoryEntryState,
    /// Stable diagnostic class.
    pub diagnostic_kind: String,
    /// Safe diagnostic detail.
    pub detail: String,
}

/// Bounded operational backup inventory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupInventoryProjection {
    /// Operational backup root.
    pub root: String,
    /// Proven same-app candidates, newest first.
    pub archives: Vec<BackupArchiveProjection>,
    /// Preserved uninspectable or excluded entries.
    pub entries: Vec<BackupInventoryEntry>,
    /// Whether enumeration completed within host bounds.
    pub complete: bool,
}

/// Debounced publication state relevant to backup capture.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum BackupPendingState {
    /// No pending configuration publication exists.
    Clear,
    /// Pending publication must be refused or explicitly flushed.
    Pending {
        /// Number of pending domains.
        domain_count: usize,
        /// Stable domain ids awaiting publication.
        domain_ids: Vec<String>,
    },
}

/// Safe encryption availability without identity or secret material.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum BackupEncryptionState {
    /// No encryption provider is composed.
    Unavailable,
    /// Provider can create encrypted archives.
    Available {
        /// Safe provider display label.
        provider: String,
    },
    /// Provider exists but needs host-owned interaction.
    InteractionRequired {
        /// Safe provider display label.
        provider: String,
    },
    /// Provider is temporarily unavailable.
    Failed {
        /// Safe provider display label.
        provider: String,
        /// Redacted failure detail.
        detail: String,
    },
}

/// Why a proven backup remains protected from retention.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BackupRetentionReasonProjection {
    /// Archive was just published.
    NewArchive,
    /// Host policy pinned the archive.
    Pinned,
    /// Archive falls inside the newest-count tier.
    NewestCount,
    /// Archive falls inside the age tier.
    Age,
    /// Archive represents a milestone bucket.
    Milestone,
}

/// Host-owned retention plan projected for explicit confirmation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupRetentionProjection {
    /// Exact proven paths selected for deletion.
    pub deletion_paths: Vec<String>,
    /// Protected archive hash and reasons.
    pub retained: Vec<(String, Vec<BackupRetentionReasonProjection>)>,
    /// Preserved listing and planning diagnostics.
    pub diagnostics: Vec<BackupInventoryEntry>,
    /// Host-issued digest required to apply this exact plan.
    pub confirmation_digest: String,
}

/// Backup operations projected in one snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupOperationsProjection {
    /// Current bounded inventory.
    pub inventory: BackupInventoryProjection,
    /// Current pending-publication state.
    pub pending: BackupPendingState,
    /// Safe encryption state.
    pub encryption: BackupEncryptionState,
    /// Current host-owned retention plan when pruning is safe.
    pub retention: Option<BackupRetentionProjection>,
}

/// Renderer choice when capture encounters pending debounced publication.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum PendingBackupPolicy {
    /// Refuse capture until ordinary publication completes.
    Refuse,
    /// Ask the injected authority to flush before capture.
    Flush,
}

/// Creates and operationally publishes a backup under host-owned policy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupCreateCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Explicit handling for pending publication.
    pub pending_policy: PendingBackupPolicy,
}

/// Exports one proven operational archive to a host-selected destination.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupExportCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Exact operational archive digest selected by the user.
    pub archive_sha256: String,
}

/// Applies the currently matching host-retained retention plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupRetentionApplyCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Expected host generation.
    pub generation: ConfigGeneration,
    /// Digest issued with the exact retention plan.
    pub confirmation_digest: String,
}

/// Exact coordinated capture evidence safe for display.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupCaptureReceiptProjection {
    /// Domains considered by host scope.
    pub selected_domains: usize,
    /// Domains captured as exact source bytes.
    pub captured_domains: usize,
    /// Domains recorded absent.
    pub absent_domains: usize,
    /// Domains preserved as non-restorable source.
    pub source_preserved_domains: usize,
    /// Domains explicitly excluded.
    pub excluded_domains: usize,
    /// Domains captured by custom adapters.
    pub custom_domains: usize,
    /// Independently consistent external groups.
    pub external_consistency_groups: usize,
    /// Total retained payload bytes.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_payload_bytes: u64,
    /// Whether pending publication was flushed first.
    pub flushed_pending_publication: bool,
}

/// Exact verified archive publication evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupPublicationReceiptProjection {
    /// Exact published path.
    pub path: String,
    /// Operational or user-export destination.
    pub destination: String,
    /// Digest over exact published bytes.
    pub archive_sha256: String,
    /// Established durability level.
    pub durability: String,
    /// Whether an authorized export was replaced.
    pub replaced_existing: bool,
}

/// Outcome of backup creation and operational publication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum BackupCreateOutcome {
    /// Capture and verified operational publication completed.
    Published {
        /// Exact capture evidence.
        capture: BackupCaptureReceiptProjection,
        /// Exact publication evidence.
        publication: BackupPublicationReceiptProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// No archive was published.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}

/// Outcome of export to an injected user-selected target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum BackupExportOutcome {
    /// Exact archive was published to the selected target.
    Published {
        /// Exact publication evidence.
        publication: BackupPublicationReceiptProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// No export was published.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}

/// Outcome of applying a confirmed host-owned retention plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum BackupRetentionApplyOutcome {
    /// Exact proven candidates were deleted.
    Applied {
        /// Exact paths removed.
        deleted_paths: Vec<String>,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// No archive was deleted.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}
