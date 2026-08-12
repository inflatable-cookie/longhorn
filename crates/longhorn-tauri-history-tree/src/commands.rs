use std::sync::Arc;

use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkChangedEvent, ForkChangedKind,
    ForkContinuationPageCommand, ForkContinuationPageSnapshot, ForkDeleteContinuationCommand,
    ForkHistoryProtocolVersion, ForkNavigationCommand, ForkNavigationResult, ForkPathPageCommand,
    ForkPathPageSnapshot, ForkPruneCommand, ForkPruneResult, ForkRemovalReceiptProjection,
    ForkSnapshot,
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
    /// Returns one caller-authorized bounded continuation page.
    fn continuations(
        &self,
        caller: &str,
        command: ForkContinuationPageCommand,
    ) -> Result<ForkContinuationPageSnapshot, ForkHistoryHostError>;
    /// Deletes one continuation and everything below it. Irreversible.
    fn delete_continuation(
        &self,
        caller: &str,
        command: ForkDeleteContinuationCommand,
    ) -> Result<ForkRemovalReceiptProjection, ForkHistoryHostError>;
    /// Prunes the unprotected share of the graph to a budget.
    fn prune(
        &self,
        caller: &str,
        command: ForkPruneCommand,
    ) -> Result<ForkPruneResult, ForkHistoryHostError>;
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

/// Returns one caller-authorized bounded continuation page.
#[tauri::command]
pub fn longhorn_history_tree_continuations<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkContinuationPageCommand,
) -> Result<ForkContinuationPageSnapshot, ForkHistoryHostError> {
    state.service.continuations(window.label(), command)
}

/// Deletes one continuation and everything below it. Irreversible.
///
/// Publishes `ForkChangedKind::Retention`: every page a consumer holds names
/// entries that may no longer exist, so the invalidation is not optional.
#[tauri::command]
pub fn longhorn_history_tree_delete_continuation<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkDeleteContinuationCommand,
) -> Result<ForkRemovalReceiptProjection, ForkHistoryHostError> {
    let receipt = state.service.delete_continuation(window.label(), command)?;
    if let Err(error) = window.emit(
        FORK_HISTORY_CHANGED_EVENT,
        fork_retention_changed_event(&receipt),
    ) {
        longhorn_core::report_best_effort_failure("history-tree.retention-emit", error);
    }
    Ok(receipt)
}

/// Prunes the unprotected share of the graph to a budget.
///
/// Under the destructive capability, not the mutate one: pruning removes
/// entries. Publishes `ForkChangedKind::Retention` only when something went.
#[tauri::command]
pub fn longhorn_history_tree_prune<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkPruneCommand,
) -> Result<ForkPruneResult, ForkHistoryHostError> {
    let result = state.service.prune(window.label(), command)?;
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
pub fn longhorn_history_tree_navigate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriForkHistoryState>,
    command: ForkNavigationCommand,
) -> Result<ForkNavigationResult, ForkHistoryHostError> {
    let result = state.service.navigate(window.label(), command)?;
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
