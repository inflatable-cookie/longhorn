//! Mutation rejection and cancellation results.

use longhorn_core::{
    OperationCatalogueRevision, OperationId, OperationRequestId, OperationRevision,
};
use serde::{Deserialize, Serialize};

use crate::protocol::{OperationSnapshot, OperationStateProjection};

use super::{OperationMutationReceiptProjection, OperationRemovalProjection};

/// Stable checked mutation rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationRejectionCode {
    /// Command protocol version is not supported.
    IncompatibleProtocol,
    /// Command failed structural validation.
    InvalidCommand,
    /// Authority no longer accepts mutations.
    AuthorityClosed,
    /// Authority composition does not match the catalogue's.
    AuthorityMismatch,
    /// Authority epoch does not match the catalogue's.
    AuthorityEpochMismatch,
    /// Expected catalogue revision is stale.
    CatalogueRevisionMismatch,
    /// Operation identity already exists.
    DuplicateOperation,
    /// Operation identity does not exist.
    UnknownOperation,
    /// Retry source is not valid for this mutation.
    InvalidRetrySource,
    /// Initial state is not valid for a new operation.
    InvalidInitialState,
    /// Requested state transition is not permitted.
    InvalidTransition,
    /// Expected operation revision is stale.
    OperationRevisionMismatch,
    /// Active-operation limit is reached.
    ActiveLimitReached,
    /// Requested active limit is below the current active count.
    ActiveLimitBelowCurrent,
    /// Operation state does not accept progress reports.
    ProgressNotReportable,
    /// Overall progress moved backwards.
    OverallProgressRegression,
    /// Phase progress moved backwards.
    PhaseProgressRegression,
    /// Dismissal requires a terminal operation state.
    DismissalRequiresTerminal,
    /// Teardown resolution names one operation twice.
    DuplicateTeardownResolution,
    /// Teardown omits resolutions for active operations.
    MissingTeardownResolutions,
    /// Teardown resolution names an unknown operation.
    UnexpectedTeardownResolution,
    /// Teardown resolution is not a valid terminal outcome.
    InvalidTeardownTerminal,
    /// Teardown transfer targets the operation's own authority.
    TeardownTransferToSelf,
    /// Catalogue capacity counter overflowed.
    CapacityOverflow,
}

/// Checked rejection with fresh-snapshot guidance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationRejection {
    /// Stable category.
    pub code: OperationRejectionCode,
    /// Product-neutral diagnostic.
    pub detail: String,
    /// Whether caller should load fresh authority before retry.
    pub refresh_required: bool,
}

/// Successful or checked-rejected management mutation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status",
    deny_unknown_fields
)]
pub enum OperationMutationResult {
    /// Authority committed the mutation.
    Committed {
        /// Echoed correlation identity.
        request_id: OperationRequestId,
        /// Fresh authoritative snapshot.
        snapshot: OperationSnapshot,
        /// Exact mutation receipt.
        receipt: Box<OperationMutationReceiptProjection>,
    },
    /// Authority rejected without mutation.
    Rejected {
        /// Echoed correlation identity.
        request_id: OperationRequestId,
        /// Unchanged authoritative snapshot.
        snapshot: OperationSnapshot,
        /// Checked rejection.
        rejection: OperationRejection,
    },
}

/// Cancellation admission outcome on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationCancellationOutcomeProjection {
    /// Newly accepted.
    Accepted,
    /// Already awaiting executor terminal fact.
    AlreadyRequested,
    /// Executor does not support cancellation.
    Unsupported,
    /// Operation already has a sticky terminal.
    Terminal,
}

/// Exact cancellation admission receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationCancellationReceiptProjection {
    /// Target operation.
    pub operation_id: OperationId,
    /// Admission classification.
    pub outcome: OperationCancellationOutcomeProjection,
    /// State before admission.
    pub previous_state: OperationStateProjection,
    /// State after admission.
    pub committed_state: OperationStateProjection,
    /// Revision before admission.
    pub previous_operation_revision: OperationRevision,
    /// Revision after admission.
    pub committed_operation_revision: OperationRevision,
    /// Catalogue revision before admission.
    pub previous_catalogue_revision: OperationCatalogueRevision,
    /// Catalogue revision after admission.
    pub committed_catalogue_revision: OperationCatalogueRevision,
    /// Terminal evictions caused by queued cancellation.
    pub evicted: Vec<OperationRemovalProjection>,
}

/// Executor dispatch evidence after authority cancellation admission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationExecutorDispatchProjection {
    /// No running executor needs notification.
    NotRequired,
    /// Request reached the injected executor boundary.
    Requested,
    /// Authority committed, but executor dispatch failed visibly.
    Failed {
        /// Stable adapter code.
        code: String,
        /// Product-neutral diagnostic.
        message: String,
        /// Whether a fresh explicit dispatch may succeed.
        retryable: bool,
    },
}

/// Successful or checked-rejected cancellation request.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status",
    deny_unknown_fields
)]
pub enum OperationCancellationResult {
    /// Admission was classified by authority.
    Committed {
        /// Echoed correlation identity.
        request_id: OperationRequestId,
        /// Fresh authoritative snapshot.
        snapshot: OperationSnapshot,
        /// Exact cancellation receipt.
        receipt: OperationCancellationReceiptProjection,
        /// Separate executor dispatch evidence.
        executor_dispatch: OperationExecutorDispatchProjection,
    },
    /// Authority rejected without mutation.
    Rejected {
        /// Echoed correlation identity.
        request_id: OperationRequestId,
        /// Unchanged authoritative snapshot.
        snapshot: OperationSnapshot,
        /// Checked rejection.
        rejection: OperationRejection,
    },
}

impl OperationCancellationResult {
    /// Replaces executor dispatch evidence without changing authority facts.
    #[must_use]
    pub fn with_executor_dispatch(self, dispatch: OperationExecutorDispatchProjection) -> Self {
        match self {
            Self::Committed {
                request_id,
                snapshot,
                receipt,
                ..
            } => Self::Committed {
                request_id,
                snapshot,
                receipt,
                executor_dispatch: dispatch,
            },
            rejected @ Self::Rejected { .. } => rejected,
        }
    }

    /// Returns an accepted running operation requiring executor dispatch.
    #[must_use]
    pub const fn executor_dispatch_operation_id(&self) -> Option<&OperationId> {
        match self {
            Self::Committed { receipt, .. }
                if matches!(
                    receipt.outcome,
                    OperationCancellationOutcomeProjection::Accepted
                ) && matches!(
                    receipt.committed_state,
                    OperationStateProjection::Cancelling
                ) =>
            {
                Some(&receipt.operation_id)
            }
            _ => None,
        }
    }
}
