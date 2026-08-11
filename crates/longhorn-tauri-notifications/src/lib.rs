//! Narrow Tauri host assembly for an injected notification authority.

mod authority;

/// Tauri command assembly for the injected notification authority. Exported
/// as a module so the `#[tauri::command]` helper macros are path-importable
/// by consumers wiring `generate_handler`.
pub mod commands;
mod error;
mod handler;

pub use authority::NotificationHostAuthority;
pub use commands::*;
pub use error::{NotificationHostError, NotificationHostErrorCode};
pub use handler::NotificationHandlerAssembly;
