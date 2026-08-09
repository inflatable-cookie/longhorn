//! Managed Tauri window mutation and honest convergence readback.

mod dispatch;
mod engine;
mod factory;
mod mutation;
mod operations;
mod readback;
mod receipt;
mod registry;

pub use dispatch::{TauriDispatchError, dispatch_tauri_window_apply};
pub use engine::{
    TauriApplyError, TauriApplyOutcome, execute_tauri_window_apply,
    execute_tauri_window_apply_in_place, tauri_deferred_settlement, tauri_host_capabilities,
};
pub use factory::{NoWindowFactory, TauriWindowFactory, WindowFactoryError};
pub use mutation::{NativeWindowMutationError, TauriWindowMutationBackend, WindowMutationBackend};
pub use readback::{ManagedDesktopReadback, TauriDesktopReadback};
pub use receipt::{
    ApplyConvergence, ApplyReadback, NativeWindowCall, ProgrammaticApplyEvidence,
    TauriApplyReceipt, WindowApplyAttempt, WindowApplyFailure, WindowApplyFailureKind,
    WindowApplyOutcome,
};
pub(crate) use registry::ManagedWindowRegistration;
pub use registry::{ManagedWindowRegistry, ManagedWindowRegistryError};
