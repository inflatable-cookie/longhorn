//! Narrow Tauri handler assembly for injected settings authorities.

mod authority;
mod commands;
mod error;
mod handler;

pub use authority::SettingsAuthority;
pub use commands::{
    SETTINGS_REGISTRY_CHANGED_EVENT, SETTINGS_SCOPE_CHANGED_EVENT, SettingsCommandService,
    TauriSettingsState, longhorn_settings_apply, longhorn_settings_load,
    longhorn_settings_registry, longhorn_settings_reset, mutation_changed_event,
    publish_registry_changed, publish_scope_changed,
};
pub use error::{SettingsHostError, SettingsHostErrorCode};
pub use handler::SettingsHandlerAssembly;
