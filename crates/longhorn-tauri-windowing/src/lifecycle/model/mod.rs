//! Lifecycle capture, flush, action, and receipt types.

pub use longhorn_windowing::{
    CapturedDisplayAssociation, CapturedDisplayEvidence, CapturedWindowPlacement,
    ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget,
};
mod action;
mod report;

pub use action::{
    TauriWindowLifecycleAction, TauriWindowLifecycleError, TauriWindowLifecycleReceipt,
};
pub use report::{
    WindowLifecycleReport, WindowRevealReceipt, WindowRevealStatus, WindowShutdownReceipt,
};
