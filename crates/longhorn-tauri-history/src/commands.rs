use std::sync::Arc;

use longhorn_history::{
    HistoryChangedEvent, HistoryChangedKind, HistoryNavigationCommand, HistoryNavigationResult,
    HistoryPageCommand, HistoryPageSnapshot, HistoryProtocolVersion, HistorySnapshot,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::HistoryHostError;

/// Non-durable committed-history invalidation hint.
pub const HISTORY_CHANGED_EVENT: &str = "longhorn://history/changed";

/// Object-safe history surface retained in Tauri managed state.
pub trait HistoryHostService: Send + Sync {
    /// Returns one caller-authorized metadata snapshot.
    fn snapshot(&self, caller: &str) -> Result<HistorySnapshot, HistoryHostError>;

    /// Returns one caller-authorized bounded metadata page.
    fn page(
        &self,
        caller: &str,
        command: HistoryPageCommand,
    ) -> Result<HistoryPageSnapshot, HistoryHostError>;

    /// Applies and commits one caller-authorized checked navigation.
    fn navigate(
        &self,
        caller: &str,
        command: HistoryNavigationCommand,
    ) -> Result<HistoryNavigationResult, HistoryHostError>;
}

/// Type-erased history host installed once in Tauri managed state.
pub struct TauriHistoryState {
    service: Arc<dyn HistoryHostService>,
}

impl TauriHistoryState {
    /// Wraps one explicitly injected history assembly.
    #[must_use]
    pub fn new(service: Arc<dyn HistoryHostService>) -> Self {
        Self { service }
    }
}

/// Returns the caller-authorized payload-free history snapshot.
#[tauri::command]
pub fn longhorn_history_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriHistoryState>,
) -> Result<HistorySnapshot, HistoryHostError> {
    state.service.snapshot(window.label())
}

/// Returns one caller-authorized bounded metadata page.
#[tauri::command]
pub fn longhorn_history_page<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriHistoryState>,
    command: HistoryPageCommand,
) -> Result<HistoryPageSnapshot, HistoryHostError> {
    state.service.page(window.label(), command)
}

/// Applies and commits one caller-authorized checked navigation.
#[tauri::command]
pub fn longhorn_history_navigate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriHistoryState>,
    command: HistoryNavigationCommand,
) -> Result<HistoryNavigationResult, HistoryHostError> {
    let result = state.service.navigate(window.label(), command)?;
    if let Some(event) = history_changed_event(&result) {
        // This event is an invalidation hint, not the durable outcome. Once the
        // authority commits navigation, publication failure must not disguise
        // the committed result as a retryable command failure.
        let _ = window.emit(HISTORY_CHANGED_EVENT, event);
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
