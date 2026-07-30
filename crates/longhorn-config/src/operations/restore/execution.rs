use super::super::{
    BackupPublicationReceiptProjection, ConfigGeneration, ConfigOperationRejection,
    ConfigOperationsSnapshot, ConfigProtocolVersion,
};
use super::RestoreAdapterParticipationProjection;
use longhorn_core::ConfigRequestId;
use serde::{Deserialize, Serialize};

/// Executes the exact host-retained ordinary restore plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreExecuteCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Expected host generation.
    pub generation: ConfigGeneration,
    /// Exact host-issued plan digest.
    pub confirmation_digest: String,
}

/// Private staging evidence retained before live mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreStagingReceiptProjection {
    /// Selected domains staged together.
    pub selected: usize,
    /// Current-schema documents held privately.
    pub documents: usize,
    /// Selected deletions.
    pub deletions: usize,
    /// Selected domains needing no publication.
    pub unchanged: usize,
    /// Exact private document byte count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_document_bytes: u64,
}

/// Fully verified successful restore receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreExecutionReceiptProjection {
    /// Executed plan digest.
    pub confirmation_digest: String,
    /// Complete private staging evidence.
    pub staging: RestoreStagingReceiptProjection,
    /// Verified pre-restore safety backup.
    pub safety_backup: BackupPublicationReceiptProjection,
    /// Domains whose documents were published.
    pub restored_domain_ids: Vec<String>,
    /// Domains deleted to reproduce archive absence.
    pub deleted_domain_ids: Vec<String>,
    /// Restored domains migrated in private staging.
    pub migrated_domain_ids: Vec<String>,
    /// Selected domains already matching the target.
    pub unchanged_domain_ids: Vec<String>,
    /// Domains explicitly preserving current state.
    pub skipped_domain_ids: Vec<String>,
    /// Manifest exclusions never selected.
    pub excluded_domain_ids: Vec<String>,
}

/// Exact failed execution phase and terminal state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreExecutionFailureProjection {
    /// Failed journaled transaction phase.
    pub stage: String,
    /// First affected domain when known.
    pub domain_id: Option<String>,
    /// Exact terminal guarantee established before return.
    pub terminal: String,
    /// Safe failure detail.
    pub detail: String,
}

/// Exact terminal result of ordinary restore execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum RestoreExecuteOutcome {
    /// Every selected target was published and verified.
    Succeeded {
        /// Exact success evidence.
        receipt: Box<RestoreExecutionReceiptProjection>,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// Publication failed and exact prior state was verified.
    RolledBack {
        /// Failed phase and rollback guarantee.
        failure: RestoreExecutionFailureProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// Rollback could not be verified and mutation remains blocked.
    RecoveryRequired {
        /// Failed phase and recovery-required guarantee.
        failure: RestoreExecutionFailureProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// No live restore publication started.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}

/// Minimum guarantee required from one custom adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum RestoreAdapterRequirementProjection {
    /// Refuse adapters that cannot prove exact rollback.
    FailureAtomic,
    /// Permit a separately receipted adapter operation.
    AllowSeparate,
}

/// Executes one explicitly confirmed custom adapter operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreAdapterExecuteCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Expected host generation.
    pub generation: ConfigGeneration,
    /// Exact inspected archive digest.
    pub archive_sha256: String,
    /// Explicit custom domain.
    pub domain_id: String,
    /// Adapter-issued confirmation digest.
    pub confirmation_digest: String,
    /// Minimum accepted transaction guarantee.
    pub requirement: RestoreAdapterRequirementProjection,
}

/// Exact custom-adapter terminal receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreAdapterReceiptProjection {
    /// Restored custom domain.
    pub domain_id: String,
    /// Stable adapter id.
    pub adapter: String,
    /// Adapter transaction guarantee.
    pub participation: RestoreAdapterParticipationProjection,
    /// Exact confirmed adapter preview.
    pub confirmation_digest: String,
    /// Verified, rolled-back, or recovery-required terminal.
    pub outcome: String,
    /// Exact semantic evidence when supplied.
    pub evidence: Option<String>,
}

/// Result of one separately authorized custom restore.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum RestoreAdapterExecuteOutcome {
    /// Adapter authority returned a truthful terminal receipt.
    Completed {
        /// Exact adapter receipt.
        receipt: RestoreAdapterReceiptProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// Adapter mutation did not start.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}

/// Requests verified rollback or terminal journal cleanup.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreRecoveryCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
}

/// Exact restore recovery receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreRecoveryReceiptProjection {
    /// No-op, verified rollback, or terminal cleanup.
    pub outcome: String,
    /// Journal domains considered by recovery.
    pub domain_ids: Vec<String>,
}

/// Result of explicit restore recovery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum RestoreRecoveryOutcomeProjection {
    /// Recovery reached a verified safe state.
    Recovered {
        /// Exact recovery evidence.
        receipt: RestoreRecoveryReceiptProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// Recovery still cannot verify exact prior state.
    RecoveryRequired {
        /// Safe recovery failure.
        failure: RestoreExecutionFailureProjection,
        /// Fresh authority snapshot.
        snapshot: Box<ConfigOperationsSnapshot>,
    },
    /// Recovery could not start.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}
