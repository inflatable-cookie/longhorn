use std::cmp::Reverse;

use longhorn_core::{OperationAuthorityId, OperationCatalogueRevision, OperationId};

use crate::{
    OperationAuthorityCursor, OperationAuthorityEpoch, OperationCancellationOutcome,
    OperationCancellationReceipt, OperationCancellationRequest, OperationCancellationSupport,
    OperationCatalogueError, OperationCatalogueLimits, OperationCatalogueProjection,
    OperationDismissal, OperationDismissalReceipt, OperationProgressReceipt,
    OperationProgressUpdate, OperationRecord, OperationRegistration, OperationRegistrationReceipt,
    OperationRemoval, OperationRemovalReason, OperationRetentionChange, OperationRetentionReceipt,
    OperationSequence, OperationState, OperationTeardown, OperationTeardownOutcome,
    OperationTeardownReceipt, OperationTeardownResolutionOutcome, OperationTransition,
    OperationTransitionReceipt,
};

/// Validated mutable authority for one finite operation catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCatalogue {
    authority: OperationAuthorityCursor,
    revision: OperationCatalogueRevision,
    next_sequence: OperationSequence,
    limits: OperationCatalogueLimits,
    operations: Vec<OperationRecord>,
    terminal_eviction_count: u64,
    closed: bool,
}

impl OperationCatalogue {
    /// Constructs an empty catalogue with explicit identity, epoch, and limits.
    #[must_use]
    pub const fn new(
        authority_id: OperationAuthorityId,
        authority_epoch: OperationAuthorityEpoch,
        limits: OperationCatalogueLimits,
    ) -> Self {
        Self {
            authority: OperationAuthorityCursor::new(authority_id, authority_epoch),
            revision: OperationCatalogueRevision::INITIAL,
            next_sequence: OperationSequence::FIRST,
            limits,
            operations: Vec::new(),
            terminal_eviction_count: 0,
            closed: false,
        }
    }

    /// Returns the exact live authority cursor.
    #[must_use]
    pub const fn authority(&self) -> &OperationAuthorityCursor {
        &self.authority
    }

    /// Returns the current catalogue revision.
    #[must_use]
    pub const fn revision(&self) -> OperationCatalogueRevision {
        self.revision
    }

    /// Returns configured finite limits.
    #[must_use]
    pub const fn limits(&self) -> OperationCatalogueLimits {
        self.limits
    }

    /// Returns whether controlled teardown closed the authority.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Returns all retained records in insertion order.
    pub fn operations(&self) -> impl ExactSizeIterator<Item = &OperationRecord> {
        self.operations.iter()
    }

    /// Returns one retained operation.
    #[must_use]
    pub fn operation(&self, operation_id: &OperationId) -> Option<&OperationRecord> {
        self.operations
            .iter()
            .find(|operation| operation.operation_id() == operation_id)
    }

    /// Returns retained terminal metadata weight.
    pub fn retained_terminal_encoded_weight(&self) -> Result<u64, OperationCatalogueError> {
        terminal_weight(&self.operations)
    }

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

    /// Produces active records in insertion order and terminal records newest first.
    #[must_use]
    pub fn project(&self) -> OperationCatalogueProjection {
        let mut recent: Vec<_> = self
            .operations
            .iter()
            .filter(|operation| operation.state().is_terminal())
            .cloned()
            .collect();
        recent.sort_by_key(|operation| {
            Reverse((
                operation.last_changed_catalogue_revision(),
                operation.sequence(),
            ))
        });
        OperationCatalogueProjection {
            authority: self.authority.clone(),
            catalogue_revision: self.revision,
            terminal_eviction_count: self.terminal_eviction_count,
            closed: self.closed,
            active: self
                .operations
                .iter()
                .filter(|operation| operation.is_active())
                .cloned()
                .collect(),
            recent,
        }
    }

    fn validate_open_authority(
        &self,
        request: &OperationAuthorityCursor,
    ) -> Result<(), OperationCatalogueError> {
        if self.closed {
            return Err(OperationCatalogueError::AuthorityClosed);
        }
        self.validate_authority(request)
    }

    fn validate_authority(
        &self,
        request: &OperationAuthorityCursor,
    ) -> Result<(), OperationCatalogueError> {
        if request.authority_id() != self.authority.authority_id() {
            return Err(OperationCatalogueError::AuthorityMismatch {
                expected: self.authority.authority_id().clone(),
                actual: request.authority_id().clone(),
            });
        }
        if request.authority_epoch() != self.authority.authority_epoch() {
            return Err(OperationCatalogueError::AuthorityEpochMismatch {
                expected: self.authority.authority_epoch(),
                actual: request.authority_epoch(),
            });
        }
        Ok(())
    }

    fn validate_catalogue_revision(
        &self,
        actual: OperationCatalogueRevision,
    ) -> Result<(), OperationCatalogueError> {
        if actual != self.revision {
            return Err(OperationCatalogueError::CatalogueRevisionMismatch {
                expected: self.revision,
                actual,
            });
        }
        Ok(())
    }

    fn operation_index(
        &self,
        operation_id: &OperationId,
    ) -> Result<usize, OperationCatalogueError> {
        self.operations
            .iter()
            .position(|operation| operation.operation_id() == operation_id)
            .ok_or_else(|| OperationCatalogueError::UnknownOperation {
                operation_id: operation_id.clone(),
            })
    }

    fn validate_operation_revision(
        &self,
        operation: &OperationRecord,
        actual: longhorn_core::OperationRevision,
    ) -> Result<(), OperationCatalogueError> {
        if actual != operation.revision() {
            return Err(OperationCatalogueError::OperationRevisionMismatch {
                expected: operation.revision(),
                actual,
            });
        }
        Ok(())
    }

    fn next_catalogue_revision(
        &self,
    ) -> Result<OperationCatalogueRevision, OperationCatalogueError> {
        self.revision
            .checked_next()
            .map_err(|_| OperationCatalogueError::CatalogueRevisionOverflow)
    }

    fn next_terminal_eviction_count(&self, evicted: usize) -> Result<u64, OperationCatalogueError> {
        let evicted = u64::try_from(evicted)
            .map_err(|_| OperationCatalogueError::TerminalEvictionCountOverflow)?;
        self.terminal_eviction_count
            .checked_add(evicted)
            .ok_or(OperationCatalogueError::TerminalEvictionCountOverflow)
    }
}

fn next_operation_revision(
    operation: &OperationRecord,
) -> Result<longhorn_core::OperationRevision, OperationCatalogueError> {
    operation
        .revision()
        .checked_next()
        .map_err(|_| OperationCatalogueError::OperationRevisionOverflow)
}

fn validate_progress(
    operation: &OperationRecord,
    request: &OperationProgressUpdate,
) -> Result<(), OperationCatalogueError> {
    match (
        operation.progress().overall().fraction(),
        request.overall.fraction(),
    ) {
        (Some(_), None) => return Err(OperationCatalogueError::OverallProgressRegression),
        (Some(previous), Some(next)) if next < previous => {
            return Err(OperationCatalogueError::OverallProgressRegression);
        }
        _ => {}
    }
    if let (Some(previous), Some(next)) = (operation.progress().phase(), request.phase.as_ref())
        && previous.phase_id() == next.phase_id()
        && next.units().fraction() < previous.units().fraction()
    {
        return Err(OperationCatalogueError::PhaseProgressRegression {
            phase_id: next.phase_id().clone(),
        });
    }
    Ok(())
}

fn terminal_weight(operations: &[OperationRecord]) -> Result<u64, OperationCatalogueError> {
    operations
        .iter()
        .filter(|operation| operation.state().is_terminal())
        .try_fold(0_u64, |total, operation| {
            total
                .checked_add(operation.encoded_metadata_weight())
                .ok_or(OperationCatalogueError::TerminalEncodedWeightOverflow)
        })
}

fn prune_terminal(
    operations: &mut Vec<OperationRecord>,
    limits: OperationCatalogueLimits,
) -> Result<Vec<OperationRemoval>, OperationCatalogueError> {
    let mut count = operations
        .iter()
        .filter(|operation| operation.state().is_terminal())
        .count();
    let mut weight = terminal_weight(operations)?;
    let mut removed = Vec::new();
    while count > limits.maximum_terminal_operations()
        || weight > limits.maximum_terminal_encoded_weight()
    {
        let index = operations
            .iter()
            .enumerate()
            .filter(|(_, operation)| operation.state().is_terminal())
            .min_by_key(|(_, operation)| {
                (
                    operation.last_changed_catalogue_revision(),
                    operation.sequence(),
                )
            })
            .map(|(index, _)| index)
            .expect("retention overflow implies a terminal candidate");
        let removal =
            OperationRemoval::from_record(&operations[index], OperationRemovalReason::Evicted);
        weight -= removal.encoded_weight();
        count -= 1;
        operations.remove(index);
        removed.push(removal);
    }
    Ok(removed)
}

fn validate_teardown(
    catalogue: &OperationCatalogue,
    request: &OperationTeardown,
) -> Result<(), OperationCatalogueError> {
    let active_ids: Vec<_> = catalogue
        .operations
        .iter()
        .filter(|operation| operation.is_active())
        .map(|operation| operation.operation_id().clone())
        .collect();
    let mut seen = Vec::<OperationId>::with_capacity(request.resolutions.len());
    for resolution in &request.resolutions {
        if seen.contains(&resolution.operation_id) {
            return Err(OperationCatalogueError::DuplicateTeardownResolution {
                operation_id: resolution.operation_id.clone(),
            });
        }
        seen.push(resolution.operation_id.clone());
        let operation = catalogue
            .operation(&resolution.operation_id)
            .filter(|operation| operation.is_active())
            .ok_or_else(|| OperationCatalogueError::UnexpectedTeardownResolution {
                operation_id: resolution.operation_id.clone(),
            })?;
        catalogue.validate_operation_revision(operation, resolution.expected_operation_revision)?;
        match &resolution.outcome {
            OperationTeardownResolutionOutcome::Complete(state) if !state.is_terminal() => {
                return Err(OperationCatalogueError::InvalidTeardownTerminal { state: *state });
            }
            OperationTeardownResolutionOutcome::Transfer(target)
                if target == &catalogue.authority =>
            {
                return Err(OperationCatalogueError::TeardownTransferToSelf {
                    operation_id: resolution.operation_id.clone(),
                });
            }
            _ => {}
        }
        if matches!(
            resolution.outcome,
            OperationTeardownResolutionOutcome::Complete(_)
        ) {
            next_operation_revision(operation)?;
        }
    }
    let missing: Vec<_> = active_ids
        .into_iter()
        .filter(|id| !seen.contains(id))
        .collect();
    if !missing.is_empty() {
        return Err(OperationCatalogueError::MissingTeardownResolutions {
            operation_ids: missing,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use longhorn_core::{OperationId, OperationKindId, OperationRevision};

    use super::*;
    use crate::{OperationCancellationSupport, OperationLabel};

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
}
