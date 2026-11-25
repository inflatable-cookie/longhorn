use std::sync::Arc;

use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkChangedEvent, ForkChangedKind,
    ForkContinuationPageCommand, ForkContinuationPageSnapshot, ForkDeleteContinuationCommand,
    ForkDeleteResult, ForkHistoryProtocolVersion, ForkNavigationCommand, ForkNavigationResult,
    ForkPathPageCommand, ForkPathPageSnapshot, ForkPruneCommand, ForkPruneResult,
    ForkRemovalReceiptProjection, ForkSnapshot,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{ForkHistoryHandlerAssembly, ForkHistoryHostAuthority, ForkHistoryHostError};

/// Non-durable committed graph invalidation hint.
pub const FORK_HISTORY_CHANGED_EVENT: &str = "longhorn://history-tree/changed";

/// Type-erased graph host installed once in Tauri managed state.
pub struct TauriForkHistoryState {
    assembly: Arc<ForkHistoryHandlerAssembly<dyn ForkHistoryHostAuthority>>,
}

impl TauriForkHistoryState {
    /// Wraps one explicitly injected graph assembly.
    #[must_use]
    pub fn new(assembly: Arc<ForkHistoryHandlerAssembly<dyn ForkHistoryHostAuthority>>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriForkHistoryState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &ForkHistoryHandlerAssembly<dyn ForkHistoryHostAuthority>,
        &str,
    ) -> Result<T, ForkHistoryHostError>
    + Send
    + 'static,
) -> Result<T, ForkHistoryHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| ForkHistoryHostError::state_unavailable())?
}

/// Returns caller-authorized linear-default graph metadata.
#[tauri::command]
pub async fn longhorn_history_tree_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
) -> Result<ForkSnapshot, ForkHistoryHostError> {
    run_blocking(state, &window, |assembly, label| assembly.snapshot(label)).await
}

/// Returns one caller-authorized bounded path page.
#[tauri::command]
pub async fn longhorn_history_tree_path<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkPathPageCommand,
) -> Result<ForkPathPageSnapshot, ForkHistoryHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.path(label, command)
    })
    .await
}

/// Returns one caller-authorized bounded branch page.
#[tauri::command]
pub async fn longhorn_history_tree_branches<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkBranchPageCommand,
) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.branches(label, command)
    })
    .await
}

/// Returns one caller-authorized bounded continuation page.
#[tauri::command]
pub async fn longhorn_history_tree_continuations<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkContinuationPageCommand,
) -> Result<ForkContinuationPageSnapshot, ForkHistoryHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.continuations(label, command)
    })
    .await
}

/// Deletes one continuation and everything below it. Irreversible.
///
/// Publishes `ForkChangedKind::Retention`: every page a consumer holds names
/// entries that may no longer exist, so the invalidation is not optional.
#[tauri::command]
pub async fn longhorn_history_tree_delete_continuation<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkDeleteContinuationCommand,
) -> Result<ForkDeleteResult, ForkHistoryHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.delete_continuation(label, command)
    })
    .await?;
    if let ForkDeleteResult::Deleted { receipt } = &result
        && let Err(error) = window.emit(
            FORK_HISTORY_CHANGED_EVENT,
            fork_retention_changed_event(receipt),
        )
    {
        longhorn_core::report_best_effort_failure("history-tree.retention-emit", error);
    }
    Ok(result)
}

/// Prunes the unprotected share of the graph to a budget.
///
/// Under the destructive capability, not the mutate one: pruning removes
/// entries. Publishes `ForkChangedKind::Retention` only when something went.
#[tauri::command]
pub async fn longhorn_history_tree_prune<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkPruneCommand,
) -> Result<ForkPruneResult, ForkHistoryHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.prune(label, command)
    })
    .await?;
    if let ForkPruneResult::Pruned { receipt } = &result
        && let Err(error) = window.emit(
            FORK_HISTORY_CHANGED_EVENT,
            fork_retention_changed_event(receipt),
        )
    {
        longhorn_core::report_best_effort_failure("history-tree.retention-emit", error);
    }
    Ok(result)
}

/// Applies and commits one caller-authorized graph navigation.
#[tauri::command]
pub async fn longhorn_history_tree_navigate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkNavigationCommand,
) -> Result<ForkNavigationResult, ForkHistoryHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.navigate(label, command)
    })
    .await?;
    if let Some(event) = fork_history_changed_event(&result)
        && let Err(error) = window.emit(FORK_HISTORY_CHANGED_EVENT, event)
    {
        longhorn_core::report_best_effort_failure("history-tree.changed-emit", error);
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

/// Projects the invalidation hint for a removal.
///
/// Both destructive commands build the same event, and a consumer holding any
/// page needs it: after a removal their pages name entries that no longer
/// exist. Pure, so it is asserted directly rather than through an emit.
#[must_use]
pub fn fork_retention_changed_event(receipt: &ForkRemovalReceiptProjection) -> ForkChangedEvent {
    ForkChangedEvent {
        protocol_version: ForkHistoryProtocolVersion::CURRENT,
        authority_epoch: receipt.authority_epoch,
        history_id: receipt.history_id.clone(),
        previous_revision: Some(receipt.previous_revision),
        committed_revision: receipt.committed_revision,
        kind: ForkChangedKind::Retention,
    }
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
