//! Narrow Tauri host assembly for the injected update controller.
//!
//! Recreated 2026-08-12. A crate of this name existed, was absorbed into
//! `longhorn-update` on 2026-08-09 when update execution became
//! host-independent, and its tauri dependency was deliberately removed. What
//! it held then was the installer; what it holds now is only the seam — the
//! commands, the capability split, and the invalidation hint.
//!
//! Nothing here decides anything. `UpdateController` in `longhorn-update`
//! holds the state and answers the commands; this crate carries them across
//! the Tauri boundary and back.

mod authority;
mod commands;
mod error;
mod handler;

pub use authority::UpdateHostAuthority;
pub use commands::{
    TauriUpdateState, UPDATE_CHANGED_EVENT, UpdateHostService, longhorn_update_check,
    longhorn_update_defer, longhorn_update_install, longhorn_update_select_channel,
    longhorn_update_snapshot, publish_update_changed, update_changed_event,
};
pub use error::{UpdateHostError, UpdateHostErrorCode};
pub use handler::UpdateHandlerAssembly;
