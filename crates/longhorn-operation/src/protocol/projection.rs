//! Projected catalogue state, progress, and entry shapes.

use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationId, OperationKindId,
    OperationPhaseId, OperationRevision, OperationScopeId,
};
use serde::{Deserialize, Serialize};

use crate::{
    OperationAuthorityCursor, OperationAuthorityEpoch, OperationCancellationSupport,
    OperationCatalogueLimits, OperationNormalizedProgress, OperationOverallProgress,
    OperationPhaseLabel, OperationPhaseProgress, OperationProgress, OperationRecord,
    OperationState, OperationUnitProgress,
};

use super::*;

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
    pub(crate) fn from_cursor(cursor: &OperationAuthorityCursor) -> Self {
        Self {
            authority_id: cursor.authority_id().clone(),
            authority_epoch: cursor.authority_epoch().get(),
        }
    }

    pub(crate) fn into_cursor(
        self,
    ) -> Result<OperationAuthorityCursor, OperationProtocolInputError> {
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
    pub(crate) fn from_progress(progress: OperationOverallProgress) -> Self {
        match progress {
            OperationOverallProgress::Indeterminate => Self::Indeterminate,
            OperationOverallProgress::Units(value) => Self::Units {
                completed: value.completed(),
                total: value.total(),
            },
            OperationOverallProgress::Normalized(value) => Self::Normalized { value: value.get() },
        }
    }

    pub(crate) fn into_progress(
        self,
    ) -> Result<OperationOverallProgress, OperationProtocolInputError> {
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
    pub(crate) fn from_progress(progress: &OperationPhaseProgress) -> Self {
        Self {
            phase_id: progress.phase_id().clone(),
            label: progress.label().as_str().to_owned(),
            completed: progress.units().completed(),
            total: progress.units().total(),
        }
    }

    pub(crate) fn into_progress(
        self,
    ) -> Result<OperationPhaseProgress, OperationProtocolInputError> {
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
    pub(crate) fn from_progress(progress: &OperationProgress) -> Self {
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
    pub(crate) fn from_record(record: &OperationRecord) -> Self {
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
    pub(crate) fn from_limits(
        limits: OperationCatalogueLimits,
    ) -> Result<Self, OperationProtocolProjectionError> {
        Ok(Self {
            maximum_active_operations: project_usize(limits.maximum_active_operations())?,
            maximum_terminal_operations: project_usize(limits.maximum_terminal_operations())?,
            maximum_terminal_encoded_weight: limits.maximum_terminal_encoded_weight(),
        })
    }

    pub(crate) fn into_limits(
        self,
    ) -> Result<OperationCatalogueLimits, OperationProtocolInputError> {
        let active = usize::try_from(self.maximum_active_operations)
            .map_err(|_| OperationProtocolInputError::Limits)?;
        let terminal = usize::try_from(self.maximum_terminal_operations)
            .map_err(|_| OperationProtocolInputError::Limits)?;
        OperationCatalogueLimits::new(active, terminal, self.maximum_terminal_encoded_weight)
            .map_err(|_| OperationProtocolInputError::Limits)
    }
}
