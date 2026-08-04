use std::sync::Arc;

use longhorn_notifications::{
    NotificationChangedEvent, NotificationMutationCommand, NotificationMutationResult,
    NotificationSnapshotQuery, NotificationSnapshotResponse,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow};

use crate::NotificationHostError;

/// Non-durable committed-notification invalidation hint.
pub const NOTIFICATION_CHANGED_EVENT: &str = "longhorn://notifications/changed";

/// Object-safe notification surface retained in Tauri managed state.
pub trait NotificationHostService: Send + Sync {
    /// Returns a caller-authorized snapshot page.
    fn snapshot(
        &self,
        caller: &str,
        query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationHostError>;
    /// Applies a caller-authorized mutation.
    fn mutate(
        &self,
        caller: &str,
        command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationHostError>;
}

/// Type-erased notification host installed once in Tauri managed state.
pub struct TauriNotificationState {
    service: Arc<dyn NotificationHostService>,
}

impl TauriNotificationState {
    /// Wraps one explicitly injected notification assembly.
    #[must_use]
    pub fn new(service: Arc<dyn NotificationHostService>) -> Self {
        Self { service }
    }
}

/// Returns one caller-authorized notification snapshot page.
#[tauri::command]
pub fn longhorn_notifications_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriNotificationState>,
    query: NotificationSnapshotQuery,
) -> Result<NotificationSnapshotResponse, NotificationHostError> {
    state.service.snapshot(window.label(), query)
}

/// Applies one caller-authorized notification mutation and broadcasts its hint.
#[tauri::command]
pub fn longhorn_notifications_mutate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriNotificationState>,
    command: NotificationMutationCommand,
) -> Result<NotificationMutationResult, NotificationHostError> {
    let result = state.service.mutate(window.label(), command)?;
    if let Some(event) = notification_mutation_changed_event(&result)
        && let Err(error) = window.app_handle().emit(NOTIFICATION_CHANGED_EVENT, event)
    {
        longhorn_core::report_best_effort_failure("notifications.changed-emit", error);
    }
    Ok(result)
}

/// Publishes a trusted invalidation hint after an external commit.
pub fn publish_notification_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: NotificationChangedEvent,
) -> Result<(), NotificationHostError> {
    app.emit(NOTIFICATION_CHANGED_EVENT, event)
        .map_err(|error| NotificationHostError::event_publication(error.to_string()))
}

/// Projects an invalidation hint only after a revision-advancing commit.
#[must_use]
pub fn notification_mutation_changed_event(
    result: &NotificationMutationResult,
) -> Option<NotificationChangedEvent> {
    NotificationChangedEvent::from_mutation(result)
}
