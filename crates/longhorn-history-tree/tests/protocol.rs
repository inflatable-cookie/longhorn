//! Exact metadata protocol and payload-boundary evidence.

use longhorn_core::{HistoryId, HistoryRevision};
use longhorn_history::HistoryAuthorityEpoch;
use longhorn_history_tree::{
    ForkBranchId, ForkBranchPageCommand, ForkHistoryProtocolVersion, ForkPathPageCommand,
    ForkPathTargetProjection, ForkSnapshot, ForkSummaryProjection,
};
use serde_json::{Value, json};

fn snapshot() -> ForkSnapshot {
    ForkSnapshot {
        protocol_version: ForkHistoryProtocolVersion::CURRENT,
        authority_epoch: HistoryAuthorityEpoch::new(7).unwrap(),
        summary: ForkSummaryProjection {
            history_id: HistoryId::new("history:fixture").unwrap(),
            revision: HistoryRevision::new(12),
            current_branch_id: ForkBranchId::new("branch:main").unwrap(),
            current_entry_id: None,
            undo_depth: 0,
            redo_depth: 0,
            next_undo_label: None,
            next_redo_label: None,
            retained_entry_count: 0,
            retained_encoded_weight: 0,
            branch_count: 1,
            alternate_path_count: 0,
        },
    }
}

#[test]
fn exact_v1_snapshot_round_trips_without_product_payloads() {
    let snapshot = snapshot();
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(!text.to_ascii_lowercase().contains("payload"));
    assert_eq!(
        serde_json::from_slice::<ForkSnapshot>(&bytes).unwrap(),
        snapshot
    );

    let mut future: Value = serde_json::from_slice(&bytes).unwrap();
    future["protocolVersion"] = Value::from(2);
    // Serde preserves the number; checked clients and hosts own exact-version
    // admission rather than silently interpreting it as v1.
    assert_eq!(
        serde_json::from_value::<ForkSnapshot>(future)
            .unwrap()
            .protocol_version
            .get(),
        2
    );
}

#[test]
fn page_commands_are_revision_bound_and_reject_unknown_fields() {
    let command = ForkPathPageCommand {
        protocol_version: ForkHistoryProtocolVersion::CURRENT,
        authority_epoch: HistoryAuthorityEpoch::new(7).unwrap(),
        history_id: HistoryId::new("history:fixture").unwrap(),
        expected_revision: HistoryRevision::new(12),
        target: ForkPathTargetProjection::Branch {
            branch_id: ForkBranchId::new("branch:alternate").unwrap(),
        },
        offset: 0,
        limit: 50,
    };
    let value = serde_json::to_value(&command).unwrap();
    assert_eq!(value["target"]["kind"], "branch");
    assert_eq!(value["target"]["branchId"], "branch:alternate");

    let branch: ForkBranchPageCommand = serde_json::from_value(json!({
        "protocolVersion": 1,
        "authorityEpoch": 7,
        "historyId": "history:fixture",
        "expectedRevision": 12,
        "offset": 0,
        "limit": 25
    }))
    .unwrap();
    assert_eq!(branch.limit, 25);

    let mut unknown = serde_json::to_value(command).unwrap();
    unknown["payload"] = json!({"consumer": "forbidden"});
    assert!(serde_json::from_value::<ForkPathPageCommand>(unknown).is_err());
}
