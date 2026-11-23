use std::sync::Arc;

use longhorn_operation::{
    OperationCancellationCommand, OperationCancellationResult, OperationChangedEvent,
    OperationMutationCommand, OperationMutationResult, OperationSnapshotQuery,
    OperationSnapshotResponse,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{OperationHandlerAssembly, OperationHostError};

/// Non-durable committed-operation invalidation hint.
pub const OPERATION_CHANGED_EVENT: &str = "longhorn://operation/changed";

/// Type-erased operation host installed once in Tauri managed state.
pub struct TauriOperationState {
    assembly: Arc<OperationHandlerAssembly>,
}

impl TauriOperationState {
    /// Wraps one explicitly injected operation assembly.
    #[must_use]
    pub fn new(assembly: Arc<OperationHandlerAssembly>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriOperationState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(&OperationHandlerAssembly, &str) -> Result<T, OperationHostError>
    + Send
    + 'static,
) -> Result<T, OperationHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| OperationHostError::authority_state_unavailable())?
}

/// Returns the caller-authorized operation snapshot.
#[tauri::command]
pub async fn longhorn_operation_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriOperationState>,
    query: OperationSnapshotQuery,
) -> Result<OperationSnapshotResponse, OperationHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.snapshot(label, query)
    })
    .await
}

/// Applies one caller-authorized management mutation.
#[tauri::command]
pub async fn longhorn_operation_mutate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriOperationState>,
    command: OperationMutationCommand,
) -> Result<OperationMutationResult, OperationHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.mutate(label, command)
    })
    .await?;
    if let Some(event) = operation_mutation_changed_event(&result)
        && let Err(error) = window.emit(OPERATION_CHANGED_EVENT, event)
    {
        longhorn_core::report_best_effort_failure("operation.changed-emit", error);
    }
    Ok(result)
}

/// Admits one caller-authorized cancellation request.
#[tauri::command]
pub async fn longhorn_operation_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriOperationState>,
    command: OperationCancellationCommand,
) -> Result<OperationCancellationResult, OperationHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.cancel(label, command)
    })
    .await?;
    if let Some(event) = operation_cancellation_changed_event(&result)
        && let Err(error) = window.emit(OPERATION_CHANGED_EVENT, event)
    {
        longhorn_core::report_best_effort_failure("operation.changed-emit", error);
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
