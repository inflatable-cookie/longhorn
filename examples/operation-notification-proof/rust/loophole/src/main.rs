use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationLedgerRevision,
    OperationAuthorityId, OperationCatalogueRevision, OperationRevision,
};
use longhorn_notifications::{
    NotificationActionProjection, NotificationAuthorityEpoch, NotificationAuthorityProjection,
    NotificationDraftProjection, NotificationLedger, NotificationLedgerLimits,
    NotificationMutationCommand, NotificationMutationResult, NotificationProtocolVersion,
    NotificationRetentionClassProjection, NotificationSeverityProjection, NotificationSnapshot,
    NotificationSnapshotQuery,
};
use longhorn_operation::{
    OperationAuthorityEpoch, OperationAuthorityProjection, OperationCancellationSupportProjection,
    OperationCatalogue, OperationCatalogueLimits, OperationMutationCommand,
    OperationMutationResult, OperationOverallProgressProjection, OperationProtocolVersion,
    OperationSnapshot, OperationSnapshotQuery, OperationSnapshotResponse, OperationStateProjection,
};
use serde_json::{Value, json};

fn main() {
    let operation = operation_fixture();
    let notifications = notification_fixture();
    println!("{}", json!({
        "shape": "loophole",
        "publicTrace": {
            "operation": operation["expectedTrace"],
            "notifications": notifications["expectedTrace"]
        },
        "rendererFixture": {
            "operation": operation,
            "notifications": notifications
        }
    }));
}

fn operation_fixture() -> Value {
    let mut catalogue = OperationCatalogue::new(
        id::<OperationAuthorityId>("authority:loophole-render"),
        OperationAuthorityEpoch::new(4).unwrap(),
        OperationCatalogueLimits::new(8, 16, 32_768).unwrap(),
    );
    let query = OperationSnapshotQuery {
        protocol_version: OperationProtocolVersion::CURRENT,
        request_id: id("request:operation-snapshot"),
    };
    let snapshot_response = OperationSnapshotResponse {
        request_id: query.request_id,
        snapshot: OperationSnapshot::from_catalogue(&catalogue).unwrap(),
    };
    let commands = vec![
        OperationMutationCommand::Register {
            request_id: id("request:queue"), protocol_version: OperationProtocolVersion::CURRENT,
            authority: operation_authority(), expected_catalogue_revision: OperationCatalogueRevision::INITIAL,
            operation_id: id("operation:render-final"), kind_id: id("loophole.render"), scope_id: None,
            label: "Render final sequence".into(), initial_state: OperationStateProjection::Queued,
            cancellation_support: OperationCancellationSupportProjection::Supported, retry_of: None,
        },
        OperationMutationCommand::Transition {
            request_id: id("request:start"), protocol_version: OperationProtocolVersion::CURRENT,
            authority: operation_authority(), operation_id: id("operation:render-final"),
            expected_operation_revision: OperationRevision::INITIAL, next_state: OperationStateProjection::Running,
        },
        OperationMutationCommand::Progress {
            request_id: id("request:progress"), protocol_version: OperationProtocolVersion::CURRENT,
            authority: operation_authority(), operation_id: id("operation:render-final"),
            expected_operation_revision: OperationRevision::new(1),
            overall: OperationOverallProgressProjection::Normalized { value: 0.65 }, phase: None,
        },
        OperationMutationCommand::Transition {
            request_id: id("request:complete"), protocol_version: OperationProtocolVersion::CURRENT,
            authority: operation_authority(), operation_id: id("operation:render-final"),
            expected_operation_revision: OperationRevision::new(2), next_state: OperationStateProjection::Succeeded,
        },
    ];
    let results = commands.iter().cloned().map(|command| catalogue.execute_protocol_mutation(command).unwrap()).collect::<Vec<_>>();
    let expected_trace = operation_trace(&results);
    json!({ "snapshotResponse": snapshot_response, "commands": commands, "results": results, "expectedTrace": expected_trace })
}

fn notification_fixture() -> Value {
    let mut ledger = NotificationLedger::new(
        id::<NotificationAuthorityId>("authority:loophole-notifications"),
        NotificationAuthorityEpoch::new(3).unwrap(),
        NotificationLedgerLimits::new(16, 32_768).unwrap(),
    );
    let snapshot_response = ledger.execute_protocol_snapshot(NotificationSnapshotQuery {
        protocol_version: NotificationProtocolVersion::CURRENT,
        request_id: id("request:notification-snapshot"), offset: 0, limit: 16,
    }).unwrap();
    let commands = vec![
        NotificationMutationCommand::Add {
            request_id: id("request:render-notification"), protocol_version: NotificationProtocolVersion::CURRENT,
            authority: notification_authority(), expected_ledger_revision: NotificationLedgerRevision::INITIAL,
            notification_id: id("notification:render-complete"), draft: notification_draft(
                "source:render", "Render complete", "The final sequence is ready.", Some("action:reveal-render"),
            ),
        },
        NotificationMutationCommand::Add {
            request_id: id("request:reliability-notification"), protocol_version: NotificationProtocolVersion::CURRENT,
            authority: notification_authority(), expected_ledger_revision: NotificationLedgerRevision::new(1),
            notification_id: id("notification:device-restored"), draft: notification_draft(
                "source:audio-reliability", "Audio device restored", "Playback can resume.", None,
            ),
        },
    ];
    let results = commands.iter().cloned().map(|command| ledger.execute_protocol_mutation(command).unwrap()).collect::<Vec<_>>();
    let expected_trace = notification_trace(&results);
    json!({ "snapshotResponse": snapshot_response, "commands": commands, "results": results, "expectedTrace": expected_trace })
}

fn operation_trace(results: &[OperationMutationResult]) -> Value {
    Value::Array(results.iter().map(|result| {
        let snapshot = operation_snapshot(result);
        let mut states = snapshot.active.iter().chain(&snapshot.recent).map(|entry| json!({
            "operationId": entry.operation_id, "state": entry.state
        })).collect::<Vec<_>>();
        states.sort_by_key(|entry| entry["operationId"].as_str().unwrap().to_owned());
        json!({ "revision": snapshot.catalogue_revision, "states": states })
    }).collect())
}

fn notification_trace(results: &[NotificationMutationResult]) -> Value {
    Value::Array(results.iter().map(|result| {
        let snapshot = notification_snapshot(result);
        let mut records = snapshot.page.records.iter().map(|record| json!({
            "notificationId": record.notification_id, "readState": record.read_state
        })).collect::<Vec<_>>();
        records.sort_by_key(|entry| entry["notificationId"].as_str().unwrap().to_owned());
        json!({ "revision": snapshot.ledger_revision, "unseenCount": snapshot.unseen_count, "records": records })
    }).collect())
}

fn operation_snapshot(result: &OperationMutationResult) -> &OperationSnapshot {
    match result { OperationMutationResult::Committed { snapshot, .. } | OperationMutationResult::Rejected { snapshot, .. } => snapshot }
}

fn notification_snapshot(result: &NotificationMutationResult) -> &NotificationSnapshot {
    match result { NotificationMutationResult::Committed { snapshot, .. } | NotificationMutationResult::Rejected { snapshot, .. } => snapshot }
}

fn operation_authority() -> OperationAuthorityProjection {
    OperationAuthorityProjection { authority_id: id("authority:loophole-render"), authority_epoch: 4 }
}

fn notification_authority() -> NotificationAuthorityProjection {
    NotificationAuthorityProjection { authority_id: id("authority:loophole-notifications"), authority_epoch: 3 }
}

fn notification_draft(source: &str, title: &str, summary: &str, action: Option<&str>) -> NotificationDraftProjection {
    NotificationDraftProjection {
        source_id: id(source), severity: NotificationSeverityProjection::Success,
        title: title.into(), summary: summary.into(), cause_id: None,
        actions: action.into_iter().map(|reference| NotificationActionProjection {
            reference_id: id::<NotificationActionReferenceId>(reference), label: "Reveal".into(),
        }).collect(), replacement_key: None, producer_token: None,
        retention_class: NotificationRetentionClassProjection::Standard, presentation_time_unix_ms: None,
    }
}

fn id<T>(value: &str) -> T where T: std::str::FromStr, T::Err: std::fmt::Debug { value.parse().unwrap() }
