//! Pure window placement, settled-state proposals, and desired/live diff planning.

mod config;
mod diff;
mod lifecycle;
mod outcome;
mod resolution;
mod restore;
mod settled;

pub use config::{PlacementPolicy, WindowPlacementConfig, WindowRole};
pub use diff::{
    ApplyFeedbackEvidence, ApplyGeneration, DesiredWindow, FocusPolicy, HostCapabilities,
    HostCapability, HostWindowHandle, HostWindowHandleError, LiveWindow, PlannedWindowOperation,
    ProtectedPrimaryPolicy, WindowDiffDiagnostic, WindowDiffError, WindowDiffInput,
    WindowDiffReceipt, WindowOperation, WindowOperationKind, plan_window_diff,
};
pub use lifecycle::{
    ApplyRegistrationOutcome, CaptureGeneration, CaptureReason, CapturedDisplayAssociation,
    CapturedDisplayEvidence, CapturedWindowPlacement, FlushReason, IgnoreReason, MonotonicMillis,
    ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget, WindowLifecycleCoordinator, WindowLifecycleDirective,
    WindowLifecycleDuration, WindowLifecycleError, WindowLifecycleEvent, WindowLifecycleEventKind,
    WindowLifecyclePolicy, WindowPlacementFlushCompletion, WindowPlacementFlushTicket,
    WindowPlacementSink,
};
pub use outcome::{
    PlacementReason, PlacementResolutionError, ResolvedWindowPlacement, UnavailablePlacementReason,
    UnavailableWindowPlacement, WindowPlacementResolution,
};
pub use resolution::resolve_window_placement;
pub use restore::{
    SavedDisplayAssociation, SavedDisplayEvidence, SavedWindowPlacement,
    resolve_saved_display_association, restore_window_placement,
};
pub use settled::{
    HomeDisplayAdoption, HomeDisplayChange, PlacementMemoryUpdate, SettledPlacementProposal,
    propose_settled_placement,
};
