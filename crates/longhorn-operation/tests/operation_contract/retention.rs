use longhorn_operation::{
    OperationCatalogueError, OperationCatalogueLimits, OperationDismissal, OperationRemovalReason,
    OperationRetentionChange, OperationState,
};

use super::support::*;

fn terminal(catalogue: &mut longhorn_operation::OperationCatalogue, id: &str) {
    catalogue
        .transition(transition(catalogue, id, OperationState::Succeeded))
        .unwrap();
}

#[test]
fn terminal_count_and_weight_evictions_are_oldest_first_and_receipted() {
    let limits = OperationCatalogueLimits::new(2, 1, 1_024).unwrap();
    let mut catalogue = longhorn_operation::OperationCatalogue::new(
        authority_id("authority:retention"),
        longhorn_operation::OperationAuthorityEpoch::new(1).unwrap(),
        limits,
    );
    catalogue
        .register(registration(
            &catalogue,
            "operation:first",
            "test",
            "First",
            OperationState::Running,
        ))
        .unwrap();
    terminal(&mut catalogue, "operation:first");
    catalogue
        .register(registration(
            &catalogue,
            "operation:second",
            "test",
            "Second",
            OperationState::Running,
        ))
        .unwrap();
    let receipt = catalogue
        .transition(transition(
            &catalogue,
            "operation:second",
            OperationState::Succeeded,
        ))
        .unwrap();
    assert_eq!(receipt.evicted().len(), 1);
    assert_eq!(
        receipt.evicted()[0].operation_id().as_str(),
        "operation:first"
    );
    assert_eq!(
        receipt.evicted()[0].reason(),
        OperationRemovalReason::Evicted
    );
    assert!(
        catalogue
            .operation(&operation_id("operation:first"))
            .is_none()
    );

    let zero_weight = OperationCatalogueLimits::new(2, 2, 0).unwrap();
    let changed = catalogue
        .change_retention(OperationRetentionChange::new(
            catalogue.authority().clone(),
            catalogue.revision(),
            zero_weight,
        ))
        .unwrap();
    assert_eq!(changed.evicted().len(), 1);
    assert_eq!(
        changed.evicted()[0].operation_id().as_str(),
        "operation:second"
    );
    assert_eq!(changed.retained_terminal_encoded_weight(), 0);
}

#[test]
fn active_records_never_evict_and_terminal_dismissal_is_explicit() {
    let limits = OperationCatalogueLimits::new(2, 0, 0).unwrap();
    let mut catalogue = longhorn_operation::OperationCatalogue::new(
        authority_id("authority:active-retention"),
        longhorn_operation::OperationAuthorityEpoch::new(1).unwrap(),
        limits,
    );
    catalogue
        .register(registration(
            &catalogue,
            "operation:one",
            "test",
            "One",
            OperationState::Running,
        ))
        .unwrap();
    catalogue
        .register(registration(
            &catalogue,
            "operation:two",
            "test",
            "Two",
            OperationState::Running,
        ))
        .unwrap();

    let before = catalogue.clone();
    let too_small = OperationCatalogueLimits::new(1, 0, 0).unwrap();
    assert_eq!(
        catalogue.change_retention(OperationRetentionChange::new(
            catalogue.authority().clone(),
            catalogue.revision(),
            too_small
        )),
        Err(OperationCatalogueError::ActiveLimitBelowCurrent {
            current: 2,
            maximum: 1
        })
    );
    assert_eq!(catalogue, before);

    let active = catalogue.operation(&operation_id("operation:one")).unwrap();
    let dismissal = OperationDismissal::new(
        catalogue.authority().clone(),
        active.operation_id().clone(),
        active.revision(),
    );
    assert_eq!(
        catalogue.dismiss_terminal(dismissal),
        Err(OperationCatalogueError::DismissalRequiresTerminal {
            state: OperationState::Running
        })
    );
    assert_eq!(catalogue, before);

    let completed = catalogue
        .transition(transition(
            &catalogue,
            "operation:one",
            OperationState::Succeeded,
        ))
        .unwrap();
    assert_eq!(completed.evicted().len(), 1);
    assert_eq!(
        completed.evicted()[0].operation_id().as_str(),
        "operation:one"
    );
    assert_eq!(catalogue.project().active().len(), 1);

    let generous = OperationCatalogueLimits::new(2, 2, 1_024).unwrap();
    catalogue
        .change_retention(OperationRetentionChange::new(
            catalogue.authority().clone(),
            catalogue.revision(),
            generous,
        ))
        .unwrap();
    terminal(&mut catalogue, "operation:two");
    let record = catalogue.operation(&operation_id("operation:two")).unwrap();
    let receipt = catalogue
        .dismiss_terminal(OperationDismissal::new(
            catalogue.authority().clone(),
            record.operation_id().clone(),
            record.revision(),
        ))
        .unwrap();
    assert_eq!(
        receipt.removed().reason(),
        OperationRemovalReason::Dismissed
    );
    assert_eq!(receipt.removed().operation_id().as_str(), "operation:two");
}
