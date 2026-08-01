//! Strict protocol projection and checked-rejection contract tests.

use longhorn_core::{NotificationAuthorityId, NotificationLedgerRevision};
use longhorn_notifications::{
    NotificationAuthorityEpoch, NotificationAuthorityProjection, NotificationChangedEvent,
    NotificationDraftProjection, NotificationLedger, NotificationLedgerLimits,
    NotificationMutationCommand, NotificationMutationResult, NotificationProtocolVersion,
    NotificationReadStateProjection, NotificationRejectionCode,
    NotificationRetentionClassProjection, NotificationSeverityProjection,
    NotificationSnapshotQuery,
};

#[test]
fn protocol_commit_projects_exact_event_and_snapshot() {
    let mut ledger = ledger();
    let result = ledger
        .execute_protocol_mutation(add("request:add", NotificationLedgerRevision::INITIAL))
        .expect("protocol mutation should project");
    let NotificationMutationResult::Committed {
        request_id,
        snapshot,
        ..
    } = &result
    else {
        panic!("add should commit");
    };
    assert_eq!(request_id.to_string(), "request:add");
    assert_eq!(snapshot.ledger_revision, NotificationLedgerRevision::new(1));
    assert_eq!(snapshot.page.records.len(), 1);
    assert_eq!(
        snapshot.page.records[0].read_state,
        NotificationReadStateProjection::Unseen
    );
    let event = NotificationChangedEvent::from_mutation(&result).expect("commit advances revision");
    assert_eq!(
        event.previous_ledger_revision,
        NotificationLedgerRevision::INITIAL
    );
    assert_eq!(
        event.committed_ledger_revision,
        NotificationLedgerRevision::new(1)
    );
}

#[test]
fn stale_mutation_is_checked_and_carries_fresh_authority() {
    let mut ledger = ledger();
    ledger
        .execute_protocol_mutation(add("request:first", NotificationLedgerRevision::INITIAL))
        .expect("first add should project");
    let result = ledger
        .execute_protocol_mutation(add("request:stale", NotificationLedgerRevision::INITIAL))
        .expect("stale add is a checked result");
    let NotificationMutationResult::Rejected {
        snapshot,
        rejection,
        ..
    } = result
    else {
        panic!("stale add should reject");
    };
    assert_eq!(
        rejection.code,
        NotificationRejectionCode::LedgerRevisionMismatch
    );
    assert!(rejection.refresh_required);
    assert_eq!(snapshot.ledger_revision, NotificationLedgerRevision::new(1));
}

#[test]
fn snapshot_queries_are_paged_and_version_checked() {
    let ledger = ledger();
    let response = ledger
        .execute_protocol_snapshot(NotificationSnapshotQuery {
            protocol_version: NotificationProtocolVersion::CURRENT,
            request_id: id("request:snapshot"),
            offset: 0,
            limit: 1,
        })
        .expect("current query should project");
    assert_eq!(response.snapshot.page.offset, 0);

    let future = serde_json::from_value(serde_json::json!({
        "protocolVersion": 2,
        "requestId": "request:future",
        "offset": 0,
        "limit": 1
    }))
    .expect("wire shape is structurally valid");
    assert!(ledger.execute_protocol_snapshot(future).is_err());
}

fn ledger() -> NotificationLedger {
    NotificationLedger::new(
        id::<NotificationAuthorityId>("authority:test"),
        NotificationAuthorityEpoch::new(1).expect("epoch is valid"),
        NotificationLedgerLimits::new(8, 16_384).expect("limits are valid"),
    )
}

fn add(request_id: &str, revision: NotificationLedgerRevision) -> NotificationMutationCommand {
    NotificationMutationCommand::Add {
        request_id: id(request_id),
        protocol_version: NotificationProtocolVersion::CURRENT,
        authority: NotificationAuthorityProjection {
            authority_id: id("authority:test"),
            authority_epoch: 1,
        },
        expected_ledger_revision: revision,
        notification_id: id(if request_id == "request:first" {
            "notification:first"
        } else {
            "notification:test"
        }),
        draft: NotificationDraftProjection {
            source_id: id("source:test"),
            severity: NotificationSeverityProjection::Info,
            title: "Test notification".into(),
            summary: "Protocol fixture".into(),
            cause_id: None,
            actions: Vec::new(),
            replacement_key: None,
            producer_token: None,
            retention_class: NotificationRetentionClassProjection::Standard,
            presentation_time_unix_ms: None,
        },
    }
}

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("test id is valid")
}
