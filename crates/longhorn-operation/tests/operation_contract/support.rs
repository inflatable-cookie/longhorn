use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationId, OperationKindId,
    OperationRevision, OperationScopeId,
};
use longhorn_operation::{
    OperationAuthorityCursor, OperationAuthorityEpoch, OperationCancellationSupport,
    OperationCatalogue, OperationCatalogueLimits, OperationLabel, OperationRegistration,
    OperationState, OperationTransition,
};

pub fn authority_id(value: &str) -> OperationAuthorityId {
    OperationAuthorityId::new(value).unwrap()
}

pub fn operation_id(value: &str) -> OperationId {
    OperationId::new(value).unwrap()
}

pub fn kind_id(value: &str) -> OperationKindId {
    OperationKindId::new(value).unwrap()
}

pub fn scope_id(value: &str) -> OperationScopeId {
    OperationScopeId::new(value).unwrap()
}

pub fn cursor(value: &str, epoch: u64) -> OperationAuthorityCursor {
    OperationAuthorityCursor::new(
        authority_id(value),
        OperationAuthorityEpoch::new(epoch).unwrap(),
    )
}

pub fn catalogue(authority: &str, epoch: u64) -> OperationCatalogue {
    OperationCatalogue::new(
        authority_id(authority),
        OperationAuthorityEpoch::new(epoch).unwrap(),
        OperationCatalogueLimits::default(),
    )
}

pub fn catalogue_with_limit(authority: &str, epoch: u64, maximum: usize) -> OperationCatalogue {
    OperationCatalogue::new(
        authority_id(authority),
        OperationAuthorityEpoch::new(epoch).unwrap(),
        OperationCatalogueLimits::new(maximum, 100, 16 * 1_024 * 1_024).unwrap(),
    )
}

pub fn registration(
    catalogue: &OperationCatalogue,
    id: &str,
    kind: &str,
    label: &str,
    initial_state: OperationState,
) -> OperationRegistration {
    OperationRegistration::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        operation_id(id),
        kind_id(kind),
        None,
        OperationLabel::new(label).unwrap(),
        initial_state,
        OperationCancellationSupport::Supported,
        None,
    )
}

pub fn scoped_registration(
    catalogue: &OperationCatalogue,
    id: &str,
    kind: &str,
    scope: &str,
    label: &str,
    initial_state: OperationState,
) -> OperationRegistration {
    OperationRegistration::new(
        catalogue.authority().clone(),
        catalogue.revision(),
        operation_id(id),
        kind_id(kind),
        Some(scope_id(scope)),
        OperationLabel::new(label).unwrap(),
        initial_state,
        OperationCancellationSupport::Supported,
        None,
    )
}

pub fn transition(
    catalogue: &OperationCatalogue,
    id: &str,
    next_state: OperationState,
) -> OperationTransition {
    let revision = catalogue.operation(&operation_id(id)).unwrap().revision();
    OperationTransition::new(
        catalogue.authority().clone(),
        operation_id(id),
        revision,
        next_state,
    )
}

pub fn stale_registration(catalogue: &OperationCatalogue, id: &str) -> OperationRegistration {
    OperationRegistration::new(
        catalogue.authority().clone(),
        OperationCatalogueRevision::INITIAL,
        operation_id(id),
        kind_id("test"),
        None,
        OperationLabel::new("Stale").unwrap(),
        OperationState::Running,
        OperationCancellationSupport::Supported,
        None,
    )
}

pub fn transition_at_revision(
    catalogue: &OperationCatalogue,
    id: &str,
    revision: u64,
    next_state: OperationState,
) -> OperationTransition {
    OperationTransition::new(
        catalogue.authority().clone(),
        operation_id(id),
        OperationRevision::new(revision),
        next_state,
    )
}
