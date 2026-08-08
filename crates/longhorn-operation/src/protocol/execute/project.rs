//! Protocol execution over OperationCatalogue.

use longhorn_core::OperationRequestId;

use crate::{
    OperationCancellationOutcome, OperationCancellationRequest, OperationCatalogue,
    OperationCatalogueError, OperationDismissal, OperationProgressUpdate, OperationRegistration,
    OperationRetentionChange, OperationTeardown, OperationTeardownOutcome,
    OperationTeardownResolution, OperationTeardownResolutionOutcome, OperationTransition,
};

use super::super::*;


pub(crate) fn project_teardown_resolution(
    value: OperationTeardownResolutionProjection,
) -> Result<OperationTeardownResolution, OperationRejection> {
    match value {
        OperationTeardownResolutionProjection::Complete {
            operation_id,
            expected_operation_revision,
            state,
        } => Ok(OperationTeardownResolution::new(
            operation_id,
            expected_operation_revision,
            OperationTeardownResolutionOutcome::Complete(state.into()),
        )),
        OperationTeardownResolutionProjection::Transfer {
            operation_id,
            expected_operation_revision,
            target_authority,
        } => Ok(OperationTeardownResolution::new(
            operation_id,
            expected_operation_revision,
            OperationTeardownResolutionOutcome::Transfer(
                target_authority
                    .into_cursor()
                    .map_err(OperationRejection::from)?,
            ),
        )),
    }
}

pub(crate) fn project_teardown_outcome(
    value: &OperationTeardownOutcome,
) -> OperationTeardownOutcomeProjection {
    match value {
        OperationTeardownOutcome::Completed {
            operation_id,
            state,
            previous_operation_revision,
            committed_operation_revision,
        } => OperationTeardownOutcomeProjection::Completed {
            operation_id: operation_id.clone(),
            state: (*state).into(),
            previous_operation_revision: *previous_operation_revision,
            committed_operation_revision: *committed_operation_revision,
        },
        OperationTeardownOutcome::Transferred {
            operation_id,
            previous_operation_revision,
            target_authority,
        } => OperationTeardownOutcomeProjection::Transferred {
            operation_id: operation_id.clone(),
            previous_operation_revision: *previous_operation_revision,
            target_authority: OperationAuthorityProjection::from_cursor(target_authority),
        },
    }
}
