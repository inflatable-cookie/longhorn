use longhorn_operation::{
    OperationCancellationSupport, OperationCatalogueError, OperationLabel, OperationRegistration,
    OperationState, OperationTeardown, OperationTeardownOutcome, OperationTeardownResolution,
    OperationTeardownResolutionOutcome,
};

use super::support::*;

fn retry_registration(
    catalogue: &longhorn_operation::OperationCatalogue,
    id: &str,
    source: &str,
) -> OperationRegistration {
    OperationRegistration::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        operation_id(id),
        kind_id("retry"),
        None,
        OperationLabel::new("Retry").unwrap(),
        OperationState::Queued,
        OperationCancellationSupport::Supported,
        Some(operation_id(source)),
    )
}

#[test]
fn retry_lineage_rejects_active_or_self_allows_terminal_or_dismissed_and_never_reopens() {
    let mut catalogue = catalogue("authority:retry", 1);
    catalogue
        .register(registration(
            &catalogue,
            "operation:source",
            "test",
            "Source",
            OperationState::Running,
        ))
        .unwrap();
    let before = catalogue.clone();
    assert_eq!(
        catalogue.register(retry_registration(
            &catalogue,
            "operation:retry",
            "operation:source"
        )),
        Err(OperationCatalogueError::InvalidRetrySource {
            operation_id: operation_id("operation:source"),
            state: Some(OperationState::Running)
        })
    );
    assert_eq!(catalogue, before);

    catalogue
        .transition(transition(
            &catalogue,
            "operation:source",
            OperationState::Failed,
        ))
        .unwrap();
    let source_before = catalogue
        .operation(&operation_id("operation:source"))
        .unwrap()
        .clone();
    let retry = catalogue
        .register(retry_registration(
            &catalogue,
            "operation:retry",
            "operation:source",
        ))
        .unwrap();
    assert_ne!(
        retry.operation().operation_id(),
        &operation_id("operation:source")
    );
    assert_eq!(
        retry.operation().retry_of().unwrap().as_str(),
        "operation:source"
    );
    assert_eq!(
        catalogue
            .operation(&operation_id("operation:source"))
            .unwrap(),
        &source_before
    );

    // Dismissing the source must not prevent a consumer from retrying it later
    // (contract 015: retry eligibility is consumer policy).
    catalogue
        .dismiss_terminal(longhorn_operation::OperationDismissal::new(
            catalogue.authority().clone(),
            operation_id("operation:source"),
            source_before.revision(),
        ))
        .unwrap();
    assert!(
        catalogue
            .operation(&operation_id("operation:source"))
            .is_none()
    );
    let post_dismiss_retry = catalogue
        .register(retry_registration(
            &catalogue,
            "operation:retry-after-dismiss",
            "operation:source",
        ))
        .unwrap();
    assert_eq!(
        post_dismiss_retry.operation().retry_of(),
        Some(&operation_id("operation:source"))
    );

    // Self-retry is still rejected.
    assert_eq!(
        catalogue.register(retry_registration(
            &catalogue,
            "operation:self-retry",
            "operation:self-retry",
        )),
        Err(OperationCatalogueError::InvalidRetrySource {
            operation_id: operation_id("operation:self-retry"),
            state: None,
        })
    );
}

#[test]
fn teardown_is_complete_atomic_and_closes_the_authority() {
    let mut catalogue = catalogue("authority:teardown", 4);
    catalogue
        .register(registration(
            &catalogue,
            "operation:interrupt",
            "test",
            "Interrupt",
            OperationState::Running,
        ))
        .unwrap();
    catalogue
        .register(registration(
            &catalogue,
            "operation:transfer",
            "test",
            "Transfer",
            OperationState::Queued,
        ))
        .unwrap();

    let interrupt = catalogue
        .operation(&operation_id("operation:interrupt"))
        .unwrap();
    let incomplete = OperationTeardown::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        vec![OperationTeardownResolution::new(
            interrupt.operation_id().clone(),
            interrupt.revision(),
            OperationTeardownResolutionOutcome::Complete(OperationState::Interrupted),
        )],
    );
    let before = catalogue.clone();
    assert!(matches!(
        catalogue.teardown(incomplete),
        Err(OperationCatalogueError::MissingTeardownResolutions { .. })
    ));
    assert_eq!(catalogue, before);

    let interrupt = catalogue
        .operation(&operation_id("operation:interrupt"))
        .unwrap();
    let transfer = catalogue
        .operation(&operation_id("operation:transfer"))
        .unwrap();
    let target = cursor("authority:receiver", 1);
    let request = OperationTeardown::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        vec![
            OperationTeardownResolution::new(
                interrupt.operation_id().clone(),
                interrupt.revision(),
                OperationTeardownResolutionOutcome::Complete(OperationState::Interrupted),
            ),
            OperationTeardownResolution::new(
                transfer.operation_id().clone(),
                transfer.revision(),
                OperationTeardownResolutionOutcome::Transfer(target.clone()),
            ),
        ],
    );
    let receipt = catalogue.teardown(request).unwrap();
    assert_eq!(receipt.outcomes().len(), 2);
    assert!(matches!(
        receipt.outcomes()[0],
        OperationTeardownOutcome::Completed {
            state: OperationState::Interrupted,
            ..
        }
    ));
    assert!(
        matches!(&receipt.outcomes()[1], OperationTeardownOutcome::Transferred { target_authority, .. } if target_authority == &target)
    );
    assert!(catalogue.is_closed());
    assert!(catalogue.project().is_closed());
    assert!(catalogue.project().active().is_empty());
    assert!(
        catalogue
            .operation(&operation_id("operation:transfer"))
            .is_none()
    );

    let closed_before = catalogue.clone();
    assert_eq!(
        catalogue.register(registration(
            &catalogue,
            "operation:late",
            "test",
            "Late",
            OperationState::Running
        )),
        Err(OperationCatalogueError::AuthorityClosed)
    );
    assert_eq!(catalogue, closed_before);
}

#[test]
fn teardown_rejects_non_terminal_completion_without_mutation() {
    for invalid_state in [
        OperationState::Queued,
        OperationState::Running,
        OperationState::Cancelling,
    ] {
        let mut catalogue = catalogue("authority:teardown-invalid-terminal", 1);
        catalogue
            .register(registration(
                &catalogue,
                "operation:active",
                "test",
                "Active",
                OperationState::Running,
            ))
            .unwrap();
        let active = catalogue
            .operation(&operation_id("operation:active"))
            .unwrap();
        let request = OperationTeardown::new(
            catalogue.authority().clone(),
            catalogue.revision(),
            vec![OperationTeardownResolution::new(
                active.operation_id().clone(),
                active.revision(),
                OperationTeardownResolutionOutcome::Complete(invalid_state),
            )],
        );
        let before = catalogue.clone();
        assert_eq!(
            catalogue.teardown(request),
            Err(OperationCatalogueError::InvalidTeardownTerminal {
                state: invalid_state,
            })
        );
        assert_eq!(catalogue, before);
        assert!(!catalogue.is_closed());
    }
}

#[test]
fn teardown_enforces_lifecycle_edges_and_rejects_queued_succeeded_without_mutation() {
    let mut catalogue = catalogue("authority:teardown-edges", 1);
    catalogue
        .register(registration(
            &catalogue,
            "operation:queued",
            "test",
            "Queued",
            OperationState::Queued,
        ))
        .unwrap();
    let queued = catalogue
        .operation(&operation_id("operation:queued"))
        .unwrap();
    let illegal = OperationTeardown::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        vec![OperationTeardownResolution::new(
            queued.operation_id().clone(),
            queued.revision(),
            OperationTeardownResolutionOutcome::Complete(OperationState::Succeeded),
        )],
    );
    let before = catalogue.clone();
    assert_eq!(
        catalogue.teardown(illegal),
        Err(OperationCatalogueError::InvalidTransition {
            current: OperationState::Queued,
            next: OperationState::Succeeded,
        })
    );
    assert_eq!(catalogue, before);
    assert!(!catalogue.is_closed());

    // Legal queued teardown terminals (`Failed`, `Cancelled`, `Interrupted`) and
    // running `Succeeded` all commit cleanly.
    for legal_queued in [
        OperationState::Failed,
        OperationState::Cancelled,
        OperationState::Interrupted,
    ] {
        let mut legal_catalogue = super::support::catalogue("authority:teardown-legal", 1);
        legal_catalogue
            .register(registration(
                &legal_catalogue,
                "operation:queued",
                "test",
                "Queued",
                OperationState::Queued,
            ))
            .unwrap();
        legal_catalogue
            .register(registration(
                &legal_catalogue,
                "operation:running",
                "test",
                "Running",
                OperationState::Running,
            ))
            .unwrap();
        let queued = legal_catalogue
            .operation(&operation_id("operation:queued"))
            .unwrap();
        let running = legal_catalogue
            .operation(&operation_id("operation:running"))
            .unwrap();
        let request = OperationTeardown::new(
            legal_catalogue.authority().clone(),
            legal_catalogue.revision(),
            vec![
                OperationTeardownResolution::new(
                    queued.operation_id().clone(),
                    queued.revision(),
                    OperationTeardownResolutionOutcome::Complete(legal_queued),
                ),
                OperationTeardownResolution::new(
                    running.operation_id().clone(),
                    running.revision(),
                    OperationTeardownResolutionOutcome::Complete(OperationState::Succeeded),
                ),
            ],
        );
        let receipt = legal_catalogue.teardown(request).unwrap();
        assert_eq!(receipt.outcomes().len(), 2);
        assert!(legal_catalogue.is_closed());
    }
}

#[test]
fn teardown_rejects_transfer_to_self_without_mutation() {
    let mut catalogue = catalogue("authority:teardown-self", 3);
    catalogue
        .register(registration(
            &catalogue,
            "operation:self-transfer",
            "test",
            "Self transfer",
            OperationState::Running,
        ))
        .unwrap();
    let op = catalogue
        .operation(&operation_id("operation:self-transfer"))
        .unwrap();
    let op_id = op.operation_id().clone();
    let op_revision = op.revision();
    let request = OperationTeardown::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        vec![OperationTeardownResolution::new(
            op_id.clone(),
            op_revision,
            OperationTeardownResolutionOutcome::Transfer(catalogue.authority().clone()),
        )],
    );
    let before = catalogue.clone();
    assert_eq!(
        catalogue.teardown(request),
        Err(OperationCatalogueError::TeardownTransferToSelf {
            operation_id: operation_id("operation:self-transfer"),
        })
    );
    assert_eq!(catalogue, before);
    assert!(!catalogue.is_closed());

    // Transferring to the same authority id at a newer epoch is allowed.
    let next_epoch = cursor("authority:teardown-self", 4);
    let valid = OperationTeardown::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        vec![OperationTeardownResolution::new(
            op_id,
            op_revision,
            OperationTeardownResolutionOutcome::Transfer(next_epoch.clone()),
        )],
    );
    let receipt = catalogue.teardown(valid).unwrap();
    assert!(matches!(
        &receipt.outcomes()[0],
        OperationTeardownOutcome::Transferred { target_authority, .. }
            if target_authority == &next_epoch
    ));
    assert!(catalogue.is_closed());
}

#[test]
fn renderer_detach_is_a_read_only_projection_lifecycle() {
    let mut catalogue = catalogue("authority:renderer", 1);
    catalogue
        .register(registration(
            &catalogue,
            "operation:host",
            "test",
            "Host work",
            OperationState::Running,
        ))
        .unwrap();
    let before_detach = catalogue.clone();
    let mounted_projection = catalogue.project();
    drop(mounted_projection);
    assert_eq!(catalogue, before_detach);
    assert_eq!(
        catalogue.project().active()[0].state(),
        OperationState::Running
    );
}
