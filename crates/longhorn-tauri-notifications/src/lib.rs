//! Narrow Tauri host assembly for an injected notification authority.

mod authority;

mod commands;
mod error;
mod handler;

pub use authority::NotificationHostAuthority;
pub use commands::{
    NOTIFICATION_CHANGED_EVENT, NotificationHostService, TauriNotificationState,
    longhorn_notifications_mutate, longhorn_notifications_snapshot,
    notification_mutation_changed_event, publish_notification_changed,
};
pub use error::{NotificationHostError, NotificationHostErrorCode};
pub use handler::NotificationHandlerAssembly;
