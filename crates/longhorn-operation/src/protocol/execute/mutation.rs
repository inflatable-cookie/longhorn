//! Protocol execution over OperationCatalogue.

use longhorn_core::OperationRequestId;

use crate::{
    OperationCancellationOutcome, OperationCancellationRequest, OperationCatalogue,
    OperationCatalogueError, OperationDismissal, OperationProgressUpdate, OperationRegistration,
    OperationRetentionChange, OperationTeardown, OperationTeardownOutcome,
    OperationTeardownResolution, OperationTeardownResolutionOutcome, OperationTransition,
};

use super::super::*;

use super::{project_teardown_outcome, project_teardown_resolution};


pub(crate) fn execute_mutation(
    catalogue: &mut OperationCatalogue,
    command: OperationMutationCommand,
) -> Result<OperationMutationReceiptProjection, OperationRejection> {
    match command {
        OperationMutationCommand::Register {
            authority,
            expected_catalogue_revision,
            operation_id,
            kind_id,
            scope_id,
            label,
            initial_state,
            cancellation_support,
            retry_of,
            ..
        } => {
            let authority = authority.into_cursor().map_err(OperationRejection::from)?;
            let label = crate::OperationLabel::new(label).map_err(|error| {
                OperationRejection::from(OperationProtocolInputError::Label(error.to_string()))
            })?;
            let receipt = catalogue
                .register(OperationRegistration::new(
                    authority,
                    expected_catalogue_revision,
                    operation_id,
                    kind_id,
                    scope_id,
                    label,
                    initial_state.into(),
                    cancellation_support.into(),
                    retry_of,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Registered {
                operation: OperationEntryProjection::from_record(receipt.operation()),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
            })
        }
        OperationMutationCommand::Progress {
            authority,
            operation_id,
            expected_operation_revision,
            overall,
            phase,
            ..
        } => {
            let request = OperationProgressUpdate::new(
                authority.into_cursor().map_err(OperationRejection::from)?,
                operation_id,
                expected_operation_revision,
                overall.into_progress().map_err(OperationRejection::from)?,
                phase
                    .map(OperationPhaseProgressProjection::into_progress)
                    .transpose()
                    .map_err(OperationRejection::from)?,
            );
            let receipt = catalogue
                .update_progress(request)
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Progressed {
                operation_id: receipt.operation_id().clone(),
                previous_operation_revision: receipt.previous_operation_revision(),
                committed_operation_revision: receipt.committed_operation_revision(),
                previous_progress_sequence: receipt.previous_sequence().get(),
                committed_progress: OperationProgressProjection::from_progress(
                    receipt.committed_progress(),
                ),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
            })
        }
        OperationMutationCommand::Transition {
            authority,
            operation_id,
            expected_operation_revision,
            next_state,
            ..
        } => {
            let receipt = catalogue
                .transition(OperationTransition::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    operation_id,
                    expected_operation_revision,
                    next_state.into(),
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Transitioned {
                operation_id: receipt.operation_id().clone(),
                previous_state: receipt.previous_state().into(),
                committed_state: receipt.committed_state().into(),
                previous_operation_revision: receipt.previous_operation_revision(),
                committed_operation_revision: receipt.committed_operation_revision(),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
                evicted: receipt.evicted().iter().map(Into::into).collect(),
            })
        }
        OperationMutationCommand::ChangeRetention {
            authority,
            expected_catalogue_revision,
            limits,
            ..
        } => {
            let receipt = catalogue
                .change_retention(OperationRetentionChange::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    expected_catalogue_revision,
                    limits.into_limits().map_err(OperationRejection::from)?,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::RetentionChanged {
                previous_limits: OperationCatalogueLimitsProjection::from_limits(
                    receipt.previous_limits(),
                )
                .expect("validated limits project to u64"),
                committed_limits: OperationCatalogueLimitsProjection::from_limits(
                    receipt.committed_limits(),
                )
                .expect("validated limits project to u64"),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
                evicted: receipt.evicted().iter().map(Into::into).collect(),
                retained_terminal_encoded_weight: receipt.retained_terminal_encoded_weight(),
            })
        }
        OperationMutationCommand::Dismiss {
            authority,
            operation_id,
            expected_operation_revision,
            ..
        } => {
            let receipt = catalogue
                .dismiss_terminal(OperationDismissal::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    operation_id,
                    expected_operation_revision,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::Dismissed {
                removed: receipt.removed().into(),
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
            })
        }
        OperationMutationCommand::Teardown {
            authority,
            expected_catalogue_revision,
            resolutions,
            ..
        } => {
            let resolutions = resolutions
                .into_iter()
                .map(project_teardown_resolution)
                .collect::<Result<Vec<_>, _>>()?;
            let receipt = catalogue
                .teardown(OperationTeardown::new(
                    authority.into_cursor().map_err(OperationRejection::from)?,
                    expected_catalogue_revision,
                    resolutions,
                ))
                .map_err(OperationRejection::from)?;
            Ok(OperationMutationReceiptProjection::TornDown {
                previous_catalogue_revision: receipt.previous_catalogue_revision(),
                committed_catalogue_revision: receipt.committed_catalogue_revision(),
                outcomes: receipt
                    .outcomes()
                    .iter()
                    .map(project_teardown_outcome)
                    .collect(),
                evicted: receipt.evicted().iter().map(Into::into).collect(),
            })
        }
    }
}
