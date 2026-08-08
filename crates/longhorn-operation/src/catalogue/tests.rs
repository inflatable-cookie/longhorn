//! Unit tests for catalogue overflow guards.

use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationId, OperationKindId,
    OperationRevision,
};

use crate::{
    OperationAuthorityEpoch, OperationCancellationSupport, OperationCatalogueError,
    OperationCatalogueLimits, OperationLabel, OperationRegistration, OperationSequence,
    OperationState, OperationTransition,
};

use super::OperationCatalogue;

fn catalogue() -> OperationCatalogue {
    OperationCatalogue::new(
        OperationAuthorityId::new("authority:test").unwrap(),
        OperationAuthorityEpoch::new(1).unwrap(),
        OperationCatalogueLimits::default(),
    )
}

fn registration(catalogue: &OperationCatalogue) -> OperationRegistration {
    OperationRegistration::new(
        catalogue.authority.clone(),
        catalogue.revision,
        OperationId::new("operation:overflow").unwrap(),
        OperationKindId::new("test").unwrap(),
        None,
        OperationLabel::new("Overflow").unwrap(),
        OperationState::Running,
        OperationCancellationSupport::Supported,
        None,
    )
}

#[test]
fn revision_overflow_rejects_without_mutation() {
    let mut catalogue = catalogue();
    catalogue.revision = OperationCatalogueRevision::new(u64::MAX);
    let before = catalogue.clone();
    let request = registration(&catalogue);
    assert_eq!(
        catalogue.register(request),
        Err(OperationCatalogueError::CatalogueRevisionOverflow)
    );
    assert_eq!(catalogue, before);
}

#[test]
fn sequence_overflow_rejects_without_mutation() {
    let mut catalogue = catalogue();
    catalogue.next_sequence = OperationSequence::new(u64::MAX).unwrap();
    let before = catalogue.clone();
    let request = registration(&catalogue);
    assert_eq!(
        catalogue.register(request),
        Err(OperationCatalogueError::SequenceOverflow)
    );
    assert_eq!(catalogue, before);
}

#[test]
fn operation_revision_overflow_rejects_without_mutation() {
    let mut catalogue = catalogue();
    let request = registration(&catalogue);
    catalogue.register(request).unwrap();
    catalogue.operations[0].set_revision_for_test(OperationRevision::new(u64::MAX));
    let before = catalogue.clone();
    let request = OperationTransition::new(
        catalogue.authority.clone(),
        OperationId::new("operation:overflow").unwrap(),
        OperationRevision::new(u64::MAX),
        OperationState::Succeeded,
    );
    assert_eq!(
        catalogue.transition(request),
        Err(OperationCatalogueError::OperationRevisionOverflow)
    );
    assert_eq!(catalogue, before);
}
