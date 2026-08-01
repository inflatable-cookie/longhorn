use longhorn_operation::{
    OperationCatalogueLimits, OperationMutationCommand, OperationMutationResult,
    OperationProtocolVersion, OperationRejectionCode, OperationSnapshot,
};
use serde_json::Value;

use crate::support::{catalogue, registration, transition};
use longhorn_operation::OperationState;

#[test]
fn rust_fixture_round_trips_without_product_payload_fields() {
    let source = include_str!("../../../../fixtures/operation/protocol-v1.json");
    let value: Value = serde_json::from_str(source).unwrap();
    let encoded = serde_json::to_string_pretty(&value).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&encoded).unwrap(), value);
    assert_payload_free(&value);
    assert_eq!(
        value["protocolVersion"],
        OperationProtocolVersion::CURRENT.get()
    );
}

#[test]
fn future_protocol_rejection_preserves_authority_state() {
    let source: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/operation/protocol-v1.json"
    ))
    .unwrap();
    let mut command = source["mutationCommands"][0].clone();
    command["protocolVersion"] = Value::from(2);
    let command: OperationMutationCommand = serde_json::from_value(command).unwrap();
    let mut catalogue = catalogue("authority:fixture", 7);

    let result = catalogue.execute_protocol_mutation(command).unwrap();
    let OperationMutationResult::Rejected {
        snapshot,
        rejection,
        ..
    } = result
    else {
        panic!("future protocol must reject");
    };
    assert_eq!(rejection.code, OperationRejectionCode::IncompatibleProtocol);
    assert_eq!(snapshot.catalogue_revision.get(), 0);
    assert!(snapshot.active.is_empty());
}

#[test]
fn snapshot_reports_cumulative_terminal_eviction_evidence() {
    let mut catalogue = catalogue("authority:retention", 1);
    catalogue
        .change_retention(longhorn_operation::OperationRetentionChange::new(
            catalogue.authority().clone(),
            catalogue.revision(),
            OperationCatalogueLimits::new(8, 1, 16 * 1_024).unwrap(),
        ))
        .unwrap();
    for id in ["operation:one", "operation:two"] {
        catalogue
            .register(registration(
                &catalogue,
                id,
                "test",
                id,
                OperationState::Running,
            ))
            .unwrap();
        catalogue
            .transition(transition(&catalogue, id, OperationState::Succeeded))
            .unwrap();
    }
    let snapshot = OperationSnapshot::from_catalogue(&catalogue).unwrap();
    assert_eq!(snapshot.terminal_eviction_count, 1);
    assert_eq!(snapshot.recent.len(), 1);
}

fn assert_payload_free(value: &Value) {
    match value {
        Value::Array(values) => values.iter().for_each(assert_payload_free),
        Value::Object(values) => {
            for (key, value) in values {
                assert!(!matches!(
                    key.as_str(),
                    "payload" | "result" | "artifact" | "report" | "log"
                ));
                assert_payload_free(value);
            }
        }
        _ => {}
    }
}
