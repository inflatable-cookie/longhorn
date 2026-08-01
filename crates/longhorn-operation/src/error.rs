use std::{error::Error, fmt};

use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationId, OperationPhaseId,
    OperationRevision,
};

use crate::{OperationAuthorityEpoch, OperationState};

/// Checked operation catalogue mutation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationCatalogueError {
    /// Controlled teardown already closed the authority.
    AuthorityClosed,
    /// The request named another authority.
    AuthorityMismatch {
        /// Authority owned by the catalogue.
        expected: OperationAuthorityId,
        /// Authority supplied by the request.
        actual: OperationAuthorityId,
    },
    /// The request used an obsolete or foreign authority epoch.
    AuthorityEpochMismatch {
        /// Live authority epoch.
        expected: OperationAuthorityEpoch,
        /// Supplied authority epoch.
        actual: OperationAuthorityEpoch,
    },
    /// A command did not target the current catalogue revision.
    CatalogueRevisionMismatch {
        /// Current catalogue revision.
        expected: OperationCatalogueRevision,
        /// Supplied catalogue revision.
        actual: OperationCatalogueRevision,
    },
    /// A registered operation already uses this id.
    DuplicateOperation {
        /// Duplicate identity.
        operation_id: OperationId,
    },
    /// No retained operation uses this id.
    UnknownOperation {
        /// Missing identity.
        operation_id: OperationId,
    },
    /// A retry source is missing or is not terminal.
    InvalidRetrySource {
        /// Requested retry source.
        operation_id: OperationId,
        /// Retained non-terminal state, or `None` when absent.
        state: Option<OperationState>,
    },
    /// Registration selected a non-initial state.
    InvalidInitialState {
        /// Supplied state.
        state: OperationState,
    },
    /// The lifecycle edge is not legal.
    InvalidTransition {
        /// Current state.
        current: OperationState,
        /// Requested state.
        next: OperationState,
    },
    /// The request did not target the current operation revision.
    OperationRevisionMismatch {
        /// Current revision.
        expected: OperationRevision,
        /// Supplied revision.
        actual: OperationRevision,
    },
    /// The active-operation bound has been reached.
    ActiveLimitReached {
        /// Configured active-operation limit.
        maximum: usize,
    },
    /// New limits cannot retain all active operations.
    ActiveLimitBelowCurrent {
        /// Current active count.
        current: usize,
        /// Requested maximum.
        maximum: usize,
    },
    /// Progress arrived after a terminal outcome.
    ProgressNotReportable {
        /// Current terminal state.
        state: OperationState,
    },
    /// Overall progress would move backwards or become indeterminate again.
    OverallProgressRegression,
    /// The same phase identity would move backwards.
    PhaseProgressRegression {
        /// Phase whose units regressed.
        phase_id: OperationPhaseId,
    },
    /// Only terminal records may be dismissed.
    DismissalRequiresTerminal {
        /// Current active state.
        state: OperationState,
    },
    /// Teardown supplied more than one resolution for an operation.
    DuplicateTeardownResolution {
        /// Repeated operation identity.
        operation_id: OperationId,
    },
    /// Teardown omitted one or more active operations.
    MissingTeardownResolutions {
        /// Active operations omitted by the command.
        operation_ids: Vec<OperationId>,
    },
    /// Teardown supplied a resolution for a terminal or unknown operation.
    UnexpectedTeardownResolution {
        /// Terminal or unknown operation identity.
        operation_id: OperationId,
    },
    /// Teardown completion selected a non-terminal state.
    InvalidTeardownTerminal {
        /// Supplied non-terminal state.
        state: OperationState,
    },
    /// Teardown cannot transfer back to the authority being closed.
    TeardownTransferToSelf {
        /// Operation assigned back to this authority cursor.
        operation_id: OperationId,
    },
    /// Terminal metadata weight addition overflowed.
    TerminalEncodedWeightOverflow,
    /// Cumulative terminal eviction evidence overflowed.
    TerminalEvictionCountOverflow,
    /// The catalogue revision cannot advance without wrapping.
    CatalogueRevisionOverflow,
    /// One operation revision cannot advance without wrapping.
    OperationRevisionOverflow,
    /// One progress sequence cannot advance without wrapping.
    ProgressSequenceOverflow,
    /// The insertion sequence cannot advance without wrapping.
    SequenceOverflow,
}

impl fmt::Display for OperationCatalogueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorityClosed => formatter.write_str("operation authority is closed"),
            Self::AuthorityMismatch { expected, actual } => write!(
                formatter,
                "operation authority mismatch: expected {expected}, got {actual}"
            ),
            Self::AuthorityEpochMismatch { expected, actual } => write!(
                formatter,
                "operation authority epoch mismatch: expected {}, got {}",
                expected.get(),
                actual.get()
            ),
            Self::CatalogueRevisionMismatch { expected, actual } => write!(
                formatter,
                "operation catalogue revision mismatch: expected {}, got {}",
                expected.get(),
                actual.get()
            ),
            Self::DuplicateOperation { operation_id } => {
                write!(formatter, "operation {operation_id} is already registered")
            }
            Self::UnknownOperation { operation_id } => {
                write!(formatter, "operation {operation_id} is not retained")
            }
            Self::InvalidRetrySource {
                operation_id,
                state,
            } => write!(
                formatter,
                "retry source {operation_id} is not a retained terminal operation (state {state:?})"
            ),
            Self::InvalidInitialState { state } => write!(
                formatter,
                "operation cannot be registered in state {state:?}"
            ),
            Self::InvalidTransition { current, next } => write!(
                formatter,
                "operation cannot transition from {current:?} to {next:?}"
            ),
            Self::OperationRevisionMismatch { expected, actual } => write!(
                formatter,
                "operation revision mismatch: expected {}, got {}",
                expected.get(),
                actual.get()
            ),
            Self::ActiveLimitReached { maximum } => write!(
                formatter,
                "operation catalogue reached its active-operation limit of {maximum}"
            ),
            Self::ActiveLimitBelowCurrent { current, maximum } => write!(
                formatter,
                "active-operation limit {maximum} cannot retain {current} active operations"
            ),
            Self::ProgressNotReportable { state } => write!(
                formatter,
                "operation in state {state:?} cannot report progress"
            ),
            Self::OverallProgressRegression => {
                formatter.write_str("overall operation progress cannot regress")
            }
            Self::PhaseProgressRegression { phase_id } => write!(
                formatter,
                "operation phase {phase_id} progress cannot regress"
            ),
            Self::DismissalRequiresTerminal { state } => write!(
                formatter,
                "operation in state {state:?} cannot be dismissed"
            ),
            Self::DuplicateTeardownResolution { operation_id } => write!(
                formatter,
                "teardown resolved operation {operation_id} more than once"
            ),
            Self::MissingTeardownResolutions { operation_ids } => write!(
                formatter,
                "teardown omitted {} active operation(s)",
                operation_ids.len()
            ),
            Self::UnexpectedTeardownResolution { operation_id } => write!(
                formatter,
                "teardown cannot resolve operation {operation_id}"
            ),
            Self::InvalidTeardownTerminal { state } => write!(
                formatter,
                "teardown completion state {state:?} is not terminal"
            ),
            Self::TeardownTransferToSelf { operation_id } => write!(
                formatter,
                "teardown cannot transfer operation {operation_id} to the closing authority"
            ),
            Self::TerminalEncodedWeightOverflow => {
                formatter.write_str("terminal operation metadata weight overflowed")
            }
            Self::TerminalEvictionCountOverflow => {
                formatter.write_str("terminal operation eviction count overflowed")
            }
            Self::CatalogueRevisionOverflow => {
                formatter.write_str("operation catalogue revision cannot advance beyond u64::MAX")
            }
            Self::OperationRevisionOverflow => {
                formatter.write_str("operation revision cannot advance beyond u64::MAX")
            }
            Self::ProgressSequenceOverflow => {
                formatter.write_str("operation progress sequence cannot advance beyond u64::MAX")
            }
            Self::SequenceOverflow => {
                formatter.write_str("operation sequence cannot advance beyond u64::MAX")
            }
        }
    }
}

impl Error for OperationCatalogueError {}
