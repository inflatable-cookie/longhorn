mod error;
mod identity;
mod model;
mod operation;
mod planner;
mod receipt;

pub use error::WindowDiffError;
pub use identity::{ApplyGeneration, HostWindowHandle, HostWindowHandleError};
pub use model::{DesiredWindow, FocusPolicy, LiveWindow, ProtectedPrimaryPolicy, WindowDiffInput};
pub use operation::{HostCapabilities, HostCapability, WindowOperation, WindowOperationKind};
pub use planner::plan_window_diff;
pub use receipt::{
    ApplyFeedbackEvidence, DeferredSettlement, PlannedWindowOperation, WindowDiffDiagnostic,
    WindowDiffReceipt,
};
