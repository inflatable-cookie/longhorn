//! Lifecycle capture, flush, action, and receipt types.

mod action;
mod report;

pub use action::{
    TauriWindowLifecycleAction, TauriWindowLifecycleError, TauriWindowLifecycleReceipt,
};
pub use report::{
    WindowLifecycleReport, WindowRevealReceipt, WindowRevealStatus, WindowShutdownReceipt,
};
