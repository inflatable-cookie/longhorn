//! Narrow Tauri host assembly for injected command catalogue and keymap authority.

mod authority;
mod commands;
mod error;
mod handler;

pub use authority::CommandHostAuthority;
pub use commands::{
    COMMAND_CATALOGUE_CHANGED_EVENT, COMMAND_KEYMAP_CHANGED_EVENT, CommandHostService,
    TauriCommandState, keymap_changed_event, longhorn_command_catalogue, longhorn_command_keymap,
    longhorn_command_keymap_commit, longhorn_command_keymap_preview, longhorn_command_keymap_reset,
    publish_catalogue_changed, publish_keymap_changed,
};
pub use error::{CommandHostError, CommandHostErrorCode};
pub use handler::CommandHandlerAssembly;
