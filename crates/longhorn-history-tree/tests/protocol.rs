//! Exact metadata protocol and payload-boundary evidence.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::HistoryAuthorityEpoch;
use longhorn_history_tree::{
    ForkBranchId, ForkBranchPageCommand, ForkHistoryProtocolVersion, ForkNavigationError,
    ForkNavigationRejectionCode, ForkNavigationRejectionProjection, ForkPathPageCommand,
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

#[test]
fn already_at_target_wire_code_round_trips_with_existing_detail() {
    // Longhorn owns the wire variant and the domain diagnostic string. Hosts
    // (Loophole) assign the code when projecting domain errors; this crate has
    // no error-to-rejection mapper.
    let detail = ForkNavigationError::<std::convert::Infallible>::AlreadyAtTarget.to_string();
    let rejection = ForkNavigationRejectionProjection {
        code: ForkNavigationRejectionCode::AlreadyAtTarget,
        detail,
        refresh_required: false,
    };
    let value = serde_json::to_value(&rejection).unwrap();
    assert_eq!(value["code"], "alreadyAtTarget");
    assert_eq!(
        value["detail"],
        "fork history is already at the requested target"
    );
    assert_eq!(value["refreshRequired"], false);
    assert_ne!(value["code"], "invalidRequest");
    assert_ne!(value["code"], "unknownTarget");

    let round_trip: ForkNavigationRejectionProjection = serde_json::from_value(value).unwrap();
    assert_eq!(round_trip, rejection);
}

#[test]
fn unknown_target_wire_code_still_round_trips() {
    let entry_id = HistoryEntryId::new("entry:missing").unwrap();
    let detail =
        ForkNavigationError::<std::convert::Infallible>::UnknownEntry(entry_id).to_string();
    let rejection = ForkNavigationRejectionProjection {
        code: ForkNavigationRejectionCode::UnknownTarget,
        detail,
        refresh_required: false,
    };
    let value = serde_json::to_value(&rejection).unwrap();
    assert_eq!(value["code"], "unknownTarget");
    assert_eq!(value["detail"], "fork navigation entry does not exist");
}
