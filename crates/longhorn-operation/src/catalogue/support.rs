//! Internal catalogue validation and pruning helpers.

use longhorn_core::{OperationCatalogueRevision, OperationId};

use crate::{
    OperationAuthorityCursor, OperationCatalogueError, OperationCatalogueLimits,
    OperationProgressUpdate, OperationRecord, OperationRemoval, OperationRemovalReason,
    OperationTeardown, OperationTeardownResolutionOutcome,
};

use super::OperationCatalogue;

impl OperationCatalogue {
    pub(crate) fn validate_open_authority(
        &self,
        request: &OperationAuthorityCursor,
    ) -> Result<(), OperationCatalogueError> {
        if self.closed {
            return Err(OperationCatalogueError::AuthorityClosed);
        }
        self.validate_authority(request)
    }

    pub(crate) fn validate_authority(
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

    pub(crate) fn validate_catalogue_revision(
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

    pub(crate) fn operation_index(
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

    pub(crate) fn validate_operation_revision(
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

    pub(crate) fn next_catalogue_revision(
        &self,
    ) -> Result<OperationCatalogueRevision, OperationCatalogueError> {
        self.revision
            .checked_next()
            .map_err(|_| OperationCatalogueError::CatalogueRevisionOverflow)
    }

    pub(crate) fn next_terminal_eviction_count(&self, evicted: usize) -> Result<u64, OperationCatalogueError> {
        let evicted = u64::try_from(evicted)
            .map_err(|_| OperationCatalogueError::TerminalEvictionCountOverflow)?;
        self.terminal_eviction_count
            .checked_add(evicted)
            .ok_or(OperationCatalogueError::TerminalEvictionCountOverflow)
    }
}

pub(crate) fn next_operation_revision(
    operation: &OperationRecord,
) -> Result<longhorn_core::OperationRevision, OperationCatalogueError> {
    operation
        .revision()
        .checked_next()
        .map_err(|_| OperationCatalogueError::OperationRevisionOverflow)
}

pub(crate) fn validate_progress(
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

pub(crate) fn terminal_weight(operations: &[OperationRecord]) -> Result<u64, OperationCatalogueError> {
    operations
        .iter()
        .filter(|operation| operation.state().is_terminal())
        .try_fold(0_u64, |total, operation| {
            total
                .checked_add(operation.encoded_metadata_weight())
                .ok_or(OperationCatalogueError::TerminalEncodedWeightOverflow)
        })
}

pub(crate) fn prune_terminal(
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

pub(crate) fn validate_teardown(
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

