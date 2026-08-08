//! Tauri listener and I/O adapter over the pure lifecycle coordinator.

mod directives;
mod events;
mod install;
mod observers;
mod reveal;
mod shutdown;
mod state;
mod support;

pub(crate) use observers::coordination_error;
pub use state::TauriWindowLifecycleHost;
pub(crate) use state::{FlushDisposition, InstalledWindow, PendingFlush};
