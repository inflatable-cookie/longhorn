//! Strict payload-free renderer and transport protocol.

mod error;
mod event;
mod execute;
mod mutation;
mod projection;
mod snapshot;
mod version;

pub use error::OperationProtocolProjectionError;
pub(crate) use error::{OperationProtocolInputError, incompatible_protocol, project_usize};
pub use event::{OperationChangedEvent, OperationChangedKind};
pub use mutation::{
    OperationCancellationCommand, OperationCancellationOutcomeProjection,
    OperationCancellationReceiptProjection, OperationCancellationResult,
    OperationExecutorDispatchProjection, OperationMutationCommand,
    OperationMutationReceiptProjection, OperationMutationResult, OperationRejection,
    OperationRejectionCode, OperationRemovalProjection, OperationRemovalReasonProjection,
    OperationTeardownOutcomeProjection,
};
pub use projection::{
    OperationAuthorityProjection, OperationCancellationSupportProjection,
    OperationCatalogueLimitsProjection, OperationEntryProjection,
    OperationOverallProgressProjection, OperationPhaseProgressProjection,
    OperationProgressProjection, OperationStateProjection,
};
pub use snapshot::{
    OperationSnapshot, OperationSnapshotQuery, OperationSnapshotResponse,
    OperationTeardownResolutionProjection,
};
pub use version::{OPERATION_PROTOCOL_VERSION, OperationProtocolVersion};
