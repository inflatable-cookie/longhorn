use super::super::{ConfigGeneration, ConfigOperationRejection, ConfigProtocolVersion};
use longhorn_core::ConfigRequestId;
use serde::{Deserialize, Serialize};

/// Explicit conflict choice for one included archive domain.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum RestoreConflictChoiceProjection {
    /// Select archive state for this domain.
    UseArchive,
    /// Preserve current state and skip this domain.
    KeepCurrent,
}

/// One exact domain choice.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreDomainChoice {
    /// Included manifest domain.
    pub domain_id: String,
    /// Explicit conflict resolution.
    pub choice: RestoreConflictChoiceProjection,
}

/// Current state bound into a restore plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum RestoreCurrentEvidenceProjection {
    /// No current target file exists.
    Absent,
    /// A current target file exists.
    Present {
        /// Exact current byte length.
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        byte_length: u64,
        /// Digest over exact current bytes.
        sha256: String,
    },
}

/// One explicit choice and derived action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestorePlanEntryProjection {
    /// Included manifest domain.
    pub domain_id: String,
    /// Explicit user choice.
    pub choice: RestoreConflictChoiceProjection,
    /// Derived action for a selected domain.
    pub action: Option<String>,
    /// Current evidence bound for a selected domain.
    pub current: Option<RestoreCurrentEvidenceProjection>,
}

/// Counts proving complete conflict planning.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestorePlanReceiptProjection {
    /// Domains selecting archive state.
    pub selected: usize,
    /// Domains preserving current state.
    pub skipped: usize,
    /// Missing targets to create.
    pub creates: usize,
    /// Existing targets to replace.
    pub replaces: usize,
    /// Existing targets to delete.
    pub deletes: usize,
    /// Selected sources requiring migration.
    pub migrations: usize,
    /// Selected targets already equal to archive state.
    pub unchanged: usize,
}

/// Complete confirmation-bound plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestorePlanProjection {
    /// Digest over the inspected archive.
    pub archive_sha256: String,
    /// Digest binding archive, choices, actions, and current evidence.
    pub confirmation_digest: String,
    /// Every explicit manifest-domain choice.
    pub entries: Vec<RestorePlanEntryProjection>,
    /// Complete action counts.
    pub receipt: RestorePlanReceiptProjection,
}

/// Plans explicit ordinary-domain choices against fresh current evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestorePlanCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Expected host generation.
    pub generation: ConfigGeneration,
    /// Exact inspected archive digest.
    pub archive_sha256: String,
    /// One explicit choice for every included domain.
    pub choices: Vec<RestoreDomainChoice>,
}

/// Result of exact conflict planning.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum RestorePlanOutcome {
    /// Every conflict was resolved into an exact plan.
    Ready {
        /// Fresh host generation.
        generation: ConfigGeneration,
        /// Confirmation-bound plan.
        plan: RestorePlanProjection,
    },
    /// No executable plan was retained.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}
