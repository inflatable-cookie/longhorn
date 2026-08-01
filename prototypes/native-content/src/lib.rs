//! Private pure coordination prototype for native content islands.

mod coordinator;
mod error;
mod geometry;
mod identity;
mod model;
mod plan;
mod proposal;
mod receipt;

pub use coordinator::{DesiredUpdateReceipt, NativeContentCoordinator, ObservationReceipt};
pub use error::{CoordinationError, ReceiptError, ViewportConversionError};
pub use geometry::viewport_to_physical;
pub use identity::{
    AttachGeneration, CounterOverflow, NativeContentFailureCode, NativeContentIdError,
    NativeContentIslandId, NativeContentKindId, NativeContentRevision, PlanStepId,
    VisibilityReasonId,
};
pub use model::{
    AttachmentLifecycle, DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility,
    DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent, InputRoutingMode,
    MechanismCapabilities, NativeContentMechanism, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, ObservedState,
};
pub use plan::{ApplyPlan, NativeContentOperation, PlannedOperation, plan_transition};
pub use proposal::{
    ContentSizeDecision, ContentSizeProposal, ContentSizeProposalReceipt, decide_content_size,
};
pub use receipt::{ApplyReceipt, OperationOutcome, StepExecution, StepReceipt};
