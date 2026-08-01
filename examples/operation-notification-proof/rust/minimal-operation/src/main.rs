use longhorn_core::{OperationAuthorityId, OperationCatalogueRevision, OperationRevision};
use longhorn_operation::{
    OperationAuthorityEpoch, OperationAuthorityProjection, OperationCancellationSupportProjection,
    OperationCatalogue, OperationCatalogueLimits, OperationMutationCommand,
    OperationMutationResult, OperationProtocolVersion, OperationSnapshot, OperationSnapshotQuery,
    OperationSnapshotResponse,
};
use serde_json::{Value, json};

fn main() {
    let mut catalogue = catalogue();
    let query = OperationSnapshotQuery {
        protocol_version: OperationProtocolVersion::CURRENT,
        request_id: id("request:snapshot"),
    };
    let snapshot_response = OperationSnapshotResponse {
        request_id: query.request_id,
        snapshot: OperationSnapshot::from_catalogue(&catalogue).unwrap(),
    };
    let commands = vec![
        OperationMutationCommand::Register {
            request_id: id("request:register"),
            protocol_version: OperationProtocolVersion::CURRENT,
            authority: authority(),
            expected_catalogue_revision: OperationCatalogueRevision::INITIAL,
            operation_id: id("operation:cleanup"),
            kind_id: id("system.cleanup"),
            scope_id: None,
            label: "Clean temporary files".into(),
            initial_state: longhorn_operation::OperationStateProjection::Running,
            cancellation_support: OperationCancellationSupportProjection::Unsupported,
            retry_of: None,
        },
        OperationMutationCommand::Transition {
            request_id: id("request:succeed"),
            protocol_version: OperationProtocolVersion::CURRENT,
            authority: authority(),
            operation_id: id("operation:cleanup"),
            expected_operation_revision: OperationRevision::INITIAL,
            next_state: longhorn_operation::OperationStateProjection::Succeeded,
        },
    ];
    let results = commands
        .iter()
        .cloned()
        .map(|command| catalogue.execute_protocol_mutation(command).unwrap())
        .collect::<Vec<_>>();
    let expected_trace = trace(&results);
    println!("{}", json!({
        "shape": "minimal-operation",
        "publicTrace": expected_trace,
        "rendererFixture": {
            "snapshotResponse": snapshot_response,
            "commands": commands,
            "results": results,
            "expectedTrace": expected_trace
        }
    }));
}

fn trace(results: &[OperationMutationResult]) -> Value {
    Value::Array(results.iter().map(|result| {
        let snapshot = snapshot(result);
        let mut states = snapshot.active.iter().chain(&snapshot.recent).map(|entry| json!({
            "operationId": entry.operation_id,
            "state": entry.state
        })).collect::<Vec<_>>();
        states.sort_by_key(|entry| entry["operationId"].as_str().unwrap().to_owned());
        json!({ "revision": snapshot.catalogue_revision, "states": states })
    }).collect())
}

fn snapshot(result: &OperationMutationResult) -> &OperationSnapshot {
    match result {
        OperationMutationResult::Committed { snapshot, .. }
        | OperationMutationResult::Rejected { snapshot, .. } => snapshot,
    }
}

fn catalogue() -> OperationCatalogue {
    OperationCatalogue::new(
        id::<OperationAuthorityId>("authority:minimal"),
        OperationAuthorityEpoch::new(1).unwrap(),
        OperationCatalogueLimits::new(8, 8, 16_384).unwrap(),
    )
}

fn authority() -> OperationAuthorityProjection {
    OperationAuthorityProjection { authority_id: id("authority:minimal"), authority_epoch: 1 }
}

fn id<T>(value: &str) -> T where T: std::str::FromStr, T::Err: std::fmt::Debug {
    value.parse().unwrap()
}
