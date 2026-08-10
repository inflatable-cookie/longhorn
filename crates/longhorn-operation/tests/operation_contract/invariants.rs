use longhorn_core::{OperationCatalogueRevision, OperationId, OperationKindId};
use longhorn_operation::{
    MAXIMUM_OPERATION_ENCODED_WEIGHT, MAXIMUM_OPERATION_LABEL_BYTES,
    MAXIMUM_OPERATION_PHASE_LABEL_BYTES, MAXIMUM_RETAINED_OPERATIONS, OperationAuthorityCursor,
    OperationAuthorityEpoch, OperationCancellationSupport, OperationCatalogueError,
    OperationCatalogueLimits, OperationCatalogueLimitsError, OperationLabel, OperationLabelError,
    OperationPhaseLabel, OperationPhaseLabelError, OperationRegistration, OperationState,
    OperationTransition,
};

use super::support::*;

#[test]
fn stale_foreign_duplicate_unknown_and_full_attempts_preserve_exact_state() {
    let mut catalogue = catalogue_with_limit("authority:invariants", 3, 1);
    catalogue
        .register(registration(
            &catalogue,
            "operation:one",
            "test",
            "One",
            OperationState::Running,
        ))
        .unwrap();

    let before = catalogue.clone();
    let stale = catalogue.register(stale_registration(&catalogue, "operation:stale"));
    assert!(matches!(
        stale,
        Err(OperationCatalogueError::CatalogueRevisionMismatch { .. })
    ));
    assert_eq!(catalogue, before);

    let duplicate = catalogue.register(OperationRegistration::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        operation_id("operation:one"),
        kind_id("test"),
        None,
        OperationLabel::new("Duplicate").unwrap(),
        OperationState::Running,
        OperationCancellationSupport::Supported,
        None,
    ));
    assert!(matches!(
        duplicate,
        Err(OperationCatalogueError::DuplicateOperation { .. })
    ));
    assert_eq!(catalogue, before);

    let full = catalogue.register(registration(
        &catalogue,
        "operation:full",
        "test",
        "Full",
        OperationState::Running,
    ));
    assert_eq!(
        full,
        Err(OperationCatalogueError::ActiveLimitReached { maximum: 1 })
    );
    assert_eq!(catalogue, before);

    let foreign = OperationTransition::new(
        cursor("authority:other", 3),
        operation_id("operation:one"),
        catalogue
            .operation(&operation_id("operation:one"))
            .unwrap()
            .revision(),
        OperationState::Succeeded,
    );
    assert!(matches!(
        catalogue.transition(foreign),
        Err(OperationCatalogueError::AuthorityMismatch { .. })
    ));
    assert_eq!(catalogue, before);

    let wrong_epoch = OperationTransition::new(
        OperationAuthorityCursor::new(
            authority_id("authority:invariants"),
            OperationAuthorityEpoch::new(4).unwrap(),
        ),
        operation_id("operation:one"),
        catalogue
            .operation(&operation_id("operation:one"))
            .unwrap()
            .revision(),
        OperationState::Succeeded,
    );
    assert!(matches!(
        catalogue.transition(wrong_epoch),
        Err(OperationCatalogueError::AuthorityEpochMismatch { .. })
    ));
    assert_eq!(catalogue, before);

    let stale_revision =
        transition_at_revision(&catalogue, "operation:one", 7, OperationState::Succeeded);
    assert!(matches!(
        catalogue.transition(stale_revision),
        Err(OperationCatalogueError::OperationRevisionMismatch { .. })
    ));
    assert_eq!(catalogue, before);

    let unknown = OperationTransition::new(
        catalogue.authority().clone(),
        operation_id("operation:missing"),
        longhorn_core::OperationRevision::INITIAL,
        OperationState::Succeeded,
    );
    assert_eq!(
        catalogue.transition(unknown),
        Err(OperationCatalogueError::UnknownOperation {
            operation_id: operation_id("operation:missing"),
        })
    );
    assert_eq!(catalogue, before);
}

#[test]
fn identity_label_limit_and_epoch_bounds_fail_closed() {
    assert!(OperationId::new("operation:valid-1").is_ok());
    assert!(OperationKindId::new("example.long-running-scan").is_ok());
    assert!(OperationId::new("Operation Invalid").is_err());
    assert!(OperationAuthorityEpoch::new(0).is_err());
    assert_eq!(OperationLabel::new(""), Err(OperationLabelError::Empty));
    assert_eq!(
        OperationLabel::new("x".repeat(MAXIMUM_OPERATION_LABEL_BYTES + 1)),
        Err(OperationLabelError::TooLong {
            maximum: MAXIMUM_OPERATION_LABEL_BYTES,
            actual: MAXIMUM_OPERATION_LABEL_BYTES + 1,
        })
    );
    assert_eq!(
        OperationCatalogueLimits::new(0, 0, 0),
        Err(OperationCatalogueLimitsError::ZeroActiveLimit)
    );
    assert_eq!(
        OperationCatalogueLimits::new(MAXIMUM_RETAINED_OPERATIONS + 1, 0, 0),
        Err(OperationCatalogueLimitsError::TooManyOperations)
    );
    assert_eq!(
        OperationPhaseLabel::new("x".repeat(MAXIMUM_OPERATION_PHASE_LABEL_BYTES + 1)),
        Err(OperationPhaseLabelError::TooLong {
            maximum: MAXIMUM_OPERATION_PHASE_LABEL_BYTES,
            actual: MAXIMUM_OPERATION_PHASE_LABEL_BYTES + 1,
        })
    );
    assert_eq!(
        OperationCatalogueLimits::new(1, 0, MAXIMUM_OPERATION_ENCODED_WEIGHT + 1),
        Err(OperationCatalogueLimitsError::EncodedWeightTooLarge {
            maximum: MAXIMUM_OPERATION_ENCODED_WEIGHT,
            actual: MAXIMUM_OPERATION_ENCODED_WEIGHT + 1,
        })
    );
}

#[test]
fn registration_receipt_distinguishes_catalogue_and_operation_revisions() {
    let mut catalogue = catalogue("authority:revision", 2);
    let first = catalogue
        .register(registration(
            &catalogue,
            "operation:first",
            "test",
            "First",
            OperationState::Running,
        ))
        .unwrap();
    assert_eq!(
        first.previous_catalogue_revision(),
        OperationCatalogueRevision::INITIAL
    );
    assert_eq!(first.committed_catalogue_revision().get(), 1);
    assert_eq!(first.operation().revision().get(), 0);

    let transitioned = catalogue
        .transition(transition(
            &catalogue,
            "operation:first",
            OperationState::Succeeded,
        ))
        .unwrap();
    assert_eq!(transitioned.previous_operation_revision().get(), 0);
    assert_eq!(transitioned.committed_operation_revision().get(), 1);
    assert_eq!(transitioned.previous_catalogue_revision().get(), 1);
    assert_eq!(transitioned.committed_catalogue_revision().get(), 2);
}
