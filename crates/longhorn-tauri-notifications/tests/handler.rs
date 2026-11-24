//! Injected notification handler contract tests.

use std::sync::{Arc, Mutex};

use longhorn_core::NotificationAuthorityId;
use longhorn_notifications::{
    NotificationAuthorityEpoch, NotificationLedger, NotificationLedgerLimits,
    NotificationMutationCommand, NotificationMutationResult, NotificationSnapshotQuery,
    NotificationSnapshotResponse,
};
use longhorn_tauri_notifications::{
    NOTIFICATION_CHANGED_EVENT, NotificationHandlerAssembly, NotificationHostAuthority,
    NotificationHostError, TauriNotificationState, longhorn_notifications_mutate,
    notification_mutation_changed_event,
};
use tauri::{Listener, Manager, WebviewUrl, WebviewWindowBuilder};

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

#[test]
fn committed_mutation_broadcasts_the_invalidation_hint_to_other_windows() {
    let authority = Authority {
        ledger: NotificationLedger::new(
            id("authority:fixture"),
            NotificationAuthorityEpoch::new(7).unwrap(),
            NotificationLedgerLimits::DEFAULT,
        ),
        callers: Vec::new(),
    };
    let app = tauri::test::mock_builder()
        .manage(TauriNotificationState::new(Arc::new(
            NotificationHandlerAssembly::new(authority),
        )))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let caller = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();
    let observer = WebviewWindowBuilder::new(&app, "other", WebviewUrl::default())
        .build()
        .unwrap();
    let received = Arc::new(Mutex::new(Vec::new()));
    let hints = Arc::clone(&received);
    observer.listen(NOTIFICATION_CHANGED_EVENT, move |event| {
        hints.lock().unwrap().push(
            serde_json::from_str::<longhorn_notifications::NotificationChangedEvent>(
                event.payload(),
            )
            .unwrap(),
        );
    });
    let protocol: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/notifications/protocol-v1.json"
    ))
    .unwrap();
    let command = serde_json::from_value(protocol["mutationCommands"][0].clone()).unwrap();
    let result =
        tauri::async_runtime::block_on(longhorn_notifications_mutate(caller, app.state(), command))
            .unwrap();

    assert_eq!(
        *received.lock().unwrap(),
        [notification_mutation_changed_event(&result).unwrap()]
    );
}

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("test id is valid")
}
