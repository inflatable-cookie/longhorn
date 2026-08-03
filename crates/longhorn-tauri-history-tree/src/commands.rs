use std::sync::Arc;

use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkChangedEvent, ForkChangedKind,
    ForkHistoryProtocolVersion, ForkNavigationCommand, ForkNavigationResult, ForkPathPageCommand,
    ForkPathPageSnapshot, ForkSnapshot,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::ForkHistoryHostError;

/// Non-durable committed graph invalidation hint.
pub const FORK_HISTORY_CHANGED_EVENT: &str = "longhorn://history-tree/changed";

/// Object-safe fork-history surface retained in Tauri managed state.
pub trait ForkHistoryHostService: Send + Sync {
    /// Returns one caller-authorized linear-default summary.
    fn snapshot(&self, caller: &str) -> Result<ForkSnapshot, ForkHistoryHostError>;
    /// Returns one caller-authorized bounded path page.
    fn path(
        &self,
        caller: &str,
        command: ForkPathPageCommand,
    ) -> Result<ForkPathPageSnapshot, ForkHistoryHostError>;
    /// Returns one caller-authorized bounded branch page.
    fn branches(
        &self,
        caller: &str,
        command: ForkBranchPageCommand,
    ) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError>;
    /// Applies and commits one caller-authorized graph navigation.
    fn navigate(
        &self,
        caller: &str,
        command: ForkNavigationCommand,
    ) -> Result<ForkNavigationResult, ForkHistoryHostError>;
}

/// Type-erased graph host installed once in Tauri managed state.
pub struct TauriForkHistoryState {
    service: Arc<dyn ForkHistoryHostService>,
}

impl TauriForkHistoryState {
    /// Wraps one explicitly injected graph assembly.
    #[must_use]
    pub fn new(service: Arc<dyn ForkHistoryHostService>) -> Self {
        Self { service }
    }
}

/// Returns caller-authorized linear-default graph metadata.
#[tauri::command]
pub fn longhorn_history_tree_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
) -> Result<ForkSnapshot, ForkHistoryHostError> {
    state.service.snapshot(window.label())
}

/// Returns one caller-authorized bounded path page.
#[tauri::command]
pub fn longhorn_history_tree_path<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkPathPageCommand,
) -> Result<ForkPathPageSnapshot, ForkHistoryHostError> {
    state.service.path(window.label(), command)
}

/// Returns one caller-authorized bounded branch page.
#[tauri::command]
pub fn longhorn_history_tree_branches<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkBranchPageCommand,
) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError> {
    state.service.branches(window.label(), command)
}

/// Applies and commits one caller-authorized graph navigation.
#[tauri::command]
pub fn longhorn_history_tree_navigate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkNavigationCommand,
) -> Result<ForkNavigationResult, ForkHistoryHostError> {
    let result = state.service.navigate(window.label(), command)?;
    if let Some(event) = fork_history_changed_event(&result) {
        let _ = window.emit(FORK_HISTORY_CHANGED_EVENT, event);
    }
    Ok(result)
}

/// Publishes a trusted graph invalidation hint after an external commit.
pub fn publish_fork_history_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: ForkChangedEvent,
) -> Result<(), ForkHistoryHostError> {
    app.emit(FORK_HISTORY_CHANGED_EVENT, event)
        .map_err(|error| ForkHistoryHostError::event_publication(error.to_string()))
}

/// Projects a non-durable hint only for committed navigation.
#[must_use]
pub fn fork_history_changed_event(result: &ForkNavigationResult) -> Option<ForkChangedEvent> {
    let ForkNavigationResult::Committed { snapshot, receipt } = result else {
        return None;
    };
    Some(ForkChangedEvent {
        protocol_version: ForkHistoryProtocolVersion::CURRENT,
        authority_epoch: snapshot.authority_epoch,
        history_id: receipt.history_id.clone(),
        previous_revision: Some(receipt.previous_revision),
        committed_revision: receipt.committed_revision,
        kind: ForkChangedKind::Navigation,
    })
}
