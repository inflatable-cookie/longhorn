//! Narrow Tauri host assembly for injected history authority.

mod authority;
mod commands;
mod error;
mod handler;

pub use authority::HistoryHostAuthority;
pub use commands::{
    HISTORY_CHANGED_EVENT, HistoryHostService, TauriHistoryState, history_changed_event,
    longhorn_history_navigate, longhorn_history_page, longhorn_history_snapshot,
    publish_history_changed,
};
pub use error::{HistoryHostError, HistoryHostErrorCode};
pub use handler::HistoryHandlerAssembly;
