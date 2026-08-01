use longhorn_operation::{
    OperationCancellationOutcome, OperationCancellationRequest, OperationCancellationSupport,
    OperationCatalogueError, OperationLabel, OperationRegistration, OperationState,
};

use super::support::*;

fn cancellation(
    catalogue: &longhorn_operation::OperationCatalogue,
    id: &str,
) -> OperationCancellationRequest {
    let operation = catalogue.operation(&operation_id(id)).unwrap();
    OperationCancellationRequest::new(
        catalogue.authority().clone(),
        operation.operation_id().clone(),
        operation.revision(),
    )
}

#[test]
fn running_cancellation_acceptance_preserves_all_three_race_terminals() {
    for terminal in [
        OperationState::Succeeded,
        OperationState::Failed,
        OperationState::Cancelled,
    ] {
        let mut catalogue = catalogue("authority:cancel-race", 1);
        catalogue
            .register(registration(
                &catalogue,
                "operation:race",
                "test",
                "Race",
                OperationState::Running,
            ))
            .unwrap();
        let accepted = catalogue
            .request_cancellation(cancellation(&catalogue, "operation:race"))
            .unwrap();
        assert_eq!(accepted.outcome(), OperationCancellationOutcome::Accepted);
        assert_eq!(accepted.committed_state(), OperationState::Cancelling);
        assert!(!accepted.committed_state().is_terminal());
        catalogue
            .transition(transition(&catalogue, "operation:race", terminal))
            .unwrap();
        assert_eq!(
            catalogue
                .operation(&operation_id("operation:race"))
                .unwrap()
                .state(),
            terminal
        );
    }
}

#[test]
fn repeated_queued_unsupported_terminal_and_stale_cancellation_are_exact() {
    let mut running = catalogue("authority:cancel", 1);
    running
        .register(registration(
            &running,
            "operation:running",
            "test",
            "Running",
            OperationState::Running,
        ))
        .unwrap();
    let stale_request = cancellation(&running, "operation:running");
    running.request_cancellation(stale_request.clone()).unwrap();
    let before_repeat = running.clone();
    let repeated = running
        .request_cancellation(cancellation(&running, "operation:running"))
        .unwrap();
    assert_eq!(
        repeated.outcome(),
        OperationCancellationOutcome::AlreadyRequested
    );
    assert_eq!(running, before_repeat);
    assert!(matches!(
        running.request_cancellation(stale_request),
        Err(OperationCatalogueError::OperationRevisionMismatch { .. })
    ));
    assert_eq!(running, before_repeat);

    let mut queued = catalogue("authority:queued-cancel", 1);
    queued
        .register(registration(
            &queued,
            "operation:queued",
            "test",
            "Queued",
            OperationState::Queued,
        ))
        .unwrap();
    let queued_receipt = queued
        .request_cancellation(cancellation(&queued, "operation:queued"))
        .unwrap();
    assert_eq!(
        queued_receipt.outcome(),
        OperationCancellationOutcome::Accepted
    );
    assert_eq!(queued_receipt.committed_state(), OperationState::Cancelled);

    let terminal_before = queued.clone();
    let terminal = queued
        .request_cancellation(cancellation(&queued, "operation:queued"))
        .unwrap();
    assert_eq!(terminal.outcome(), OperationCancellationOutcome::Terminal);
    assert_eq!(queued, terminal_before);

    let mut unsupported = catalogue("authority:unsupported-cancel", 1);
    let request = OperationRegistration::new(
        unsupported.authority().clone(),
        unsupported.revision(),
        operation_id("operation:unsupported"),
        kind_id("test"),
        None,
        OperationLabel::new("Unsupported").unwrap(),
        OperationState::Running,
        OperationCancellationSupport::Unsupported,
        None,
    );
    unsupported.register(request).unwrap();
    let before = unsupported.clone();
    let receipt = unsupported
        .request_cancellation(cancellation(&unsupported, "operation:unsupported"))
        .unwrap();
    assert_eq!(receipt.outcome(), OperationCancellationOutcome::Unsupported);
    assert_eq!(unsupported, before);
}
