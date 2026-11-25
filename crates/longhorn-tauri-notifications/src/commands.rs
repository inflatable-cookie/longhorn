use std::sync::Arc;

use longhorn_notifications::{
    NotificationChangedEvent, NotificationMutationCommand, NotificationMutationResult,
    NotificationSnapshotQuery, NotificationSnapshotResponse,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{NotificationHandlerAssembly, NotificationHostAuthority, NotificationHostError};

/// Non-durable committed-notification invalidation hint.
pub const NOTIFICATION_CHANGED_EVENT: &str = "longhorn://notifications/changed";

/// Type-erased notification host installed once in Tauri managed state.
pub struct TauriNotificationState {
    assembly: Arc<NotificationHandlerAssembly<dyn NotificationHostAuthority>>,
}

impl TauriNotificationState {
    /// Wraps one explicitly injected notification assembly.
    #[must_use]
    pub fn new(assembly: Arc<NotificationHandlerAssembly<dyn NotificationHostAuthority>>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriNotificationState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &NotificationHandlerAssembly<dyn NotificationHostAuthority>,
        &str,
    ) -> Result<T, NotificationHostError>
    + Send
    + 'static,
) -> Result<T, NotificationHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| NotificationHostError::authority_state_unavailable())?
}

/// Returns one caller-authorized notification snapshot page.
#[tauri::command]
pub async fn longhorn_notifications_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriNotificationState>,
    query: NotificationSnapshotQuery,
) -> Result<NotificationSnapshotResponse, NotificationHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.snapshot(label, query)
    })
    .await
}

/// Applies one caller-authorized notification mutation and broadcasts its commit hint.
#[tauri::command]
pub async fn longhorn_notifications_mutate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriNotificationState>,
    command: NotificationMutationCommand,
) -> Result<NotificationMutationResult, NotificationHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.mutate(label, command)
    })
    .await?;
    if let Some(event) = notification_mutation_changed_event(&result)
        && let Err(error) = window.emit(NOTIFICATION_CHANGED_EVENT, event)
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
