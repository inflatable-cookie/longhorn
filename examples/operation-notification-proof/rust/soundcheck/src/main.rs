use longhorn_core::{OperationAuthorityId, OperationCatalogueRevision, OperationRevision};
use longhorn_operation::{
    OperationAuthorityEpoch, OperationAuthorityProjection, OperationCancellationCommand,
    OperationCancellationOutcomeProjection, OperationCancellationResult,
    OperationCancellationSupportProjection, OperationCatalogue, OperationCatalogueLimits,
    OperationMutationCommand, OperationMutationResult,
    OperationOverallProgressProjection, OperationProtocolVersion, OperationSnapshot,
    OperationSnapshotQuery, OperationSnapshotResponse, OperationStateProjection,
};
use serde_json::{Value, json};

fn main() {
    let mut catalogue = OperationCatalogue::new(
        id::<OperationAuthorityId>("authority:soundcheck"),
        OperationAuthorityEpoch::new(6).unwrap(),
        OperationCatalogueLimits::new(4, 8, 16_384).unwrap(),
    );
    let query = OperationSnapshotQuery {
        protocol_version: OperationProtocolVersion::CURRENT,
        request_id: id("request:snapshot"),
    };
    let snapshot_response = OperationSnapshotResponse {
        request_id: query.request_id,
        snapshot: OperationSnapshot::from_catalogue(&catalogue).unwrap(),
    };
    let register = OperationMutationCommand::Register {
        request_id: id("request:register"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority(),
        expected_catalogue_revision: OperationCatalogueRevision::INITIAL,
        operation_id: id("operation:scan"),
        kind_id: id("soundcheck.plugin-scan"),
        scope_id: None,
        label: "Scan plug-ins".into(),
        initial_state: OperationStateProjection::Running,
        cancellation_support: OperationCancellationSupportProjection::Supported,
        retry_of: None,
    };
    let registered = catalogue.execute_protocol_mutation(register.clone()).unwrap();
    let cancel = OperationCancellationCommand {
        request_id: id("request:cancel"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority(),
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::INITIAL,
    };
    let cancelled = catalogue.execute_protocol_cancellation(cancel.clone()).unwrap();
    let repeat_cancel = OperationCancellationCommand {
        request_id: id("request:repeat-cancel"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority(),
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::new(1),
    };
    let repeated = catalogue.execute_protocol_cancellation(repeat_cancel.clone()).unwrap();
    assert!(matches!(
        &repeated,
        OperationCancellationResult::Committed { receipt, .. }
            if receipt.outcome == OperationCancellationOutcomeProjection::AlreadyRequested
    ));
    let terminal = OperationMutationCommand::Transition {
        request_id: id("request:terminal"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority(),
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::new(1),
        next_state: OperationStateProjection::Cancelled,
    };
    let terminal_result = catalogue.execute_protocol_mutation(terminal.clone()).unwrap();
    let late_progress = OperationMutationCommand::Progress {
        request_id: id("request:late-progress"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority(),
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::new(2),
        overall: OperationOverallProgressProjection::Normalized { value: 0.9 },
        phase: None,
    };
    let late_result = catalogue.execute_protocol_mutation(late_progress.clone()).unwrap();
    assert!(matches!(&late_result, OperationMutationResult::Rejected { .. }));
    let snapshots = vec![
        snapshot_mutation(&registered),
        snapshot_cancellation(&cancelled),
        snapshot_cancellation(&repeated),
        snapshot_mutation(&terminal_result),
        snapshot_mutation(&late_result),
    ];
    let expected_trace = trace(&snapshots);
    println!("{}", json!({
        "shape": "soundcheck",
        "publicTrace": expected_trace,
        "rendererFixture": {
            "snapshotResponse": snapshot_response,
            "steps": [
                { "kind": "mutate", "command": register, "result": registered },
                { "kind": "cancel", "command": cancel, "result": cancelled },
                { "kind": "cancel", "command": repeat_cancel, "result": repeated },
                { "kind": "mutate", "command": terminal, "result": terminal_result },
                { "kind": "mutate", "command": late_progress, "result": late_result }
            ],
            "expectedTrace": expected_trace
        }
    }));
}

fn trace(snapshots: &[&OperationSnapshot]) -> Value {
    Value::Array(snapshots.iter().map(|snapshot| {
        let mut states = snapshot.active.iter().chain(&snapshot.recent).map(|entry| json!({
            "operationId": entry.operation_id,
            "state": entry.state
        })).collect::<Vec<_>>();
        states.sort_by_key(|entry| entry["operationId"].as_str().unwrap().to_owned());
        json!({ "revision": snapshot.catalogue_revision, "states": states })
    }).collect())
}

fn snapshot_mutation(result: &OperationMutationResult) -> &OperationSnapshot {
    match result { OperationMutationResult::Committed { snapshot, .. } | OperationMutationResult::Rejected { snapshot, .. } => snapshot }
}

fn snapshot_cancellation(result: &OperationCancellationResult) -> &OperationSnapshot {
    match result { OperationCancellationResult::Committed { snapshot, .. } | OperationCancellationResult::Rejected { snapshot, .. } => snapshot }
}

fn authority() -> OperationAuthorityProjection {
    OperationAuthorityProjection { authority_id: id("authority:soundcheck"), authority_epoch: 6 }
}

fn id<T>(value: &str) -> T where T: std::str::FromStr, T::Err: std::fmt::Debug { value.parse().unwrap() }
