use std::error::Error;

use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationPhaseId, OperationRevision,
};
use longhorn_operation::{
    OperationAuthorityEpoch, OperationAuthorityProjection, OperationCancellationCommand,
    OperationCancellationSupportProjection, OperationCatalogue, OperationCatalogueLimits,
    OperationChangedEvent, OperationMutationCommand, OperationOverallProgressProjection,
    OperationPhaseProgressProjection, OperationProtocolVersion, OperationSnapshot,
    OperationSnapshotQuery, OperationSnapshotResponse, OperationStateProjection,
};
use serde_json::{json, to_value};

pub fn render() -> Result<String, Box<dyn Error>> {
    let mut catalogue = OperationCatalogue::new(
        id::<OperationAuthorityId>("authority:fixture"),
        OperationAuthorityEpoch::new(7)?,
        OperationCatalogueLimits::new(8, 4, 16_384)?,
    );
    let authority = authority();
    let query = OperationSnapshotQuery {
        protocol_version: OperationProtocolVersion::CURRENT,
        request_id: id("request:snapshot"),
    };
    let initial_snapshot = OperationSnapshot::from_catalogue(&catalogue)?;
    let snapshot_response = OperationSnapshotResponse {
        request_id: query.request_id.clone(),
        snapshot: initial_snapshot,
    };

    let register = OperationMutationCommand::Register {
        request_id: id("request:register"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority.clone(),
        expected_catalogue_revision: OperationCatalogueRevision::INITIAL,
        operation_id: id("operation:scan"),
        kind_id: id("example.long-running-scan"),
        scope_id: None,
        label: "Scan plug-ins".into(),
        initial_state: OperationStateProjection::Running,
        cancellation_support: OperationCancellationSupportProjection::Supported,
        retry_of: None,
    };
    let registered = catalogue.execute_protocol_mutation(register.clone())?;
    let progress = OperationMutationCommand::Progress {
        request_id: id("request:progress"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority.clone(),
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::INITIAL,
        overall: OperationOverallProgressProjection::Units {
            completed: 2.0,
            total: 10.0,
        },
        phase: Some(OperationPhaseProgressProjection {
            phase_id: id::<OperationPhaseId>("phase:introspect"),
            label: "Introspecting".into(),
            completed: 2.0,
            total: 10.0,
        }),
    };
    let progressed = catalogue.execute_protocol_mutation(progress.clone())?;
    let cancel = OperationCancellationCommand {
        request_id: id("request:cancel"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority.clone(),
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::new(1),
    };
    let cancelled = catalogue.execute_protocol_cancellation(cancel.clone())?;
    let terminal = OperationMutationCommand::Transition {
        request_id: id("request:terminal"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority: authority.clone(),
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::new(2),
        next_state: OperationStateProjection::Succeeded,
    };
    let terminal_result = catalogue.execute_protocol_mutation(terminal.clone())?;
    let stale = OperationMutationCommand::Transition {
        request_id: id("request:stale"),
        protocol_version: OperationProtocolVersion::CURRENT,
        authority,
        operation_id: id("operation:scan"),
        expected_operation_revision: OperationRevision::INITIAL,
        next_state: OperationStateProjection::Failed,
    };
    let stale_result = catalogue.execute_protocol_mutation(stale.clone())?;

    let fixture = json!({
        "protocolVersion": 1,
        "snapshotQuery": to_value(query)?,
        "snapshotResponse": to_value(snapshot_response)?,
        "mutationCommands": [to_value(register)?, to_value(progress)?, to_value(terminal)?, to_value(stale)?],
        "mutationResults": [to_value(&registered)?, to_value(&progressed)?, to_value(&terminal_result)?, to_value(&stale_result)?],
        "cancellationCommand": to_value(cancel)?,
        "cancellationResult": to_value(&cancelled)?,
        "changedEvents": [
            to_value(OperationChangedEvent::from_mutation(&registered))?,
            to_value(OperationChangedEvent::from_mutation(&progressed))?,
            to_value(OperationChangedEvent::from_cancellation(&cancelled))?,
            to_value(OperationChangedEvent::from_mutation(&terminal_result))?
        ],
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownState": "paused",
            "unknownMutationKind": "executeProductPayload",
            "unknownRejectionCode": "futureRejection",
            "unknownMutationStatus": "uncertain",
            "unknownCancellationStatus": "stopped",
            "unknownChangedKind": "product"
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}

fn authority() -> OperationAuthorityProjection {
    OperationAuthorityProjection {
        authority_id: id("authority:fixture"),
        authority_epoch: 7,
    }
}

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("operation fixture id must be valid")
}
