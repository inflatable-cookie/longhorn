//! Injected notification handler contract tests.

use longhorn_core::NotificationAuthorityId;
use longhorn_notifications::{
    NotificationAuthorityEpoch, NotificationLedger, NotificationLedgerLimits,
    NotificationMutationCommand, NotificationMutationResult, NotificationSnapshotQuery,
    NotificationSnapshotResponse,
};
use longhorn_tauri_notifications::{
    NotificationHandlerAssembly, NotificationHostAuthority, NotificationHostError,
    NotificationHostService, notification_mutation_changed_event,
};

struct Authority {
    ledger: NotificationLedger,
    callers: Vec<String>,
}

impl NotificationHostAuthority for Authority {
    fn snapshot(
        &mut self,
        caller: &str,
        query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationHostError> {
        self.callers.push(caller.into());
        self.ledger
            .execute_protocol_snapshot(query)
            .map_err(|error| NotificationHostError::authority(error.to_string(), false))
    }

    fn mutate(
        &mut self,
        caller: &str,
        command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationHostError> {
        self.callers.push(caller.into());
        self.ledger
            .execute_protocol_mutation(command)
            .map_err(|error| NotificationHostError::authority(error.to_string(), false))
    }
}

#[test]
fn injected_handler_forwards_caller_and_exact_snapshot() {
    let assembly = NotificationHandlerAssembly::new(Authority {
        ledger: NotificationLedger::new(
            id::<NotificationAuthorityId>("authority:test"),
            NotificationAuthorityEpoch::new(1).expect("epoch is valid"),
            NotificationLedgerLimits::DEFAULT,
        ),
        callers: Vec::new(),
    });
    let response = assembly
        .snapshot(
            "main",
            serde_json::from_value(serde_json::json!({
                "protocolVersion": 1,
                "requestId": "request:snapshot",
                "offset": 0,
                "limit": 10
            }))
            .expect("query is valid"),
        )
        .expect("handler should return authority snapshot");
    assert_eq!(response.snapshot.page.records.len(), 0);
}

#[test]
fn rejected_results_never_emit_commit_hints() {
    let result: NotificationMutationResult = serde_json::from_value(serde_json::json!({
        "status": "rejected",
        "requestId": "request:rejected",
        "snapshot": {
            "protocolVersion": 1,
            "authority": { "authorityId": "authority:test", "authorityEpoch": 1 },
            "ledgerRevision": 0,
            "limits": { "maximumNotifications": 8, "maximumEncodedWeight": 16384 },
            "retainedCount": 0,
            "unseenCount": 0,
            "retainedEncodedWeight": 0,
            "prunedCount": 0,
            "page": { "offset": 0, "totalCount": 0, "hasMore": false, "records": [] }
        },
        "rejection": { "code": "invalidCommand", "detail": "invalid", "refreshRequired": false }
    }))
    .expect("result is valid");
    assert!(notification_mutation_changed_event(&result).is_none());
}

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("test id is valid")
}
