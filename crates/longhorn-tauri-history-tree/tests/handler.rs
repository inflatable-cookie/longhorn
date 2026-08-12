//! Direct, serialized, and Tauri mock-runtime handler conformance.

use std::sync::{Arc, Mutex};

use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkChangedKind, ForkContinuationPageCommand,
    ForkContinuationPageSnapshot, ForkDeleteContinuationCommand, ForkNavigationCommand,
    ForkNavigationResult, ForkPathPageCommand, ForkPathPageSnapshot, ForkPruneCommand,
    ForkPruneResult, ForkRemovalReceiptProjection, ForkSnapshot,
};
use longhorn_tauri_history_tree::{
    ForkHistoryHandlerAssembly, ForkHistoryHostAuthority, ForkHistoryHostError,
    ForkHistoryHostService, TauriForkHistoryState, fork_history_changed_event,
    fork_retention_changed_event, longhorn_history_tree_branches,
    longhorn_history_tree_continuations, longhorn_history_tree_delete_continuation,
    longhorn_history_tree_navigate, longhorn_history_tree_path, longhorn_history_tree_prune,
    longhorn_history_tree_snapshot,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct Authority {
    calls: Arc<Mutex<Vec<String>>>,
}

impl ForkHistoryHostAuthority for Authority {
    fn snapshot(&mut self, caller: &str) -> Result<ForkSnapshot, ForkHistoryHostError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("snapshot:{caller}"));
        Ok(snapshot())
    }

    fn path(
        &mut self,
        caller: &str,
        _: ForkPathPageCommand,
    ) -> Result<ForkPathPageSnapshot, ForkHistoryHostError> {
        self.calls.lock().unwrap().push(format!("path:{caller}"));
        Ok(path())
    }

    fn branches(
        &mut self,
        caller: &str,
        _: ForkBranchPageCommand,
    ) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("branches:{caller}"));
        Ok(branches())
    }

    fn continuations(
        &mut self,
        caller: &str,
        _: ForkContinuationPageCommand,
    ) -> Result<ForkContinuationPageSnapshot, ForkHistoryHostError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("continuations:{caller}"));
        Ok(continuations())
    }

    fn delete_continuation(
        &mut self,
        caller: &str,
        _: ForkDeleteContinuationCommand,
    ) -> Result<ForkRemovalReceiptProjection, ForkHistoryHostError> {
        self.calls.lock().unwrap().push(format!("delete:{caller}"));
        Ok(removal())
    }

    fn prune(
        &mut self,
        caller: &str,
        _: ForkPruneCommand,
    ) -> Result<ForkPruneResult, ForkHistoryHostError> {
        self.calls.lock().unwrap().push(format!("prune:{caller}"));
        Ok(ForkPruneResult::Unchanged)
    }

    fn navigate(
        &mut self,
        caller: &str,
        _: ForkNavigationCommand,
    ) -> Result<ForkNavigationResult, ForkHistoryHostError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("navigate:{caller}"));
        Ok(committed())
    }
}

#[test]
fn mock_runtime_uses_one_injected_caller_aware_assembly() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = Arc::new(ForkHistoryHandlerAssembly::new(Authority {
        calls: calls.clone(),
    }));
    let app = tauri::test::mock_builder()
        .manage(TauriForkHistoryState::new(service))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let window = WebviewWindowBuilder::new(&app, "history", WebviewUrl::default())
        .build()
        .unwrap();
    assert_eq!(
        longhorn_history_tree_snapshot(window.clone(), app.state()).unwrap(),
        snapshot()
    );
    assert_eq!(
        longhorn_history_tree_path(window.clone(), app.state(), path_command()).unwrap(),
        path()
    );
    assert_eq!(
        longhorn_history_tree_branches(window.clone(), app.state(), branch_command()).unwrap(),
        branches()
    );
    assert_eq!(
        longhorn_history_tree_continuations(window.clone(), app.state(), continuation_command())
            .unwrap(),
        continuations()
    );
    assert_eq!(
        longhorn_history_tree_delete_continuation(window.clone(), app.state(), delete_command())
            .unwrap(),
        removal()
    );
    assert_eq!(
        longhorn_history_tree_prune(window.clone(), app.state(), prune_command()).unwrap(),
        ForkPruneResult::Unchanged
    );
    assert_eq!(
        longhorn_history_tree_navigate(window, app.state(), navigation_command()).unwrap(),
        committed()
    );
    assert_eq!(
        *calls.lock().unwrap(),
        [
            "snapshot:history",
            "path:history",
            "branches:history",
            "continuations:history",
            "delete:history",
            "prune:history",
            "navigate:history"
        ]
    );
}

#[test]
fn serialized_service_and_committed_event_remain_payload_free() {
    let service = ForkHistoryHandlerAssembly::new(Authority {
        calls: Arc::new(Mutex::new(Vec::new())),
    });
    let page: ForkPathPageSnapshot = serde_json::from_slice(
        &serde_json::to_vec(&service.path("main", path_command()).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(page, path());
    let event = fork_history_changed_event(&committed()).unwrap();
    assert_eq!(event.previous_revision.unwrap().get(), 4);
    assert_eq!(event.committed_revision.get(), 5);
    // Card 185/186: both destructive commands invalidate every page a
    // consumer holds, because after a removal those pages name entries that no
    // longer exist.
    let retention = fork_retention_changed_event(&removal());
    assert_eq!(retention.kind, ForkChangedKind::Retention);
    assert_eq!(retention.previous_revision.unwrap().get(), 4);
    assert_eq!(retention.committed_revision.get(), 5);
    assert_eq!(retention.history_id.as_str(), "history:tree");

    let text = serde_json::to_string(&(snapshot(), path(), branches(), committed())).unwrap();
    assert!(!text.to_ascii_lowercase().contains("payload"));
}

fn snapshot() -> ForkSnapshot {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"summary":{"historyId":"history:tree","revision":4,"currentBranchId":"branch:main","currentEntryId":"entry:b","undoDepth":2,"redoDepth":1,"nextUndoLabel":"Move","nextRedoLabel":"Resize","retainedEntryCount":4,"retainedEncodedWeight":64,"branchCount":2,"alternatePathCount":2}})).unwrap()
}
fn path() -> ForkPathPageSnapshot {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","revision":4,"branchId":null,"headEntryId":"entry:c","precedingContinuationCount":1,"offset":0,"totalEntries":1,"entries":[{"entryId":"entry:b","label":"Move","kindId":null,"groupId":null,"recordedAt":null,"continuationCount":2,"sequence":2,"committedRevision":2,"encodedWeight":16,"position":"current"}],"truncatedBefore":false,"truncatedAfter":false})).unwrap()
}
fn branches() -> ForkBranchPageSnapshot {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","revision":4,"offset":0,"totalBranches":1,"branches":[{"branchId":"branch:main","headEntryId":"entry:c","divergenceEntryId":null,"divergenceBranchId":null,"name":"Main","annotation":null,"pinned":true,"current":true}],"truncatedBefore":false,"truncatedAfter":false})).unwrap()
}
fn continuations() -> ForkContinuationPageSnapshot {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","revision":4,"anchorEntryId":"entry:b","offset":0,"totalContinuations":1,"continuations":[{"entryId":"entry:c","label":"Resize","recordedAt":null,"preferred":true,"entryCount":1,"branchId":"branch:main","branchName":"Main"}],"truncatedBefore":false,"truncatedAfter":false})).unwrap()
}
fn removal() -> ForkRemovalReceiptProjection {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","previousRevision":4,"committedRevision":5,"removedEntries":[{"entryId":"entry:d","sequence":4,"encodedWeight":16}],"removedBranches":["branch:alternate"],"removedCheckpoints":[],"retainedEntryCount":3,"retainedEncodedWeight":48,"unprotectedEntryCount":0,"unprotectedEncodedWeight":0})).unwrap()
}
fn delete_command() -> ForkDeleteContinuationCommand {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","expectedRevision":4,"entryId":"entry:d"})).unwrap()
}
fn committed() -> ForkNavigationResult {
    serde_json::from_value(serde_json::json!({"status":"committed","snapshot":{"protocolVersion":1,"authorityEpoch":7,"summary":{"historyId":"history:tree","revision":5,"currentBranchId":"branch:main","currentEntryId":"entry:c","undoDepth":3,"redoDepth":0,"nextUndoLabel":"Resize","nextRedoLabel":null,"retainedEntryCount":4,"retainedEncodedWeight":64,"branchCount":2,"alternatePathCount":2}},"receipt":{"historyId":"history:tree","planId":"plan:test","previousRevision":4,"committedRevision":5,"sourceEntryId":"entry:b","targetEntryId":"entry:c","targetBranchId":"branch:main","movedEntryIds":["entry:c"]}})).unwrap()
}
fn path_command() -> ForkPathPageCommand {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","expectedRevision":4,"target":{"kind":"default"},"offset":0,"limit":10})).unwrap()
}
fn branch_command() -> ForkBranchPageCommand {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","expectedRevision":4,"offset":0,"limit":10})).unwrap()
}
fn continuation_command() -> ForkContinuationPageCommand {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","expectedRevision":4,"anchorEntryId":"entry:b","offset":0,"limit":10})).unwrap()
}
fn prune_command() -> ForkPruneCommand {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","expectedRevision":4,"maximumEntries":16,"maximumEncodedWeight":1024})).unwrap()
}
fn navigation_command() -> ForkNavigationCommand {
    serde_json::from_value(serde_json::json!({"protocolVersion":1,"authorityEpoch":7,"historyId":"history:tree","planId":"plan:test","expectedRevision":4,"target":{"kind":"redo"}})).unwrap()
}
