//! Pure asynchronous-operation lifecycle authority.
//!
//! Longhorn owns structural identity, revisions, lifecycle transitions, and
//! payload-free projections. Consumers own admission, scheduling, execution,
//! product progress, outcomes, artifacts, persistence, and recovery.

mod cancellation;
mod catalogue;
mod error;
mod identity;
mod limits;
mod model;
mod progress;
mod protocol;
mod retention;
mod teardown;

pub use cancellation::{
    OperationCancellationOutcome, OperationCancellationReceipt, OperationCancellationRequest,
    OperationCancellationSupport,
};
pub use catalogue::OperationCatalogue;
pub use error::OperationCatalogueError;
pub use identity::{
    OperationAuthorityEpoch, OperationAuthorityEpochError, OperationProgressSequence,
    OperationProgressSequenceOverflow, OperationSequence, OperationSequenceOverflow,
    OperationSequenceZero,
};
pub use limits::{
    MAXIMUM_OPERATION_ENCODED_WEIGHT, MAXIMUM_OPERATION_LABEL_BYTES,
    MAXIMUM_OPERATION_PHASE_LABEL_BYTES, MAXIMUM_RETAINED_OPERATIONS, OperationCatalogueLimits,
    OperationCatalogueLimitsError, OperationLabel, OperationLabelError, OperationPhaseLabel,
    OperationPhaseLabelError,
};
pub use model::{
    OperationAuthorityCursor, OperationCatalogueProjection, OperationRecord, OperationRegistration,
    OperationRegistrationReceipt, OperationState, OperationTransition, OperationTransitionReceipt,
};
pub use progress::{
    OperationNormalizedProgress, OperationOverallProgress, OperationPhaseProgress,
    OperationProgress, OperationProgressReceipt, OperationProgressUpdate,
    OperationProgressValueError, OperationUnitProgress,
};
pub use protocol::*;
pub use retention::{
    OperationDismissal, OperationDismissalReceipt, OperationRemoval, OperationRemovalReason,
    OperationRetentionChange, OperationRetentionReceipt,
};
pub use teardown::{
    OperationTeardown, OperationTeardownOutcome, OperationTeardownReceipt,
    OperationTeardownResolution, OperationTeardownResolutionOutcome,
};
