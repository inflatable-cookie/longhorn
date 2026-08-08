//! Catalogue mutation operations.

use crate::{
    OperationCancellationOutcome, OperationCancellationReceipt, OperationCancellationRequest,
    OperationCancellationSupport, OperationCatalogueError, OperationDismissal,
    OperationDismissalReceipt, OperationProgressReceipt, OperationProgressUpdate, OperationRecord,
    OperationRegistration, OperationRegistrationReceipt, OperationRemoval, OperationRemovalReason,
    OperationRetentionChange, OperationRetentionReceipt, OperationState, OperationTeardown,
    OperationTeardownOutcome, OperationTeardownReceipt, OperationTeardownResolutionOutcome,
    OperationTransition, OperationTransitionReceipt,
};

use super::{
    OperationCatalogue, next_operation_revision, prune_terminal, terminal_weight,
    validate_progress, validate_teardown,
};

impl OperationCatalogue {
    /// Registers consumer-admitted queued or running work.
    pub fn register(
        &mut self,
        request: OperationRegistration,
    ) -> Result<OperationRegistrationReceipt, OperationCatalogueError> {
        self.validate_open_authority(&request.authority)?;
        self.validate_catalogue_revision(request.expected_catalogue_revision)?;
        if !request.initial_state.is_initial() {
            return Err(OperationCatalogueError::InvalidInitialState {
                state: request.initial_state,
            });
        }
        if self.operation(&request.operation_id).is_some() {
            return Err(OperationCatalogueError::DuplicateOperation {
                operation_id: request.operation_id,
            });
        }
        if self
            .operations
            .iter()
            .filter(|operation| operation.is_active())
            .count()
            >= self.limits.maximum_active_operations()
        {
            return Err(OperationCatalogueError::ActiveLimitReached {
                maximum: self.limits.maximum_active_operations(),
            });
        }
        if let Some(source_id) = request.retry_of.as_ref() {
            let source_state = self.operation(source_id).map(OperationRecord::state);
            if !source_state.is_some_and(OperationState::is_terminal) {
                return Err(OperationCatalogueError::InvalidRetrySource {
                    operation_id: source_id.clone(),
                    state: source_state,
                });
            }
        }

        let committed_catalogue_revision = self.next_catalogue_revision()?;
        let next_sequence = self
            .next_sequence
            .checked_next()
            .map_err(|_| OperationCatalogueError::SequenceOverflow)?;
        let record = OperationRecord::registered(
            self.authority.clone(),
            request,
            self.next_sequence,
            committed_catalogue_revision,
        );
        let receipt = OperationRegistrationReceipt {
            operation: record.clone(),
            previous_catalogue_revision: self.revision,
            committed_catalogue_revision,
        };
        self.operations.push(record);
        self.next_sequence = next_sequence;
        self.revision = committed_catalogue_revision;
        Ok(receipt)
    }

    /// Applies one revision-bound legal lifecycle transition.
    pub fn transition(
        &mut self,
        request: OperationTransition,
    ) -> Result<OperationTransitionReceipt, OperationCatalogueError> {
        self.validate_open_authority(&request.authority)?;
        let index = self.operation_index(&request.operation_id)?;
        let operation = &self.operations[index];
        self.validate_operation_revision(operation, request.expected_operation_revision)?;
        if !operation.state().can_transition_to(request.next_state) {
            return Err(OperationCatalogueError::InvalidTransition {
                current: operation.state(),
                next: request.next_state,
            });
        }
        let committed_operation_revision = next_operation_revision(operation)?;
        let committed_catalogue_revision = self.next_catalogue_revision()?;
        let mut operations = self.operations.clone();
        operations[index].commit_transition(
            committed_operation_revision,
            committed_catalogue_revision,
            request.next_state,
        );
        let evicted = prune_terminal(&mut operations, self.limits)?;
        let terminal_eviction_count = self.next_terminal_eviction_count(evicted.len())?;
        let receipt = OperationTransitionReceipt {
            operation_id: request.operation_id,
            previous_state: operation.state(),
            committed_state: request.next_state,
            previous_operation_revision: operation.revision(),
            committed_operation_revision,
            previous_catalogue_revision: self.revision,
            committed_catalogue_revision,
            evicted,
        };
        self.operations = operations;
        self.terminal_eviction_count = terminal_eviction_count;
        self.revision = committed_catalogue_revision;
        Ok(receipt)
    }

    /// Applies one monotonic, revision-bound progress update.
    pub fn update_progress(
        &mut self,
        request: OperationProgressUpdate,
    ) -> Result<OperationProgressReceipt, OperationCatalogueError> {
        self.validate_open_authority(&request.authority)?;
        let index = self.operation_index(&request.operation_id)?;
        let operation = &self.operations[index];
        self.validate_operation_revision(operation, request.expected_operation_revision)?;
        if operation.state().is_terminal() {
            return Err(OperationCatalogueError::ProgressNotReportable {
                state: operation.state(),
            });
        }
        validate_progress(operation, &request)?;
        let committed_operation_revision = next_operation_revision(operation)?;
        let committed_catalogue_revision = self.next_catalogue_revision()?;
        let committed_sequence = operation
            .progress()
            .sequence()
            .checked_next()
            .map_err(|_| OperationCatalogueError::ProgressSequenceOverflow)?;
        let mut committed_progress = operation.progress().clone();
        committed_progress.commit(committed_sequence, request.overall, request.phase);
        let receipt = OperationProgressReceipt {
            operation_id: request.operation_id,
            previous_operation_revision: operation.revision(),
            committed_operation_revision,
            previous_sequence: operation.progress().sequence(),
            committed_progress: committed_progress.clone(),
            previous_catalogue_revision: self.revision,
            committed_catalogue_revision,
        };
        self.operations[index].commit_progress(
            committed_operation_revision,
            committed_catalogue_revision,
            committed_progress,
        );
        self.revision = committed_catalogue_revision;
        Ok(receipt)
    }

    /// Admits or classifies one revision-bound cancellation request.
    pub fn request_cancellation(
        &mut self,
        request: OperationCancellationRequest,
    ) -> Result<OperationCancellationReceipt, OperationCatalogueError> {
        self.validate_open_authority(&request.authority)?;
        let index = self.operation_index(&request.operation_id)?;
        let operation = &self.operations[index];
        self.validate_operation_revision(operation, request.expected_operation_revision)?;

        let outcome = if operation.state().is_terminal() {
            OperationCancellationOutcome::Terminal
        } else if operation.state() == OperationState::Cancelling {
            OperationCancellationOutcome::AlreadyRequested
        } else if operation.cancellation_support() == OperationCancellationSupport::Unsupported {
            OperationCancellationOutcome::Unsupported
        } else {
            OperationCancellationOutcome::Accepted
        };
        if outcome != OperationCancellationOutcome::Accepted {
            return Ok(OperationCancellationReceipt {
                operation_id: request.operation_id,
                outcome,
                previous_state: operation.state(),
                committed_state: operation.state(),
                previous_operation_revision: operation.revision(),
                committed_operation_revision: operation.revision(),
                previous_catalogue_revision: self.revision,
                committed_catalogue_revision: self.revision,
                evicted: Vec::new(),
            });
        }

        let committed_state = if operation.state() == OperationState::Queued {
            OperationState::Cancelled
        } else {
            OperationState::Cancelling
        };
        let committed_operation_revision = next_operation_revision(operation)?;
        let committed_catalogue_revision = self.next_catalogue_revision()?;
        let mut operations = self.operations.clone();
        operations[index].commit_transition(
            committed_operation_revision,
            committed_catalogue_revision,
            committed_state,
        );
        let evicted = prune_terminal(&mut operations, self.limits)?;
        let terminal_eviction_count = self.next_terminal_eviction_count(evicted.len())?;
        let receipt = OperationCancellationReceipt {
            operation_id: request.operation_id,
            outcome,
            previous_state: operation.state(),
            committed_state,
            previous_operation_revision: operation.revision(),
            committed_operation_revision,
            previous_catalogue_revision: self.revision,
            committed_catalogue_revision,
            evicted,
        };
        self.operations = operations;
        self.terminal_eviction_count = terminal_eviction_count;
        self.revision = committed_catalogue_revision;
        Ok(receipt)
    }

    /// Atomically changes limits and reports every resulting terminal eviction.
    pub fn change_retention(
        &mut self,
        request: OperationRetentionChange,
    ) -> Result<OperationRetentionReceipt, OperationCatalogueError> {
        self.validate_open_authority(&request.authority)?;
        self.validate_catalogue_revision(request.expected_catalogue_revision)?;
        let active_count = self
            .operations
            .iter()
            .filter(|operation| operation.is_active())
            .count();
        if active_count > request.limits.maximum_active_operations() {
            return Err(OperationCatalogueError::ActiveLimitBelowCurrent {
                current: active_count,
                maximum: request.limits.maximum_active_operations(),
            });
        }
        let committed_catalogue_revision = self.next_catalogue_revision()?;
        let mut operations = self.operations.clone();
        let evicted = prune_terminal(&mut operations, request.limits)?;
        let terminal_eviction_count = self.next_terminal_eviction_count(evicted.len())?;
        let retained_terminal_encoded_weight = terminal_weight(&operations)?;
        let receipt = OperationRetentionReceipt {
            previous_limits: self.limits,
            committed_limits: request.limits,
            previous_catalogue_revision: self.revision,
            committed_catalogue_revision,
            evicted,
            retained_terminal_encoded_weight,
        };
        self.operations = operations;
        self.terminal_eviction_count = terminal_eviction_count;
        self.limits = request.limits;
        self.revision = committed_catalogue_revision;
        Ok(receipt)
    }

    /// Explicitly dismisses one retained terminal projection.
    pub fn dismiss_terminal(
        &mut self,
        request: OperationDismissal,
    ) -> Result<OperationDismissalReceipt, OperationCatalogueError> {
        self.validate_open_authority(&request.authority)?;
        let index = self.operation_index(&request.operation_id)?;
        let operation = &self.operations[index];
        self.validate_operation_revision(operation, request.expected_operation_revision)?;
        if !operation.state().is_terminal() {
            return Err(OperationCatalogueError::DismissalRequiresTerminal {
                state: operation.state(),
            });
        }
        let committed_catalogue_revision = self.next_catalogue_revision()?;
        let removed = OperationRemoval::from_record(operation, OperationRemovalReason::Dismissed);
        self.operations.remove(index);
        let receipt = OperationDismissalReceipt {
            removed,
            previous_catalogue_revision: self.revision,
            committed_catalogue_revision,
        };
        self.revision = committed_catalogue_revision;
        Ok(receipt)
    }

    /// Atomically resolves all active work and permanently closes this authority.
    pub fn teardown(
        &mut self,
        request: OperationTeardown,
    ) -> Result<OperationTeardownReceipt, OperationCatalogueError> {
        self.validate_open_authority(&request.authority)?;
        self.validate_catalogue_revision(request.expected_catalogue_revision)?;
        validate_teardown(self, &request)?;
        let committed_catalogue_revision = self.next_catalogue_revision()?;
        let mut operations = self.operations.clone();
        let mut outcomes = Vec::with_capacity(request.resolutions.len());

        for resolution in request.resolutions {
            let index = operations
                .iter()
                .position(|operation| operation.operation_id() == &resolution.operation_id)
                .expect("teardown was fully validated");
            let previous_revision = operations[index].revision();
            match resolution.outcome {
                OperationTeardownResolutionOutcome::Complete(state) => {
                    let committed_revision = next_operation_revision(&operations[index])?;
                    operations[index].commit_transition(
                        committed_revision,
                        committed_catalogue_revision,
                        state,
                    );
                    outcomes.push(OperationTeardownOutcome::Completed {
                        operation_id: resolution.operation_id,
                        state,
                        previous_operation_revision: previous_revision,
                        committed_operation_revision: committed_revision,
                    });
                }
                OperationTeardownResolutionOutcome::Transfer(target_authority) => {
                    operations.remove(index);
                    outcomes.push(OperationTeardownOutcome::Transferred {
                        operation_id: resolution.operation_id,
                        previous_operation_revision: previous_revision,
                        target_authority,
                    });
                }
            }
        }
        let evicted = prune_terminal(&mut operations, self.limits)?;
        let terminal_eviction_count = self.next_terminal_eviction_count(evicted.len())?;
        let receipt = OperationTeardownReceipt {
            previous_catalogue_revision: self.revision,
            committed_catalogue_revision,
            outcomes,
            evicted,
        };
        self.operations = operations;
        self.terminal_eviction_count = terminal_eviction_count;
        self.revision = committed_catalogue_revision;
        self.closed = true;
        Ok(receipt)
    }
}
