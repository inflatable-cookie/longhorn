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

    pub(crate) fn next_terminal_eviction_count(
        &self,
        evicted: usize,
    ) -> Result<u64, OperationCatalogueError> {
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

pub(crate) fn terminal_weight(
    operations: &[OperationRecord],
) -> Result<u64, OperationCatalogueError> {
    operations
        .iter()
        .filter(|operation| operation.state().is_terminal())
        .try_fold(0_u64, |total, operation| {
            total
                .checked_add(operation.encoded_metadata_weight())
                .ok_or(OperationCatalogueError::TerminalEncodedWeightOverflow)
        })
}

#[derive(Clone, Copy)]
pub(crate) enum PlannedSlot {
    Unchanged,
    Completed(OperationCatalogueRevision),
    Removed,
}

#[derive(Clone, Copy)]
pub(crate) enum PlannedTerminalChange<'a> {
    None,
    Single(usize, OperationCatalogueRevision),
    Batch(&'a [PlannedSlot]),
}

impl PlannedTerminalChange<'_> {
    #[inline]
    fn slot(&self, index: usize) -> PlannedSlot {
        match self {
            Self::None => PlannedSlot::Unchanged,
            Self::Single(target, revision) => {
                if *target == index {
                    PlannedSlot::Completed(*revision)
                } else {
                    PlannedSlot::Unchanged
                }
            }
            Self::Batch(slots) => slots[index],
        }
    }
}

pub(crate) fn plan_prune_terminal(
    operations: &[OperationRecord],
    limits: OperationCatalogueLimits,
    change: PlannedTerminalChange<'_>,
) -> Result<(Vec<usize>, u64), OperationCatalogueError> {
    let mut count = 0usize;
    let mut weight = 0_u64;
    for (index, operation) in operations.iter().enumerate() {
        let is_terminal = match change.slot(index) {
            PlannedSlot::Removed => false,
            PlannedSlot::Completed(_) => true,
            PlannedSlot::Unchanged => operation.state().is_terminal(),
        };
        if is_terminal {
            count += 1;
            weight = weight
                .checked_add(operation.encoded_metadata_weight())
                .ok_or(OperationCatalogueError::TerminalEncodedWeightOverflow)?;
        }
    }
    if count <= limits.maximum_terminal_operations()
        && weight <= limits.maximum_terminal_encoded_weight()
    {
        return Ok((Vec::new(), weight));
    }

    let mut candidates: Vec<(OperationCatalogueRevision, crate::OperationSequence, usize)> =
        Vec::with_capacity(count);
    for (index, operation) in operations.iter().enumerate() {
        let effective_revision = match change.slot(index) {
            PlannedSlot::Removed => None,
            PlannedSlot::Completed(revision) => Some(revision),
            PlannedSlot::Unchanged => operation
                .state()
                .is_terminal()
                .then_some(operation.last_changed_catalogue_revision()),
        };
        if let Some(revision) = effective_revision {
            candidates.push((revision, operation.sequence(), index));
        }
    }
    candidates.sort_unstable_by_key(|(revision, sequence, _)| (*revision, *sequence));

    let mut evict_indices = Vec::new();
    for (_, _, index) in candidates {
        if count <= limits.maximum_terminal_operations()
            && weight <= limits.maximum_terminal_encoded_weight()
        {
            break;
        }
        weight -= operations[index].encoded_metadata_weight();
        count -= 1;
        evict_indices.push(index);
    }
    Ok((evict_indices, weight))
}

pub(crate) fn apply_planned_evictions(
    operations: &mut Vec<OperationRecord>,
    evict_indices: &[usize],
) -> Vec<OperationRemoval> {
    if evict_indices.is_empty() {
        return Vec::new();
    }
    let removals: Vec<_> = evict_indices
        .iter()
        .map(|&index| {
            OperationRemoval::from_record(&operations[index], OperationRemovalReason::Evicted)
        })
        .collect();
    let mut evicted_mask = vec![false; operations.len()];
    for &index in evict_indices {
        evicted_mask[index] = true;
    }
    let mut cursor = 0usize;
    operations.retain(|_| {
        let keep = !evicted_mask[cursor];
        cursor += 1;
        keep
    });
    removals
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
            OperationTeardownResolutionOutcome::Complete(state)
                if !operation.state().can_transition_to(*state) =>
            {
                return Err(OperationCatalogueError::InvalidTransition {
                    current: operation.state(),
                    next: *state,
                });
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
