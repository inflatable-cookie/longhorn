use longhorn_operation::{OperationCatalogue, OperationCatalogueError, OperationState};

use super::support::*;

const STATES: [OperationState; 7] = [
    OperationState::Queued,
    OperationState::Running,
    OperationState::Cancelling,
    OperationState::Succeeded,
    OperationState::Failed,
    OperationState::Cancelled,
    OperationState::Interrupted,
];

fn expected_edge(current: OperationState, next: OperationState) -> bool {
    match current {
        OperationState::Queued => matches!(
            next,
            OperationState::Running
                | OperationState::Failed
                | OperationState::Cancelled
                | OperationState::Interrupted
        ),
        OperationState::Running => matches!(
            next,
            OperationState::Cancelling
                | OperationState::Succeeded
                | OperationState::Failed
                | OperationState::Cancelled
                | OperationState::Interrupted
        ),
        OperationState::Cancelling => matches!(
            next,
            OperationState::Succeeded
                | OperationState::Failed
                | OperationState::Cancelled
                | OperationState::Interrupted
        ),
        OperationState::Succeeded
        | OperationState::Failed
        | OperationState::Cancelled
        | OperationState::Interrupted => false,
    }
}

fn catalogue_in_state(state: OperationState) -> OperationCatalogue {
    let initial = if state == OperationState::Queued {
        OperationState::Queued
    } else {
        OperationState::Running
    };
    let mut catalogue = catalogue("authority:matrix", 1);
    catalogue
        .register(registration(
            &catalogue,
            "operation:matrix",
            "matrix",
            "Matrix",
            initial,
        ))
        .unwrap();
    if state != initial {
        catalogue
            .transition(transition(&catalogue, "operation:matrix", state))
            .unwrap();
    }
    catalogue
}

#[test]
fn closed_transition_matrix_is_exact_and_terminal_states_are_sticky() {
    for current in STATES {
        for next in STATES {
            let mut catalogue = catalogue_in_state(current);
            let before = catalogue.clone();
            let result = catalogue.transition(transition(&catalogue, "operation:matrix", next));
            if expected_edge(current, next) {
                let receipt = result.unwrap();
                assert_eq!(receipt.previous_state(), current);
                assert_eq!(receipt.committed_state(), next);
                assert_eq!(
                    catalogue
                        .operation(&operation_id("operation:matrix"))
                        .unwrap()
                        .state(),
                    next
                );
                assert_eq!(
                    receipt.committed_operation_revision().get(),
                    receipt.previous_operation_revision().get() + 1
                );
                assert_eq!(
                    receipt.committed_catalogue_revision().get(),
                    receipt.previous_catalogue_revision().get() + 1
                );
            } else {
                assert_eq!(
                    result,
                    Err(OperationCatalogueError::InvalidTransition { current, next })
                );
                assert_eq!(catalogue, before);
            }
        }
    }
}

#[test]
fn only_queued_and_running_are_valid_registration_states() {
    for state in STATES {
        let mut catalogue = catalogue("authority:initial", 1);
        let before = catalogue.clone();
        let result = catalogue.register(registration(
            &catalogue,
            "operation:initial",
            "initial",
            "Initial",
            state,
        ));
        if matches!(state, OperationState::Queued | OperationState::Running) {
            assert_eq!(result.unwrap().operation().state(), state);
        } else {
            assert_eq!(
                result,
                Err(OperationCatalogueError::InvalidInitialState { state })
            );
            assert_eq!(catalogue, before);
        }
    }
}
