//! Pure, framework-independent coordination for native content islands.
//!
//! The crate owns logical island identity, generation-bound host attachment,
//! desired and observed state, immutable apply plans, and exact receipts. It
//! never owns platform handles, native API calls, product payloads, or outer
//! window placement.

mod coordinator;
mod error;
mod geometry;
mod identity;
mod model;
mod plan;
mod proposal;
mod protocol;
mod receipt;

pub use coordinator::{
    DesiredUpdateReceipt, HostDestroyOutcome, HostDestroyReceipt, NativeContentCoordinator,
    ObservationReceipt,
};
pub use error::{CoordinationError, ReceiptError, ViewportConversionError};
pub use geometry::viewport_to_physical;
pub use identity::{AttachGeneration, CounterOverflow, PlanStepId, PositiveCounterError};
pub use longhorn_core::{
    NativeContentFailureCode, NativeContentIslandId, NativeContentKindId, NativeContentRequestId,
    NativeContentRevision, OpaqueIdError, VisibilityReasonId,
};
pub use model::{
    AttachmentLifecycle, DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility,
    DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent, InputRoutingMode,
    MechanismCapabilities, NativeContentMechanism, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, ObservedState,
};
pub use plan::{ApplyPlan, MAX_APPLY_PLAN_STEPS, NativeContentOperation, PlannedOperation};
pub use proposal::{ContentSizeDecision, ContentSizeProposal, ContentSizeProposalReceipt};
pub use protocol::{
    NATIVE_CONTENT_PROTOCOL_VERSION, NativeContentAuthorityEpoch, NativeContentChangeProjection,
    NativeContentChangedEvent, NativeContentClientEpoch, NativeContentConnectRequest,
    NativeContentConnectResult, NativeContentContentSizeDecisionRequest,
    NativeContentContentSizeDecisionResult, NativeContentCursor, NativeContentDesiredUpdateRequest,
    NativeContentDesiredUpdateResult, NativeContentFailurePhase, NativeContentProtocolCounterError,
    NativeContentProtocolHost, NativeContentProtocolRejection, NativeContentProtocolVersion,
    NativeContentRejectionCode, NativeContentRetryClass, NativeContentSnapshot,
    NativeContentSnapshotRequest, NativeContentSnapshotResult,
};
pub use receipt::{ApplyReceipt, OperationOutcome, StepExecution, StepReceipt};
