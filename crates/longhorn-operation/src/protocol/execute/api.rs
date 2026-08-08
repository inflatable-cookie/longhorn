//! Protocol execution over OperationCatalogue.

use longhorn_core::OperationRequestId;

use crate::{OperationCancellationOutcome, OperationCancellationRequest, OperationCatalogue};

use super::super::*;

use super::execute_mutation;

impl OperationCatalogue {
    /// Executes one strict management command and returns fresh authority.
    pub fn execute_protocol_mutation(
        &mut self,
        command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationProtocolProjectionError> {
        let request_id = command.request_id().clone();
        if command.protocol_version() != OperationProtocolVersion::CURRENT {
            return self.rejected_mutation(request_id, incompatible_protocol());
        }
        let result = execute_mutation(self, command);
        match result {
            Ok(receipt) => Ok(OperationMutationResult::Committed {
                request_id,
                snapshot: OperationSnapshot::from_catalogue(self)?,
                receipt: Box::new(receipt),
            }),
            Err(rejection) => self.rejected_mutation(request_id, rejection),
        }
    }

    /// Executes one strict cancellation command and returns fresh authority.
    pub fn execute_protocol_cancellation(
        &mut self,
        command: OperationCancellationCommand,
    ) -> Result<OperationCancellationResult, OperationProtocolProjectionError> {
        let request_id = command.request_id.clone();
        if command.protocol_version != OperationProtocolVersion::CURRENT {
            return self.rejected_cancellation(request_id, incompatible_protocol());
        }
        let request = match command.authority.into_cursor() {
            Ok(authority) => OperationCancellationRequest::new(
                authority,
                command.operation_id,
                command.expected_operation_revision,
            ),
            Err(error) => return self.rejected_cancellation(request_id, error.into()),
        };
        match self.request_cancellation(request) {
            Ok(receipt) => Ok(OperationCancellationResult::Committed {
                request_id,
                snapshot: OperationSnapshot::from_catalogue(self)?,
                receipt: OperationCancellationReceiptProjection {
                    operation_id: receipt.operation_id().clone(),
                    outcome: match receipt.outcome() {
                        OperationCancellationOutcome::Accepted => {
                            OperationCancellationOutcomeProjection::Accepted
                        }
                        OperationCancellationOutcome::AlreadyRequested => {
                            OperationCancellationOutcomeProjection::AlreadyRequested
                        }
                        OperationCancellationOutcome::Unsupported => {
                            OperationCancellationOutcomeProjection::Unsupported
                        }
                        OperationCancellationOutcome::Terminal => {
                            OperationCancellationOutcomeProjection::Terminal
                        }
                    },
                    previous_state: receipt.previous_state().into(),
                    committed_state: receipt.committed_state().into(),
                    previous_operation_revision: receipt.previous_operation_revision(),
                    committed_operation_revision: receipt.committed_operation_revision(),
                    previous_catalogue_revision: receipt.previous_catalogue_revision(),
                    committed_catalogue_revision: receipt.committed_catalogue_revision(),
                    evicted: receipt.evicted().iter().map(Into::into).collect(),
                },
                executor_dispatch: OperationExecutorDispatchProjection::NotRequired,
            }),
            Err(error) => self.rejected_cancellation(request_id, error.into()),
        }
    }

    pub(crate) fn rejected_mutation(
        &self,
        request_id: OperationRequestId,
        rejection: OperationRejection,
    ) -> Result<OperationMutationResult, OperationProtocolProjectionError> {
        Ok(OperationMutationResult::Rejected {
            request_id,
            snapshot: OperationSnapshot::from_catalogue(self)?,
            rejection,
        })
    }

    pub(crate) fn rejected_cancellation(
        &self,
        request_id: OperationRequestId,
        rejection: OperationRejection,
    ) -> Result<OperationCancellationResult, OperationProtocolProjectionError> {
        Ok(OperationCancellationResult::Rejected {
            request_id,
            snapshot: OperationSnapshot::from_catalogue(self)?,
            rejection,
        })
    }
}
