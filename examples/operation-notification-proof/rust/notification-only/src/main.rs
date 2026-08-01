use longhorn_core::{NotificationAuthorityId, NotificationLedgerRevision};
use longhorn_notifications::{
    NotificationAuthorityEpoch, NotificationAuthorityProjection, NotificationDraftProjection,
    NotificationLedger, NotificationLedgerLimits, NotificationMutationCommand,
    NotificationMutationResult, NotificationProtocolVersion, NotificationRetentionClassProjection,
    NotificationSeverityProjection, NotificationSnapshot, NotificationSnapshotQuery,
};
use serde_json::{Value, json};

fn main() {
    let mut ledger = NotificationLedger::new(
        id::<NotificationAuthorityId>("authority:reliability"),
        NotificationAuthorityEpoch::new(2).unwrap(),
        NotificationLedgerLimits::new(8, 16_384).unwrap(),
    );
    let snapshot_response = ledger.execute_protocol_snapshot(NotificationSnapshotQuery {
        protocol_version: NotificationProtocolVersion::CURRENT,
        request_id: id("request:snapshot"),
        offset: 0,
        limit: 8,
    }).unwrap();
    let commands = vec![
        NotificationMutationCommand::Add {
            request_id: id("request:add"),
            protocol_version: NotificationProtocolVersion::CURRENT,
            authority: authority(),
            expected_ledger_revision: NotificationLedgerRevision::INITIAL,
            notification_id: id("notification:connection"),
            draft: draft("Backend connection restored"),
        },
        NotificationMutationCommand::MarkSeen {
            request_id: id("request:seen"),
            protocol_version: NotificationProtocolVersion::CURRENT,
            authority: authority(),
            expected_ledger_revision: NotificationLedgerRevision::new(1),
            notification_id: id("notification:connection"),
        },
        NotificationMutationCommand::Dismiss {
            request_id: id("request:dismiss"),
            protocol_version: NotificationProtocolVersion::CURRENT,
            authority: authority(),
            expected_ledger_revision: NotificationLedgerRevision::new(2),
            notification_id: id("notification:connection"),
        },
    ];
    let results = commands.iter().cloned().map(|command| ledger.execute_protocol_mutation(command).unwrap()).collect::<Vec<_>>();
    let expected_trace = trace(&results);
    println!("{}", json!({
        "shape": "notification-only",
        "publicTrace": expected_trace,
        "rendererFixture": {
            "snapshotResponse": snapshot_response,
            "commands": commands,
            "results": results,
            "expectedTrace": expected_trace
        }
    }));
}

fn trace(results: &[NotificationMutationResult]) -> Value {
    Value::Array(results.iter().map(|result| {
        let snapshot = snapshot(result);
        let mut records = snapshot.page.records.iter().map(|record| json!({
            "notificationId": record.notification_id,
            "readState": record.read_state
        })).collect::<Vec<_>>();
        records.sort_by_key(|entry| entry["notificationId"].as_str().unwrap().to_owned());
        json!({ "revision": snapshot.ledger_revision, "unseenCount": snapshot.unseen_count, "records": records })
    }).collect())
}

fn snapshot(result: &NotificationMutationResult) -> &NotificationSnapshot {
    match result { NotificationMutationResult::Committed { snapshot, .. } | NotificationMutationResult::Rejected { snapshot, .. } => snapshot }
}

fn draft(title: &str) -> NotificationDraftProjection {
    NotificationDraftProjection {
        source_id: id("source:reliability"), severity: NotificationSeverityProjection::Success,
        title: title.into(), summary: "The local service is available again.".into(), cause_id: None,
        actions: Vec::new(), replacement_key: None, producer_token: None,
        retention_class: NotificationRetentionClassProjection::Standard, presentation_time_unix_ms: None,
    }
}

fn authority() -> NotificationAuthorityProjection {
    NotificationAuthorityProjection { authority_id: id("authority:reliability"), authority_epoch: 2 }
}

fn id<T>(value: &str) -> T where T: std::str::FromStr, T::Err: std::fmt::Debug { value.parse().unwrap() }
