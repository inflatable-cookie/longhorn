use std::sync::Arc;

use longhorn_history::{
    HistoryChangedEvent, HistoryChangedKind, HistoryNavigationCommand, HistoryNavigationResult,
    HistoryPageCommand, HistoryPageSnapshot, HistoryProtocolVersion, HistorySnapshot,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{HistoryHandlerAssembly, HistoryHostAuthority, HistoryHostError};

/// Non-durable committed-history invalidation hint.
pub const HISTORY_CHANGED_EVENT: &str = "longhorn://history/changed";

/// Type-erased history host installed once in Tauri managed state.
pub struct TauriHistoryState {
    assembly: Arc<HistoryHandlerAssembly<dyn HistoryHostAuthority>>,
}

impl TauriHistoryState {
    /// Wraps one explicitly injected history assembly.
    #[must_use]
    pub fn new(assembly: Arc<HistoryHandlerAssembly<dyn HistoryHostAuthority>>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriHistoryState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &HistoryHandlerAssembly<dyn HistoryHostAuthority>,
        &str,
    ) -> Result<T, HistoryHostError>
    + Send
    + 'static,
) -> Result<T, HistoryHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| HistoryHostError::state_unavailable())?
}

/// Returns the caller-authorized payload-free history snapshot.
#[tauri::command]
pub async fn longhorn_history_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriHistoryState>,
) -> Result<HistorySnapshot, HistoryHostError> {
    run_blocking(state, &window, |assembly, label| assembly.snapshot(label)).await
}

/// Returns one caller-authorized bounded metadata page.
#[tauri::command]
pub async fn longhorn_history_page<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriHistoryState>,
    command: HistoryPageCommand,
) -> Result<HistoryPageSnapshot, HistoryHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.page(label, command)
    })
    .await
}

/// Applies and commits one caller-authorized checked navigation.
#[tauri::command]
pub async fn longhorn_history_navigate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriHistoryState>,
    command: HistoryNavigationCommand,
) -> Result<HistoryNavigationResult, HistoryHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.navigate(label, command)
    })
    .await?;
    if let Some(event) = history_changed_event(&result) {
        // This event is an invalidation hint, not the durable outcome. Once the
        // authority commits navigation, publication failure must not disguise
        // the committed result as a retryable command failure.
        if let Err(error) = window.emit(HISTORY_CHANGED_EVENT, event) {
            longhorn_core::report_best_effort_failure("history.changed-emit", error);
        }
    }
    Ok(result)
}

/// Publishes a trusted invalidation hint after an external committed transition.
pub fn publish_history_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: HistoryChangedEvent,
) -> Result<(), HistoryHostError> {
    app.emit(HISTORY_CHANGED_EVENT, event)
        .map_err(|error| HistoryHostError::event_publication(error.to_string()))
}

/// Projects a non-durable invalidation hint only for committed navigation.
#[must_use]
pub fn history_changed_event(result: &HistoryNavigationResult) -> Option<HistoryChangedEvent> {
    let HistoryNavigationResult::Committed { snapshot, receipt } = result else {
        return None;
    };
    Some(HistoryChangedEvent {
        protocol_version: HistoryProtocolVersion::CURRENT,
        authority_epoch: snapshot.authority_epoch,
        history_id: receipt.history_id.clone(),
        previous_revision: Some(receipt.previous_revision),
        committed_revision: receipt.committed_revision,
        kind: HistoryChangedKind::Navigation,
    })
}
