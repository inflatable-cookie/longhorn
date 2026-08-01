use std::error::Error;

use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationLedgerRevision,
};
use longhorn_notifications::{
    NotificationActionProjection, NotificationAuthorityEpoch, NotificationAuthorityProjection,
    NotificationChangedEvent, NotificationDraftProjection, NotificationLedger,
    NotificationLedgerLimits, NotificationMutationCommand, NotificationProtocolVersion,
    NotificationRetentionClassProjection, NotificationSeverityProjection,
    NotificationSnapshotQuery,
};
use serde_json::{json, to_value};

pub fn render() -> Result<String, Box<dyn Error>> {
    let mut ledger = NotificationLedger::new(
        id::<NotificationAuthorityId>("authority:fixture"),
        NotificationAuthorityEpoch::new(7)?,
        NotificationLedgerLimits::new(8, 16_384)?,
    );
    let snapshot_query = NotificationSnapshotQuery {
        protocol_version: NotificationProtocolVersion::CURRENT,
        request_id: id("request:snapshot"),
        offset: 0,
        limit: 4,
    };
    let snapshot_response = ledger.execute_protocol_snapshot(snapshot_query.clone())?;
    let add = add_command(
        "request:add",
        NotificationLedgerRevision::INITIAL,
        "notification:scan",
        "Scan complete",
    );
    let added = ledger.execute_protocol_mutation(add.clone())?;
    let seen = NotificationMutationCommand::MarkSeen {
        request_id: id("request:seen"),
        protocol_version: NotificationProtocolVersion::CURRENT,
        authority: authority(),
        expected_ledger_revision: NotificationLedgerRevision::new(1),
        notification_id: id("notification:scan"),
    };
    let seen_result = ledger.execute_protocol_mutation(seen.clone())?;
    let dismiss = NotificationMutationCommand::Dismiss {
        request_id: id("request:dismiss"),
        protocol_version: NotificationProtocolVersion::CURRENT,
        authority: authority(),
        expected_ledger_revision: NotificationLedgerRevision::new(2),
        notification_id: id("notification:scan"),
    };
    let dismissed = ledger.execute_protocol_mutation(dismiss.clone())?;
    let stale = add_command(
        "request:stale",
        NotificationLedgerRevision::INITIAL,
        "notification:stale",
        "Stale mutation",
    );
    let stale_result = ledger.execute_protocol_mutation(stale.clone())?;

    let fixture = json!({
        "protocolVersion": 1,
        "snapshotQuery": to_value(snapshot_query)?,
        "snapshotResponse": to_value(snapshot_response)?,
        "mutationCommands": [to_value(add)?, to_value(seen)?, to_value(dismiss)?, to_value(stale)?],
        "mutationResults": [to_value(&added)?, to_value(&seen_result)?, to_value(&dismissed)?, to_value(&stale_result)?],
        "changedEvents": [
            to_value(NotificationChangedEvent::from_mutation(&added))?,
            to_value(NotificationChangedEvent::from_mutation(&seen_result))?,
            to_value(NotificationChangedEvent::from_mutation(&dismissed))?
        ],
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownSeverity": "notice",
            "unknownMutationKind": "executeAction",
            "unknownRejectionCode": "futureRejection",
            "unknownMutationStatus": "uncertain",
            "unknownChangedKind": "product"
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}

fn add_command(
    request_id: &str,
    expected_ledger_revision: NotificationLedgerRevision,
    notification_id: &str,
    title: &str,
) -> NotificationMutationCommand {
    NotificationMutationCommand::Add {
        request_id: id(request_id),
        protocol_version: NotificationProtocolVersion::CURRENT,
        authority: authority(),
        expected_ledger_revision,
        notification_id: id(notification_id),
        draft: NotificationDraftProjection {
            source_id: id("source:fixture"),
            severity: NotificationSeverityProjection::Success,
            title: title.into(),
            summary: "The fixture operation completed.".into(),
            cause_id: None,
            actions: vec![NotificationActionProjection {
                reference_id: id::<NotificationActionReferenceId>("action:open-result"),
                label: "Open result".into(),
            }],
            replacement_key: None,
            producer_token: None,
            retention_class: NotificationRetentionClassProjection::Standard,
            presentation_time_unix_ms: Some(1_700_000_000_000),
        },
    }
}

fn authority() -> NotificationAuthorityProjection {
    NotificationAuthorityProjection {
        authority_id: id("authority:fixture"),
        authority_epoch: 7,
    }
}

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value
        .parse()
        .expect("notification fixture id must be valid")
}
