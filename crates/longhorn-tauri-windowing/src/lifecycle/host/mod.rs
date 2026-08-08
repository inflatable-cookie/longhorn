//! Tauri listener and I/O adapter over the pure lifecycle coordinator.

mod directives;
mod events;
mod install;
mod observers;
mod reveal;
mod shutdown;
mod state;
mod support;

pub use state::TauriWindowLifecycleHost;
pub(crate) use observers::coordination_error;
pub(crate) use state::{FlushDisposition, InstalledWindow, PendingFlush};
