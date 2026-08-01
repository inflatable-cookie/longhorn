use std::sync::Arc;

use longhorn_operation::{
    OperationCancellationCommand, OperationCancellationResult, OperationChangedEvent,
    OperationMutationCommand, OperationMutationResult, OperationSnapshotQuery,
    OperationSnapshotResponse,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::OperationHostError;

/// Non-durable committed-operation invalidation hint.
pub const OPERATION_CHANGED_EVENT: &str = "longhorn://operation/changed";

/// Object-safe operation surface retained in Tauri managed state.
pub trait OperationHostService: Send + Sync {
    /// Returns a caller-authorized snapshot.
    fn snapshot(
        &self,
        caller: &str,
        query: OperationSnapshotQuery,
    ) -> Result<OperationSnapshotResponse, OperationHostError>;

    /// Applies a caller-authorized management mutation.
    fn mutate(
        &self,
        caller: &str,
        command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationHostError>;

    /// Admits cancellation and dispatches through the injected executor.
    fn cancel(
        &self,
        caller: &str,
        command: OperationCancellationCommand,
    ) -> Result<OperationCancellationResult, OperationHostError>;
}

/// Type-erased operation host installed once in Tauri managed state.
pub struct TauriOperationState {
    service: Arc<dyn OperationHostService>,
}

impl TauriOperationState {
    /// Wraps one explicitly injected operation assembly.
    #[must_use]
    pub fn new(service: Arc<dyn OperationHostService>) -> Self {
        Self { service }
    }
}

/// Returns the caller-authorized operation snapshot.
#[tauri::command]
pub fn longhorn_operation_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriOperationState>,
    query: OperationSnapshotQuery,
) -> Result<OperationSnapshotResponse, OperationHostError> {
    state.service.snapshot(window.label(), query)
}

/// Applies one caller-authorized management mutation.
#[tauri::command]
pub fn longhorn_operation_mutate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriOperationState>,
    command: OperationMutationCommand,
) -> Result<OperationMutationResult, OperationHostError> {
    let result = state.service.mutate(window.label(), command)?;
    if let Some(event) = operation_mutation_changed_event(&result) {
        let _ = window.emit(OPERATION_CHANGED_EVENT, event);
    }
    Ok(result)
}

/// Admits one caller-authorized cancellation request.
#[tauri::command]
pub fn longhorn_operation_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriOperationState>,
    command: OperationCancellationCommand,
) -> Result<OperationCancellationResult, OperationHostError> {
    let result = state.service.cancel(window.label(), command)?;
    if let Some(event) = operation_cancellation_changed_event(&result) {
        let _ = window.emit(OPERATION_CHANGED_EVENT, event);
    }
    Ok(result)
}

/// Publishes a trusted invalidation hint after an external commit.
pub fn publish_operation_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: OperationChangedEvent,
) -> Result<(), OperationHostError> {
    app.emit(OPERATION_CHANGED_EVENT, event)
        .map_err(|error| OperationHostError::event_publication(error.to_string()))
}

/// Projects a mutation invalidation hint only after authority commit.
#[must_use]
pub fn operation_mutation_changed_event(
    result: &OperationMutationResult,
) -> Option<OperationChangedEvent> {
    OperationChangedEvent::from_mutation(result)
}

/// Projects a cancellation hint only when authority revision advanced.
#[must_use]
pub fn operation_cancellation_changed_event(
    result: &OperationCancellationResult,
) -> Option<OperationChangedEvent> {
    OperationChangedEvent::from_cancellation(result)
}
