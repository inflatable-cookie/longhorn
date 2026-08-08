//! Mutation and cancellation commands.

use longhorn_core::{
    OperationCatalogueRevision, OperationId, OperationKindId, OperationRequestId, OperationRevision,
    OperationScopeId,
};
use serde::{Deserialize, Serialize};

use crate::{OperationRemoval, OperationRemovalReason};
use crate::protocol::{
    OperationAuthorityProjection, OperationCancellationSupportProjection,
    OperationCatalogueLimitsProjection, OperationEntryProjection,
    OperationOverallProgressProjection, OperationPhaseProgressProjection,
    OperationProgressProjection, OperationProtocolVersion, OperationSnapshot,
    OperationStateProjection, OperationTeardownResolutionProjection, incompatible_protocol,
};

/// Typed management mutation command.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationMutationCommand {
    /// Register consumer-admitted work.
    Register {
        /// Correlation identity.
        request_id: OperationRequestId,
        /// Exact protocol line.
        protocol_version: OperationProtocolVersion,
        /// Target authority.
        authority: OperationAuthorityProjection,
        /// Expected catalogue revision.
        expected_catalogue_revision: OperationCatalogueRevision,
        /// New operation identity.
        operation_id: OperationId,
        /// Consumer-owned kind.
        kind_id: OperationKindId,
        /// Optional consumer scope.
        scope_id: Option<OperationScopeId>,
        /// Presentation label.
        label: String,
        /// Queued or running initial state.
        initial_state: OperationStateProjection,
        /// Executor cancellation support.
        cancellation_support: OperationCancellationSupportProjection,
        /// Optional retained terminal retry source.
        retry_of: Option<OperationId>,
    },
    /// Commit monotonic progress.
    Progress {
        /// Correlation identity.
        request_id: OperationRequestId,
        /// Exact protocol line.
        protocol_version: OperationProtocolVersion,
        /// Target authority.
        authority: OperationAuthorityProjection,
        /// Operation identity.
        operation_id: OperationId,
        /// Expected operation revision.
        expected_operation_revision: OperationRevision,
        /// Overall progress.
        overall: OperationOverallProgressProjection,
        /// Optional phase update.
        phase: Option<OperationPhaseProgressProjection>,
    },
    /// Commit one legal lifecycle transition.
    Transition {
        /// Correlation identity.
        request_id: OperationRequestId,
        /// Exact protocol line.
        protocol_version: OperationProtocolVersion,
        /// Target authority.
        authority: OperationAuthorityProjection,
        /// Operation identity.
        operation_id: OperationId,
        /// Expected operation revision.
        expected_operation_revision: OperationRevision,
        /// Requested state.
        next_state: OperationStateProjection,
    },
    /// Change finite catalogue limits.
    ChangeRetention {
        /// Correlation identity.
        request_id: OperationRequestId,
        /// Exact protocol line.
        protocol_version: OperationProtocolVersion,
        /// Target authority.
        authority: OperationAuthorityProjection,
        /// Expected catalogue revision.
        expected_catalogue_revision: OperationCatalogueRevision,
        /// New finite limits.
        limits: OperationCatalogueLimitsProjection,
    },
    /// Explicitly dismiss one terminal projection.
    Dismiss {
        /// Correlation identity.
        request_id: OperationRequestId,
        /// Exact protocol line.
        protocol_version: OperationProtocolVersion,
        /// Target authority.
        authority: OperationAuthorityProjection,
        /// Operation identity.
        operation_id: OperationId,
        /// Expected operation revision.
        expected_operation_revision: OperationRevision,
    },
    /// Resolve every active operation and close the authority.
    Teardown {
        /// Correlation identity.
        request_id: OperationRequestId,
        /// Exact protocol line.
        protocol_version: OperationProtocolVersion,
        /// Target authority.
        authority: OperationAuthorityProjection,
        /// Expected catalogue revision.
        expected_catalogue_revision: OperationCatalogueRevision,
        /// Complete active-operation resolutions.
        resolutions: Vec<OperationTeardownResolutionProjection>,
    },
}

impl OperationMutationCommand {
    /// Returns the request correlation identity.
    #[must_use]
    pub const fn request_id(&self) -> &OperationRequestId {
        match self {
            Self::Register { request_id, .. }
            | Self::Progress { request_id, .. }
            | Self::Transition { request_id, .. }
            | Self::ChangeRetention { request_id, .. }
            | Self::Dismiss { request_id, .. }
            | Self::Teardown { request_id, .. } => request_id,
        }
    }

    pub(crate) const fn protocol_version(&self) -> OperationProtocolVersion {
        match self {
            Self::Register {
                protocol_version, ..
            }
            | Self::Progress {
                protocol_version, ..
            }
            | Self::Transition {
                protocol_version, ..
            }
            | Self::ChangeRetention {
                protocol_version, ..
            }
            | Self::Dismiss {
                protocol_version, ..
            }
            | Self::Teardown {
                protocol_version, ..
            } => *protocol_version,
        }
    }
}

/// Revision-bound cancellation command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationCancellationCommand {
    /// Correlation identity.
    pub request_id: OperationRequestId,
    /// Exact protocol line.
    pub protocol_version: OperationProtocolVersion,
    /// Target authority.
    pub authority: OperationAuthorityProjection,
    /// Target operation.
    pub operation_id: OperationId,
    /// Expected operation revision.
    pub expected_operation_revision: OperationRevision,
}

