//! Strict payload-free renderer and transport protocol.

use std::{error::Error, fmt};

use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationId, OperationKindId,
    OperationPhaseId, OperationRequestId, OperationRevision, OperationScopeId,
};
use serde::{Deserialize, Serialize};

use crate::{
    OperationAuthorityCursor, OperationAuthorityEpoch, OperationCancellationOutcome,
    OperationCancellationRequest, OperationCancellationSupport, OperationCatalogue,
    OperationCatalogueError, OperationCatalogueLimits, OperationCatalogueProjection,
    OperationDismissal, OperationNormalizedProgress, OperationOverallProgress, OperationPhaseLabel,
    OperationPhaseProgress, OperationProgress, OperationProgressUpdate, OperationRecord,
    OperationRegistration, OperationRemoval, OperationRemovalReason, OperationRetentionChange,
    OperationState, OperationTeardown, OperationTeardownOutcome, OperationTeardownResolution,
    OperationTeardownResolutionOutcome, OperationTransition, OperationUnitProgress,
};

/// Current exact operation protocol line.
pub const OPERATION_PROTOCOL_VERSION: u32 = 1;

/// Exact operation protocol version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct OperationProtocolVersion(u32);

impl OperationProtocolVersion {
    /// Current exact protocol version.
    pub const CURRENT: Self = Self(OPERATION_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Serialized authority identity and live epoch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationAuthorityProjection {
    /// Stable authority identity.
    pub authority_id: OperationAuthorityId,
    /// Nonzero authority lifetime.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
}

impl OperationAuthorityProjection {
    fn from_cursor(cursor: &OperationAuthorityCursor) -> Self {
        Self {
            authority_id: cursor.authority_id().clone(),
            authority_epoch: cursor.authority_epoch().get(),
        }
    }

    fn into_cursor(self) -> Result<OperationAuthorityCursor, OperationProtocolInputError> {
        let epoch = OperationAuthorityEpoch::new(self.authority_epoch)
            .map_err(|_| OperationProtocolInputError::AuthorityEpoch)?;
        Ok(OperationAuthorityCursor::new(self.authority_id, epoch))
    }
}

/// Serialized operation lifecycle state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationStateProjection {
    /// Accepted but not started.
    Queued,
    /// Executor reports active work.
    Running,
    /// Cancellation is requested but not terminal.
    Cancelling,
    /// Successful terminal outcome.
    Succeeded,
    /// Failed terminal outcome.
    Failed,
    /// Confirmed terminal cancellation.
    Cancelled,
    /// Consumer-proven executor or host loss.
    Interrupted,
}

impl From<OperationState> for OperationStateProjection {
    fn from(value: OperationState) -> Self {
        match value {
            OperationState::Queued => Self::Queued,
            OperationState::Running => Self::Running,
            OperationState::Cancelling => Self::Cancelling,
            OperationState::Succeeded => Self::Succeeded,
            OperationState::Failed => Self::Failed,
            OperationState::Cancelled => Self::Cancelled,
            OperationState::Interrupted => Self::Interrupted,
        }
    }
}

impl From<OperationStateProjection> for OperationState {
    fn from(value: OperationStateProjection) -> Self {
        match value {
            OperationStateProjection::Queued => Self::Queued,
            OperationStateProjection::Running => Self::Running,
            OperationStateProjection::Cancelling => Self::Cancelling,
            OperationStateProjection::Succeeded => Self::Succeeded,
            OperationStateProjection::Failed => Self::Failed,
            OperationStateProjection::Cancelled => Self::Cancelled,
            OperationStateProjection::Interrupted => Self::Interrupted,
        }
    }
}

/// Serialized cancellation support declared at registration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationCancellationSupportProjection {
    /// Consumer executor accepts requests.
    Supported,
    /// Consumer executor does not accept requests.
    Unsupported,
}

impl From<OperationCancellationSupport> for OperationCancellationSupportProjection {
    fn from(value: OperationCancellationSupport) -> Self {
        match value {
            OperationCancellationSupport::Supported => Self::Supported,
            OperationCancellationSupport::Unsupported => Self::Unsupported,
        }
    }
}

impl From<OperationCancellationSupportProjection> for OperationCancellationSupport {
    fn from(value: OperationCancellationSupportProjection) -> Self {
        match value {
            OperationCancellationSupportProjection::Supported => Self::Supported,
            OperationCancellationSupportProjection::Unsupported => Self::Unsupported,
        }
    }
}

/// Product-neutral overall progress on the wire.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationOverallProgressProjection {
    /// No determinate overall measure.
    Indeterminate,
    /// Completed and total consumer units.
    Units {
        /// Completed units.
        completed: f64,
        /// Total units.
        total: f64,
    },
    /// Normalized zero-through-one progress.
    Normalized {
        /// Normalized value.
        value: f64,
    },
}

impl OperationOverallProgressProjection {
    fn from_progress(progress: OperationOverallProgress) -> Self {
        match progress {
            OperationOverallProgress::Indeterminate => Self::Indeterminate,
            OperationOverallProgress::Units(value) => Self::Units {
                completed: value.completed(),
                total: value.total(),
            },
            OperationOverallProgress::Normalized(value) => Self::Normalized { value: value.get() },
        }
    }

    fn into_progress(self) -> Result<OperationOverallProgress, OperationProtocolInputError> {
        match self {
            Self::Indeterminate => Ok(OperationOverallProgress::Indeterminate),
            Self::Units { completed, total } => OperationUnitProgress::new(completed, total)
                .map(OperationOverallProgress::Units)
                .map_err(|error| OperationProtocolInputError::Progress(error.to_string())),
            Self::Normalized { value } => OperationNormalizedProgress::new(value)
                .map(OperationOverallProgress::Normalized)
                .map_err(|error| OperationProtocolInputError::Progress(error.to_string())),
        }
    }
}

/// Current phase-local progress on the wire.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationPhaseProgressProjection {
    /// Stable consumer phase identity.
    pub phase_id: OperationPhaseId,
    /// Bounded presentation label.
    pub label: String,
    /// Completed phase units.
    pub completed: f64,
    /// Total phase units.
    pub total: f64,
}

impl OperationPhaseProgressProjection {
    fn from_progress(progress: &OperationPhaseProgress) -> Self {
        Self {
            phase_id: progress.phase_id().clone(),
            label: progress.label().as_str().to_owned(),
            completed: progress.units().completed(),
            total: progress.units().total(),
        }
    }

    fn into_progress(self) -> Result<OperationPhaseProgress, OperationProtocolInputError> {
        let label = OperationPhaseLabel::new(self.label)
            .map_err(|error| OperationProtocolInputError::Phase(error.to_string()))?;
        let units = OperationUnitProgress::new(self.completed, self.total)
            .map_err(|error| OperationProtocolInputError::Progress(error.to_string()))?;
        Ok(OperationPhaseProgress::new(self.phase_id, label, units))
    }
}

/// Current authoritative progress projection.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationProgressProjection {
    /// Monotonic progress sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// Overall progress.
    pub overall: OperationOverallProgressProjection,
    /// Optional current phase.
    pub phase: Option<OperationPhaseProgressProjection>,
}

impl OperationProgressProjection {
    fn from_progress(progress: &OperationProgress) -> Self {
        Self {
            sequence: progress.sequence().get(),
            overall: OperationOverallProgressProjection::from_progress(progress.overall()),
            phase: progress
                .phase()
                .map(OperationPhaseProgressProjection::from_progress),
        }
    }
}

/// One payload-free authoritative operation entry.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationEntryProjection {
    /// Owning authority.
    pub authority: OperationAuthorityProjection,
    /// Stable operation identity.
    pub operation_id: OperationId,
    /// Consumer-owned kind.
    pub kind_id: OperationKindId,
    /// Optional consumer-owned scope.
    pub scope_id: Option<OperationScopeId>,
    /// Bounded presentation label.
    pub label: String,
    /// Cancellation support.
    pub cancellation_support: OperationCancellationSupportProjection,
    /// Optional retained retry source.
    pub retry_of: Option<OperationId>,
    /// Insertion sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// Current operation revision.
    pub revision: OperationRevision,
    /// Catalogue revision that last changed the entry.
    pub last_changed_catalogue_revision: OperationCatalogueRevision,
    /// Current lifecycle state.
    pub state: OperationStateProjection,
    /// Current progress.
    pub progress: OperationProgressProjection,
    /// Canonical structural metadata weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub encoded_metadata_weight: u64,
}

impl OperationEntryProjection {
    fn from_record(record: &OperationRecord) -> Self {
        Self {
            authority: OperationAuthorityProjection::from_cursor(record.authority()),
            operation_id: record.operation_id().clone(),
            kind_id: record.kind_id().clone(),
            scope_id: record.scope_id().cloned(),
            label: record.label().as_str().to_owned(),
            cancellation_support: record.cancellation_support().into(),
            retry_of: record.retry_of().cloned(),
            sequence: record.sequence().get(),
            revision: record.revision(),
            last_changed_catalogue_revision: record.last_changed_catalogue_revision(),
            state: record.state().into(),
            progress: OperationProgressProjection::from_progress(record.progress()),
            encoded_metadata_weight: record.encoded_metadata_weight(),
        }
    }
}

/// Finite catalogue limits on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationCatalogueLimitsProjection {
    /// Maximum active operations.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub maximum_active_operations: u64,
    /// Maximum retained terminal operations.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub maximum_terminal_operations: u64,
    /// Maximum terminal structural metadata weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub maximum_terminal_encoded_weight: u64,
}

impl OperationCatalogueLimitsProjection {
    fn from_limits(
        limits: OperationCatalogueLimits,
    ) -> Result<Self, OperationProtocolProjectionError> {
        Ok(Self {
            maximum_active_operations: project_usize(limits.maximum_active_operations())?,
            maximum_terminal_operations: project_usize(limits.maximum_terminal_operations())?,
            maximum_terminal_encoded_weight: limits.maximum_terminal_encoded_weight(),
        })
    }

    fn into_limits(self) -> Result<OperationCatalogueLimits, OperationProtocolInputError> {
        let active = usize::try_from(self.maximum_active_operations)
            .map_err(|_| OperationProtocolInputError::Limits)?;
        let terminal = usize::try_from(self.maximum_terminal_operations)
            .map_err(|_| OperationProtocolInputError::Limits)?;
        OperationCatalogueLimits::new(active, terminal, self.maximum_terminal_encoded_weight)
            .map_err(|_| OperationProtocolInputError::Limits)
    }
}

/// One exact bounded catalogue snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationSnapshot {
    /// Exact protocol line.
    pub protocol_version: OperationProtocolVersion,
    /// Live authority cursor.
    pub authority: OperationAuthorityProjection,
    /// Authoritative catalogue revision.
    pub catalogue_revision: OperationCatalogueRevision,
    /// Whether controlled teardown closed this authority.
    pub closed: bool,
    /// Current finite limits.
    pub limits: OperationCatalogueLimitsProjection,
    /// Cumulative finite-retention truncation evidence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub terminal_eviction_count: u64,
    /// Current retained terminal metadata weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_terminal_encoded_weight: u64,
    /// Active entries in insertion order.
    pub active: Vec<OperationEntryProjection>,
    /// Terminal entries newest first.
    pub recent: Vec<OperationEntryProjection>,
}

impl OperationSnapshot {
    /// Projects one authoritative catalogue without product payloads.
    pub fn from_catalogue(
        catalogue: &OperationCatalogue,
    ) -> Result<Self, OperationProtocolProjectionError> {
        let projection = catalogue.project();
        Self::from_projection(catalogue, &projection)
    }

    fn from_projection(
        catalogue: &OperationCatalogue,
        projection: &OperationCatalogueProjection,
    ) -> Result<Self, OperationProtocolProjectionError> {
        let retained_terminal_encoded_weight = catalogue
            .retained_terminal_encoded_weight()
            .map_err(|error| OperationProtocolProjectionError(error.to_string()))?;
        Ok(Self {
            protocol_version: OperationProtocolVersion::CURRENT,
            authority: OperationAuthorityProjection::from_cursor(projection.authority()),
            catalogue_revision: projection.catalogue_revision(),
            closed: projection.is_closed(),
            limits: OperationCatalogueLimitsProjection::from_limits(catalogue.limits())?,
            terminal_eviction_count: projection.terminal_eviction_count(),
            retained_terminal_encoded_weight,
            active: projection
                .active()
                .iter()
                .map(OperationEntryProjection::from_record)
                .collect(),
            recent: projection
                .recent()
                .iter()
                .map(OperationEntryProjection::from_record)
                .collect(),
        })
    }
}

/// Correlated snapshot query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationSnapshotQuery {
    /// Exact protocol line.
    pub protocol_version: OperationProtocolVersion,
    /// Correlation identity.
    pub request_id: OperationRequestId,
}

/// Correlated snapshot response.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationSnapshotResponse {
    /// Echoed correlation identity.
    pub request_id: OperationRequestId,
    /// Authoritative snapshot.
    pub snapshot: OperationSnapshot,
}

/// One teardown resolution carried by a management command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationTeardownResolutionProjection {
    /// Commit a terminal fact.
    Complete {
        /// Operation identity.
        operation_id: OperationId,
        /// Expected operation revision.
        expected_operation_revision: OperationRevision,
        /// Consumer-proven terminal state.
        state: OperationStateProjection,
    },
    /// Transfer to another live authority.
    Transfer {
        /// Operation identity.
        operation_id: OperationId,
        /// Expected operation revision.
        expected_operation_revision: OperationRevision,
        /// Receiving authority.
        target_authority: OperationAuthorityProjection,
    },
}

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

    const fn protocol_version(&self) -> OperationProtocolVersion {
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

/// Why one retained terminal entry was removed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationRemovalReasonProjection {
    /// Removed by finite retention.
    Evicted,
    /// Removed by explicit dismissal.
    Dismissed,
}

/// Exact removed terminal metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationRemovalProjection {
    /// Removed operation identity.
    pub operation_id: OperationId,
    /// Original insertion sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// Canonical removed metadata weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub encoded_weight: u64,
    /// Removal reason.
    pub reason: OperationRemovalReasonProjection,
}

impl From<&OperationRemoval> for OperationRemovalProjection {
    fn from(value: &OperationRemoval) -> Self {
        Self {
            operation_id: value.operation_id().clone(),
            sequence: value.sequence().get(),
            encoded_weight: value.encoded_weight(),
            reason: match value.reason() {
                OperationRemovalReason::Evicted => OperationRemovalReasonProjection::Evicted,
                OperationRemovalReason::Dismissed => OperationRemovalReasonProjection::Dismissed,
            },
        }
    }
}

/// Exact successful management receipt.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationMutationReceiptProjection {
    Registered {
        operation: OperationEntryProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    Progressed {
        operation_id: OperationId,
        previous_operation_revision: OperationRevision,
        committed_operation_revision: OperationRevision,
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        previous_progress_sequence: u64,
        committed_progress: OperationProgressProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    Transitioned {
        operation_id: OperationId,
        previous_state: OperationStateProjection,
        committed_state: OperationStateProjection,
        previous_operation_revision: OperationRevision,
        committed_operation_revision: OperationRevision,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
        evicted: Vec<OperationRemovalProjection>,
    },
    RetentionChanged {
        previous_limits: OperationCatalogueLimitsProjection,
        committed_limits: OperationCatalogueLimitsProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
        evicted: Vec<OperationRemovalProjection>,
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        retained_terminal_encoded_weight: u64,
    },
    Dismissed {
        removed: OperationRemovalProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    TornDown {
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
        outcomes: Vec<OperationTeardownOutcomeProjection>,
        evicted: Vec<OperationRemovalProjection>,
    },
}

impl OperationMutationReceiptProjection {
    /// Returns the previous catalogue revision.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        match self {
            Self::Registered {
                previous_catalogue_revision,
                ..
            }
            | Self::Progressed {
                previous_catalogue_revision,
                ..
            }
            | Self::Transitioned {
                previous_catalogue_revision,
                ..
            }
            | Self::RetentionChanged {
                previous_catalogue_revision,
                ..
            }
            | Self::Dismissed {
                previous_catalogue_revision,
                ..
            }
            | Self::TornDown {
                previous_catalogue_revision,
                ..
            } => *previous_catalogue_revision,
        }
    }

    /// Returns the committed catalogue revision.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        match self {
            Self::Registered {
                committed_catalogue_revision,
                ..
            }
            | Self::Progressed {
                committed_catalogue_revision,
                ..
            }
            | Self::Transitioned {
                committed_catalogue_revision,
                ..
            }
            | Self::RetentionChanged {
                committed_catalogue_revision,
                ..
            }
            | Self::Dismissed {
                committed_catalogue_revision,
                ..
            }
            | Self::TornDown {
                committed_catalogue_revision,
                ..
            } => *committed_catalogue_revision,
        }
    }

    /// Returns one directly targeted operation when present.
    #[must_use]
    pub const fn operation_id(&self) -> Option<&OperationId> {
        match self {
            Self::Registered { operation, .. } => Some(&operation.operation_id),
            Self::Progressed { operation_id, .. } | Self::Transitioned { operation_id, .. } => {
                Some(operation_id)
            }
            Self::Dismissed { removed, .. } => Some(&removed.operation_id),
            Self::RetentionChanged { .. } | Self::TornDown { .. } => None,
        }
    }
}

/// Exact teardown result on the wire.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationTeardownOutcomeProjection {
    Completed {
        operation_id: OperationId,
        state: OperationStateProjection,
        previous_operation_revision: OperationRevision,
        committed_operation_revision: OperationRevision,
    },
    Transferred {
        operation_id: OperationId,
        previous_operation_revision: OperationRevision,
        target_authority: OperationAuthorityProjection,
    },
}

/// Stable checked mutation rejection category.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationRejectionCode {
    IncompatibleProtocol,
    InvalidCommand,
    AuthorityClosed,
    AuthorityMismatch,
    AuthorityEpochMismatch,
    CatalogueRevisionMismatch,
    DuplicateOperation,
    UnknownOperation,
    InvalidRetrySource,
    InvalidInitialState,
    InvalidTransition,
    OperationRevisionMismatch,
    ActiveLimitReached,
    ActiveLimitBelowCurrent,
    ProgressNotReportable,
    OverallProgressRegression,
    PhaseProgressRegression,
    DismissalRequiresTerminal,
    DuplicateTeardownResolution,
    MissingTeardownResolutions,
    UnexpectedTeardownResolution,
    InvalidTeardownTerminal,
    TeardownTransferToSelf,
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

/// Authoritative event summary kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationChangedKind {
    /// Management mutation committed.
    Mutation,
    /// Cancellation changed authority state.
    Cancellation,
}

/// Non-durable request-correlated authority invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationChangedEvent {
    /// Exact protocol line.
    pub protocol_version: OperationProtocolVersion,
    /// Request correlation identity.
    pub request_id: OperationRequestId,
    /// Live authority cursor.
    pub authority: OperationAuthorityProjection,
    /// Catalogue revision before commit.
    pub previous_catalogue_revision: OperationCatalogueRevision,
    /// Catalogue revision after commit.
    pub committed_catalogue_revision: OperationCatalogueRevision,
    /// Directly targeted operation when applicable.
    pub operation_id: Option<OperationId>,
    /// Change source.
    pub kind: OperationChangedKind,
}

impl OperationChangedEvent {
    /// Projects an event only for a committed management mutation.
    #[must_use]
    pub fn from_mutation(result: &OperationMutationResult) -> Option<Self> {
        let OperationMutationResult::Committed {
            request_id,
            snapshot,
            receipt,
        } = result
        else {
            return None;
        };
        Some(Self {
            protocol_version: OperationProtocolVersion::CURRENT,
            request_id: request_id.clone(),
            authority: snapshot.authority.clone(),
            previous_catalogue_revision: receipt.previous_catalogue_revision(),
            committed_catalogue_revision: receipt.committed_catalogue_revision(),
            operation_id: receipt.operation_id().cloned(),
            kind: OperationChangedKind::Mutation,
        })
    }

    /// Projects an event only when cancellation advanced catalogue state.
    #[must_use]
    pub fn from_cancellation(result: &OperationCancellationResult) -> Option<Self> {
        let OperationCancellationResult::Committed {
            request_id,
            snapshot,
            receipt,
            ..
        } = result
        else {
            return None;
        };
        if receipt.committed_catalogue_revision == receipt.previous_catalogue_revision {
            return None;
        }
        Some(Self {
            protocol_version: OperationProtocolVersion::CURRENT,
            request_id: request_id.clone(),
            authority: snapshot.authority.clone(),
            previous_catalogue_revision: receipt.previous_catalogue_revision,
            committed_catalogue_revision: receipt.committed_catalogue_revision,
            operation_id: Some(receipt.operation_id.clone()),
            kind: OperationChangedKind::Cancellation,
        })
    }
}

impl OperationCatalogue {
    /// Executes one strict management command and returns fresh authority.
    pub fn execute_protocol_mutation(
        &mut self,
        command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationProtocolProjectionError> {
        let request_id = command.request_id().clone();
        if command.protocol_version() != OperationProtocolVersion::CURRENT {
            return self.rejected_mutation(request_id, incompatible_protocol());
        }
        let result = execute_mutation(self, command);
        match result {
            Ok(receipt) => Ok(OperationMutationResult::Committed {
                request_id,
                snapshot: OperationSnapshot::from_catalogue(self)?,
                receipt: Box::new(receipt),
            }),
            Err(rejection) => self.rejected_mutation(request_id, rejection),
        }
    }

    /// Executes one strict cancellation command and returns fresh authority.
    pub fn execute_protocol_cancellation(
        &mut self,
        command: OperationCancellationCommand,
    ) -> Result<OperationCancellationResult, OperationProtocolProjectionError> {
        let request_id = command.request_id.clone();
        if command.protocol_version != OperationProtocolVersion::CURRENT {
            return self.rejected_cancellation(request_id, incompatible_protocol());
        }
        let request = match command.authority.into_cursor() {
            Ok(authority) => OperationCancellationRequest::new(
                authority,
                command.operation_id,
                command.expected_operation_revision,
            ),
            Err(error) => return self.rejected_cancellation(request_id, error.into()),
        };
        match self.request_cancellation(request) {
            Ok(receipt) => Ok(OperationCancellationResult::Committed {
                request_id,
                snapshot: OperationSnapshot::from_catalogue(self)?,
                receipt: OperationCancellationReceiptProjection {
                    operation_id: receipt.operation_id().clone(),
                    outcome: match receipt.outcome() {
                        OperationCancellationOutcome::Accepted => {
                            OperationCancellationOutcomeProjection::Accepted
                        }
                        OperationCancellationOutcome::AlreadyRequested => {
                            OperationCancellationOutcomeProjection::AlreadyRequested
                        }
                        OperationCancellationOutcome::Unsupported => {
                            OperationCancellationOutcomeProjection::Unsupported
                        }
                        OperationCancellationOutcome::Terminal => {
                            OperationCancellationOutcomeProjection::Terminal
                        }
                    },
                    previous_state: receipt.previous_state().into(),
                    committed_state: receipt.committed_state().into(),
                    previous_operation_revision: receipt.previous_operation_revision(),
                    committed_operation_revision: receipt.committed_operation_revision(),
                    previous_catalogue_revision: receipt.previous_catalogue_revision(),
                    committed_catalogue_revision: receipt.committed_catalogue_revision(),
                    evicted: receipt.evicted().iter().map(Into::into).collect(),
                },
                executor_dispatch: OperationExecutorDispatchProjection::NotRequired,
            }),
            Err(error) => self.rejected_cancellation(request_id, error.into()),
        }
    }

    fn rejected_mutation(
        &self,
        request_id: OperationRequestId,
        rejection: OperationRejection,
    ) -> Result<OperationMutationResult, OperationProtocolProjectionError> {
        Ok(OperationMutationResult::Rejected {
            request_id,
            snapshot: OperationSnapshot::from_catalogue(self)?,
            rejection,
        })
    }

    fn rejected_cancellation(
        &self,
        request_id: OperationRequestId,
        rejection: OperationRejection,
    ) -> Result<OperationCancellationResult, OperationProtocolProjectionError> {
        Ok(OperationCancellationResult::Rejected {
            request_id,
            snapshot: OperationSnapshot::from_catalogue(self)?,
            rejection,
        })
    }
}

fn execute_mutation(
    catalogue: &mut OperationCatalogue,
    command: OperationMutationCommand,
) -> Result<OperationMutationReceiptProjection, OperationRejection> {
    match command {
        OperationMutationCommand::Register {
            authority,
            expected_catalogue_revision,
            operation_id,
            kind_id,
            scope_id,
            label,
            initial_state,
            cancellation_support,
            retry_of,
            ..
        } => {
            let authority = authority.into_cursor().map_err(OperationRejection::from)?;
            let label = crate::OperationLabel::new(label).map_err(|error| {
                OperationRejection::from(OperationProtocolInputError::Label(error.to_string()))
            })?;
            let receipt = catalogue
                .register(OperationRegistration::new(
                    authority,
                    expected_catalogue_revision,
                    operation_id,
                    kind_id,
                    scope_id,
                    label,
                    initial_state.into(),
                    cancellation_support.into(),
                    retry_of,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Registered {
                operation: OperationEntryProjection::from_record(receipt.operation()),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
            })
        }
        OperationMutationCommand::Progress {
            authority,
            operation_id,
            expected_operation_revision,
            overall,
            phase,
            ..
        } => {
            let request = OperationProgressUpdate::new(
                authority.into_cursor().map_err(OperationRejection::from)?,
                operation_id,
                expected_operation_revision,
                overall.into_progress().map_err(OperationRejection::from)?,
                phase
                    .map(OperationPhaseProgressProjection::into_progress)
                    .transpose()
                    .map_err(OperationRejection::from)?,
            );
            let receipt = catalogue
                .update_progress(request)
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Progressed {
                operation_id: receipt.operation_id().clone(),
                previous_operation_revision: receipt.previous_operation_revision(),
                committed_operation_revision: receipt.committed_operation_revision(),
                previous_progress_sequence: receipt.previous_sequence().get(),
                committed_progress: OperationProgressProjection::from_progress(
                    receipt.committed_progress(),
                ),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
            })
        }
        OperationMutationCommand::Transition {
            authority,
            operation_id,
            expected_operation_revision,
            next_state,
            ..
        } => {
            let receipt = catalogue
                .transition(OperationTransition::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    operation_id,
                    expected_operation_revision,
                    next_state.into(),
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Transitioned {
                operation_id: receipt.operation_id().clone(),
                previous_state: receipt.previous_state().into(),
                committed_state: receipt.committed_state().into(),
                previous_operation_revision: receipt.previous_operation_revision(),
                committed_operation_revision: receipt.committed_operation_revision(),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
                evicted: receipt.evicted().iter().map(Into::into).collect(),
            })
        }
        OperationMutationCommand::ChangeRetention {
            authority,
            expected_catalogue_revision,
            limits,
            ..
        } => {
            let receipt = catalogue
                .change_retention(OperationRetentionChange::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    expected_catalogue_revision,
                    limits.into_limits().map_err(OperationRejection::from)?,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::RetentionChanged {
                previous_limits: OperationCatalogueLimitsProjection::from_limits(
                    receipt.previous_limits(),
                )
                .expect("validated limits project to u64"),
                committed_limits: OperationCatalogueLimitsProjection::from_limits(
                    receipt.committed_limits(),
                )
                .expect("validated limits project to u64"),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
                evicted: receipt.evicted().iter().map(Into::into).collect(),
                retained_terminal_encoded_weight: receipt.retained_terminal_encoded_weight(),
            })
        }
        OperationMutationCommand::Dismiss {
            authority,
            operation_id,
            expected_operation_revision,
            ..
        } => {
            let receipt = catalogue
                .dismiss_terminal(OperationDismissal::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    operation_id,
                    expected_operation_revision,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Dismissed {
                removed: receipt.removed().into(),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
            })
        }
        OperationMutationCommand::Teardown {
            authority,
            expected_catalogue_revision,
            resolutions,
            ..
        } => {
            let resolutions = resolutions
                .into_iter()
                .map(project_teardown_resolution)
                .collect::<Result<Vec<_>, _>>()?;
            let receipt = catalogue
                .teardown(OperationTeardown::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    expected_catalogue_revision,
                    resolutions,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::TornDown {
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
                outcomes: receipt
                    .outcomes()
                    .iter()
                    .map(project_teardown_outcome)
                    .collect(),
                evicted: receipt.evicted().iter().map(Into::into).collect(),
            })
        }
    }
}

fn project_teardown_resolution(
    value: OperationTeardownResolutionProjection,
) -> Result<OperationTeardownResolution, OperationRejection> {
    match value {
        OperationTeardownResolutionProjection::Complete {
            operation_id,
            expected_operation_revision,
            state,
        } => Ok(OperationTeardownResolution::new(
            operation_id,
            expected_operation_revision,
            OperationTeardownResolutionOutcome::Complete(state.into()),
        )),
        OperationTeardownResolutionProjection::Transfer {
            operation_id,
            expected_operation_revision,
            target_authority,
        } => Ok(OperationTeardownResolution::new(
            operation_id,
            expected_operation_revision,
            OperationTeardownResolutionOutcome::Transfer(
                target_authority
                    .into_cursor()
                    .map_err(OperationRejection::from)?,
            ),
        )),
    }
}

fn project_teardown_outcome(
    value: &OperationTeardownOutcome,
) -> OperationTeardownOutcomeProjection {
    match value {
        OperationTeardownOutcome::Completed {
            operation_id,
            state,
            previous_operation_revision,
            committed_operation_revision,
        } => OperationTeardownOutcomeProjection::Completed {
            operation_id: operation_id.clone(),
            state: (*state).into(),
            previous_operation_revision: *previous_operation_revision,
            committed_operation_revision: *committed_operation_revision,
        },
        OperationTeardownOutcome::Transferred {
            operation_id,
            previous_operation_revision,
            target_authority,
        } => OperationTeardownOutcomeProjection::Transferred {
            operation_id: operation_id.clone(),
            previous_operation_revision: *previous_operation_revision,
            target_authority: OperationAuthorityProjection::from_cursor(target_authority),
        },
    }
}

impl From<OperationCatalogueError> for OperationRejection {
    fn from(error: OperationCatalogueError) -> Self {
        let code = match error {
            OperationCatalogueError::AuthorityClosed => OperationRejectionCode::AuthorityClosed,
            OperationCatalogueError::AuthorityMismatch { .. } => {
                OperationRejectionCode::AuthorityMismatch
            }
            OperationCatalogueError::AuthorityEpochMismatch { .. } => {
                OperationRejectionCode::AuthorityEpochMismatch
            }
            OperationCatalogueError::CatalogueRevisionMismatch { .. } => {
                OperationRejectionCode::CatalogueRevisionMismatch
            }
            OperationCatalogueError::DuplicateOperation { .. } => {
                OperationRejectionCode::DuplicateOperation
            }
            OperationCatalogueError::UnknownOperation { .. } => {
                OperationRejectionCode::UnknownOperation
            }
            OperationCatalogueError::InvalidRetrySource { .. } => {
                OperationRejectionCode::InvalidRetrySource
            }
            OperationCatalogueError::InvalidInitialState { .. } => {
                OperationRejectionCode::InvalidInitialState
            }
            OperationCatalogueError::InvalidTransition { .. } => {
                OperationRejectionCode::InvalidTransition
            }
            OperationCatalogueError::OperationRevisionMismatch { .. } => {
                OperationRejectionCode::OperationRevisionMismatch
            }
            OperationCatalogueError::ActiveLimitReached { .. } => {
                OperationRejectionCode::ActiveLimitReached
            }
            OperationCatalogueError::ActiveLimitBelowCurrent { .. } => {
                OperationRejectionCode::ActiveLimitBelowCurrent
            }
            OperationCatalogueError::ProgressNotReportable { .. } => {
                OperationRejectionCode::ProgressNotReportable
            }
            OperationCatalogueError::OverallProgressRegression => {
                OperationRejectionCode::OverallProgressRegression
            }
            OperationCatalogueError::PhaseProgressRegression { .. } => {
                OperationRejectionCode::PhaseProgressRegression
            }
            OperationCatalogueError::DismissalRequiresTerminal { .. } => {
                OperationRejectionCode::DismissalRequiresTerminal
            }
            OperationCatalogueError::DuplicateTeardownResolution { .. } => {
                OperationRejectionCode::DuplicateTeardownResolution
            }
            OperationCatalogueError::MissingTeardownResolutions { .. } => {
                OperationRejectionCode::MissingTeardownResolutions
            }
            OperationCatalogueError::UnexpectedTeardownResolution { .. } => {
                OperationRejectionCode::UnexpectedTeardownResolution
            }
            OperationCatalogueError::InvalidTeardownTerminal { .. } => {
                OperationRejectionCode::InvalidTeardownTerminal
            }
            OperationCatalogueError::TeardownTransferToSelf { .. } => {
                OperationRejectionCode::TeardownTransferToSelf
            }
            OperationCatalogueError::TerminalEncodedWeightOverflow
            | OperationCatalogueError::TerminalEvictionCountOverflow
            | OperationCatalogueError::CatalogueRevisionOverflow
            | OperationCatalogueError::OperationRevisionOverflow
            | OperationCatalogueError::ProgressSequenceOverflow
            | OperationCatalogueError::SequenceOverflow => OperationRejectionCode::CapacityOverflow,
        };
        let refresh_required = matches!(
            code,
            OperationRejectionCode::AuthorityClosed
                | OperationRejectionCode::AuthorityMismatch
                | OperationRejectionCode::AuthorityEpochMismatch
                | OperationRejectionCode::CatalogueRevisionMismatch
                | OperationRejectionCode::UnknownOperation
                | OperationRejectionCode::OperationRevisionMismatch
        );
        Self {
            code,
            detail: error.to_string(),
            refresh_required,
        }
    }
}

impl From<OperationProtocolInputError> for OperationRejection {
    fn from(error: OperationProtocolInputError) -> Self {
        Self {
            code: OperationRejectionCode::InvalidCommand,
            detail: error.to_string(),
            refresh_required: false,
        }
    }
}

fn incompatible_protocol() -> OperationRejection {
    OperationRejection {
        code: OperationRejectionCode::IncompatibleProtocol,
        detail: format!("operation protocol version must be {OPERATION_PROTOCOL_VERSION}"),
        refresh_required: false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum OperationProtocolInputError {
    AuthorityEpoch,
    Progress(String),
    Phase(String),
    Label(String),
    Limits,
}

impl fmt::Display for OperationProtocolInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorityEpoch => {
                formatter.write_str("operation authority epoch must be nonzero")
            }
            Self::Progress(detail) | Self::Phase(detail) | Self::Label(detail) => {
                formatter.write_str(detail)
            }
            Self::Limits => formatter.write_str("operation catalogue limits are invalid"),
        }
    }
}

/// A bounded internal projection could not fit the protocol integer domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProtocolProjectionError(String);

impl fmt::Display for OperationProtocolProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for OperationProtocolProjectionError {}

fn project_usize(value: usize) -> Result<u64, OperationProtocolProjectionError> {
    u64::try_from(value)
        .map_err(|_| OperationProtocolProjectionError("operation count does not fit u64".into()))
}
