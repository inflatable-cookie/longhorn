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
fn retry_lineage_requires_a_retained_terminal_and_never_reopens_it() {
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
