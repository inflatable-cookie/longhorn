//! Lifecycle capture, flush, action, and receipt types.

mod action;
mod capture;
mod flush;
mod report;

pub use action::{TauriWindowLifecycleAction, TauriWindowLifecycleError, TauriWindowLifecycleReceipt};
pub use capture::{CapturedDisplayAssociation, CapturedDisplayEvidence, CapturedWindowPlacement};
pub use flush::{
    ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget,
};
pub use report::{WindowLifecycleReport, WindowRevealReceipt, WindowRevealStatus, WindowShutdownReceipt};
