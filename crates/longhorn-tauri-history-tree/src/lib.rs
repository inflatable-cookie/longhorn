//! Narrow Tauri host assembly for injected fork-history authority.

mod authority;
mod commands;
mod error;
mod handler;

pub use authority::ForkHistoryHostAuthority;
pub use commands::{
    FORK_HISTORY_CHANGED_EVENT, ForkHistoryHostService, TauriForkHistoryState,
    fork_history_changed_event, fork_retention_changed_event, longhorn_history_tree_branches,
    longhorn_history_tree_continuations, longhorn_history_tree_delete_continuation,
    longhorn_history_tree_navigate, longhorn_history_tree_path, longhorn_history_tree_prune,
    longhorn_history_tree_snapshot, publish_fork_history_changed,
};
pub use error::{ForkHistoryHostError, ForkHistoryHostErrorCode};
pub use handler::ForkHistoryHandlerAssembly;
