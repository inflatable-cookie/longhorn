//! Applied-record mutation on linear history.

use std::collections::BTreeSet;

use longhorn_core::{HistoryEntryId, HistoryRevision};

use crate::retention::retained_weight;
use crate::{
    AppliedHistoryRecord, HistoryCoalesce, HistoryCoalesceContext, HistoryCommittedTransition,
    HistoryCommittedTransitionKind, HistoryEntry, HistoryLimits, HistoryPolicy, HistoryPrunedEntry,
    HistoryPruningReceipt, HistoryRecordTransitionEffect, HistoryRetentionError,
};

use super::{
    HistoryRecordError, HistoryRecordOutcome, HistoryRecordResult, HistoryStateError,
    LinearHistory, LinearHistoryState,
};

impl<P> LinearHistory<P> {
    /// Records a product mutation that the consumer already applied successfully.
    ///
    /// A non-no-op record after undo clears the complete future path. Adjacent
    /// entries are offered to the consumer policy only when their explicit
    /// group identities match.
    pub fn record_applied<T>(
        &mut self,
        record: AppliedHistoryRecord<P>,
        policy: &T,
    ) -> Result<HistoryRecordResult, HistoryRecordError<T::Error>>
    where
        T: HistoryPolicy<P>,
    {
        if let Some(active) = &self.active_group {
            return Err(HistoryRecordError::ActiveGroupOpen(
                active.group_id().clone(),
            ));
        }
        self.record_applied_with_group(record, None, self.coalescing_open, policy)
    }

    pub(crate) fn record_applied_with_group<T>(
        &mut self,
        record: AppliedHistoryRecord<P>,
        group_id: Option<longhorn_core::HistoryGroupId>,
        may_coalesce: bool,
        policy: &T,
    ) -> Result<HistoryRecordResult, HistoryRecordError<T::Error>>
    where
        T: HistoryPolicy<P>,
    {
        if record.expected_revision() != self.state.revision {
            return Err(HistoryRecordError::StaleRevision {
                expected: record.expected_revision(),
                actual: self.state.revision,
            });
        }
        if record.metadata().label().len_bytes() > self.limits.maximum_label_bytes() {
            return Err(HistoryRecordError::LabelTooLong {
                maximum: self.limits.maximum_label_bytes(),
                actual: record.metadata().label().len_bytes(),
            });
        }
        if record.metadata().group_id().is_some() {
            return Err(HistoryRecordError::CommittedGroupRequiresLifecycle);
        }
        if contains_entry_id(&self.state, record.entry_id()) {
            return Err(HistoryRecordError::DuplicateEntryId(
                record.entry_id().clone(),
            ));
        }

        let (expected_revision, entry_id, mut metadata, payload) = record.into_parts();
        metadata.set_group_id(group_id.clone());
        if policy.is_noop(&payload) {
            return Ok(HistoryRecordResult {
                previous_revision: expected_revision,
                committed_revision: expected_revision,
                outcome: HistoryRecordOutcome::IgnoredNoOp { entry_id },
                cleared_future: Vec::new(),
                pruning: HistoryPruningReceipt::default(),
                retained_encoded_weight: retained_weight(&self.state.applied, &self.state.future)
                    .map_err(HistoryRecordError::Retention)?,
                transition: None,
            });
        }

        let committed_revision = self
            .state
            .revision
            .checked_next()
            .map_err(|_| HistoryRecordError::RevisionOverflow)?;
        let coalesce = if may_coalesce
            && self
                .state
                .applied
                .last()
                .is_some_and(|previous| previous.metadata().group_id() == group_id.as_ref())
        {
            let previous = self.state.applied.last().expect("checked above");
            policy
                .coalesce(
                    previous.payload(),
                    &payload,
                    match group_id.as_ref() {
                        Some(group_id) => HistoryCoalesceContext::Group { group_id },
                        None => HistoryCoalesceContext::Adjacent,
                    },
                )
                .map_err(HistoryRecordError::Policy)?
        } else {
            HistoryCoalesce::KeepSeparate
        };

        if group_id.is_some() && may_coalesce && matches!(coalesce, HistoryCoalesce::KeepSeparate) {
            return Err(HistoryRecordError::GroupPolicyKeptSeparate);
        }
        if let HistoryCoalesce::Replace(merged) = &coalesce
            && policy.is_noop(merged)
        {
            return Err(HistoryRecordError::CoalescedPayloadIsNoOp);
        }

        let encoded_weight = match &coalesce {
            HistoryCoalesce::KeepSeparate => policy
                .encoded_weight(&payload)
                .map_err(HistoryRecordError::Policy)?,
            HistoryCoalesce::Replace(merged) => policy
                .encoded_weight(merged)
                .map_err(HistoryRecordError::Policy)?,
            HistoryCoalesce::Remove => 0,
        };
        if encoded_weight > self.limits.maximum_encoded_weight() {
            return Err(HistoryRecordError::PayloadWeightExceedsLimit {
                maximum: self.limits.maximum_encoded_weight(),
                actual: encoded_weight,
            });
        }

        let next_sequence = match coalesce {
            HistoryCoalesce::KeepSeparate => Some(
                self.state
                    .next_sequence
                    .checked_next()
                    .map_err(|_| HistoryRecordError::SequenceOverflow)?,
            ),
            HistoryCoalesce::Replace(_) | HistoryCoalesce::Remove => None,
        };

        let retained_before = retained_weight(&self.state.applied, &self.state.future)
            .map_err(HistoryRecordError::Retention)?;
        let future_weight = self.state.future.iter().try_fold(0_u64, |total, entry| {
            total
                .checked_add(entry.encoded_weight())
                .ok_or(HistoryRecordError::Retention(
                    HistoryRetentionError::EncodedWeightOverflow,
                ))
        })?;
        let mut retained_count = self.state.applied.len();
        let mut retained_weight = retained_before - future_weight;
        let maximum_prunable = match &coalesce {
            HistoryCoalesce::KeepSeparate => {
                retained_count += 1;
                retained_weight = retained_weight.checked_add(encoded_weight).ok_or(
                    HistoryRecordError::Retention(HistoryRetentionError::EncodedWeightOverflow),
                )?;
                self.state.applied.len()
            }
            HistoryCoalesce::Replace(_) => {
                let previous = self
                    .state
                    .applied
                    .last()
                    .expect("coalescing requires a previous entry");
                retained_weight -= previous.encoded_weight();
                retained_weight = retained_weight.checked_add(encoded_weight).ok_or(
                    HistoryRecordError::Retention(HistoryRetentionError::EncodedWeightOverflow),
                )?;
                self.state.applied.len().saturating_sub(1)
            }
            HistoryCoalesce::Remove => {
                let previous = self
                    .state
                    .applied
                    .last()
                    .expect("coalescing requires a previous entry");
                retained_count -= 1;
                retained_weight -= previous.encoded_weight();
                self.state.applied.len().saturating_sub(1)
            }
        };
        let mut applied_to_prune = 0;
        while retained_count > self.limits.maximum_entries()
            || retained_weight > self.limits.maximum_encoded_weight()
        {
            let entry = self
                .state
                .applied
                .get(applied_to_prune)
                .filter(|_| applied_to_prune < maximum_prunable)
                .ok_or(HistoryRecordError::ImpossibleRetention)?;
            retained_count -= 1;
            retained_weight -= entry.encoded_weight();
            applied_to_prune += 1;
        }
        let prospective_baseline = self
            .state
            .retained_baseline
            .checked_advance(&self.state.applied[..applied_to_prune])
            .map_err(HistoryRecordError::Retention)?;
        let advanced_baseline = self.state.applied[..applied_to_prune]
            .iter()
            .map(HistoryPrunedEntry::from_entry)
            .collect();
        let cleared_future: Vec<HistoryEntryId> = self
            .state
            .future
            .iter()
            .rev()
            .map(|entry| entry.entry_id().clone())
            .collect();
        self.state.future.clear();
        self.state.applied.drain(..applied_to_prune);
        self.state.retained_baseline = prospective_baseline;

        let outcome = match coalesce {
            HistoryCoalesce::KeepSeparate => {
                let sequence = self.state.next_sequence;
                self.state.applied.push(HistoryEntry::new(
                    entry_id.clone(),
                    metadata,
                    sequence,
                    committed_revision,
                    encoded_weight,
                    payload,
                ));
                self.state.next_sequence = next_sequence.expect("separate record sequence");
                HistoryRecordOutcome::Added { entry_id, sequence }
            }
            HistoryCoalesce::Replace(merged) => {
                let previous = self
                    .state
                    .applied
                    .last_mut()
                    .expect("coalescing requires a previous entry");
                let retained_entry_id = previous.entry_id().clone();
                let retained_sequence = previous.sequence();
                previous.replace(metadata, committed_revision, encoded_weight, merged);
                HistoryRecordOutcome::Replaced {
                    entry_id: retained_entry_id,
                    sequence: retained_sequence,
                }
            }
            HistoryCoalesce::Remove => {
                let removed = self
                    .state
                    .applied
                    .pop()
                    .expect("coalescing requires a previous entry");
                HistoryRecordOutcome::Removed {
                    entry_id: removed.entry_id().clone(),
                    sequence: removed.sequence(),
                }
            }
        };

        self.state.revision = committed_revision;
        self.coalescing_open = matches!(
            outcome,
            HistoryRecordOutcome::Added { .. } | HistoryRecordOutcome::Replaced { .. }
        );
        let pruning = HistoryPruningReceipt::new(advanced_baseline, Vec::new());
        let effect = HistoryRecordTransitionEffect::from_outcome(&outcome)
            .expect("committed record cannot be an ignored no-op");
        let transition = HistoryCommittedTransition::new(
            self.state.history_id.clone(),
            Some(expected_revision),
            committed_revision,
            HistoryCommittedTransitionKind::Record {
                effect,
                cleared_future: cleared_future.clone(),
                pruning: pruning.clone(),
            },
        );
        Ok(HistoryRecordResult {
            previous_revision: expected_revision,
            committed_revision,
            outcome,
            cleared_future,
            pruning,
            retained_encoded_weight: retained_weight,
            transition: Some(transition),
        })
    }
}

pub(crate) fn contains_entry_id<P>(
    state: &LinearHistoryState<P>,
    entry_id: &HistoryEntryId,
) -> bool {
    state
        .applied
        .iter()
        .chain(&state.future)
        .any(|entry| entry.entry_id() == entry_id)
}

pub(crate) fn validate_state<P>(
    limits: HistoryLimits,
    state: &LinearHistoryState<P>,
) -> Result<(), HistoryStateError> {
    let total = state
        .applied
        .len()
        .checked_add(state.future.len())
        .ok_or(HistoryStateError::EntryCountOverflow)?;
    if total > limits.maximum_entries() {
        return Err(HistoryStateError::TooManyEntries {
            maximum: limits.maximum_entries(),
            actual: total,
        });
    }
    let retained_encoded_weight =
        retained_weight(&state.applied, &state.future).map_err(HistoryStateError::Retention)?;
    if retained_encoded_weight > limits.maximum_encoded_weight() {
        return Err(HistoryStateError::TooMuchEncodedWeight {
            maximum: limits.maximum_encoded_weight(),
            actual: retained_encoded_weight,
        });
    }
    let baseline_valid = if state.retained_baseline.pruned_entry_count() == 0 {
        state.retained_baseline.last_pruned_entry_id().is_none()
            && state.retained_baseline.last_pruned_sequence().is_none()
            && state.retained_baseline.pruned_encoded_weight() == 0
    } else {
        state.retained_baseline.last_pruned_entry_id().is_some()
            && state.retained_baseline.last_pruned_sequence().is_some()
    };
    if !baseline_valid {
        return Err(HistoryStateError::InvalidRetainedBaseline);
    }

    let mut ids = BTreeSet::new();
    let mut prior_sequence = None;
    for entry in state.applied.iter().chain(state.future.iter().rev()) {
        if !ids.insert(entry.entry_id()) {
            return Err(HistoryStateError::DuplicateEntryId(
                entry.entry_id().clone(),
            ));
        }
        if entry.metadata().label().len_bytes() > limits.maximum_label_bytes() {
            return Err(HistoryStateError::LabelTooLong {
                entry_id: entry.entry_id().clone(),
                maximum: limits.maximum_label_bytes(),
                actual: entry.metadata().label().len_bytes(),
            });
        }
        if entry.committed_revision() == HistoryRevision::INITIAL
            || entry.committed_revision() > state.revision
        {
            return Err(HistoryStateError::InvalidEntryRevision {
                entry_id: entry.entry_id().clone(),
                entry_revision: entry.committed_revision(),
                history_revision: state.revision,
            });
        }
        if prior_sequence.is_some_and(|prior| entry.sequence() <= prior) {
            return Err(HistoryStateError::SequenceOrder);
        }
        prior_sequence = Some(entry.sequence());
    }

    if let (Some(baseline_sequence), Some(first_retained)) = (
        state.retained_baseline.last_pruned_sequence(),
        state.applied.first().or_else(|| state.future.last()),
    ) && baseline_sequence >= first_retained.sequence()
    {
        return Err(HistoryStateError::InvalidRetainedBaseline);
    }
    if prior_sequence.is_some_and(|last| state.next_sequence <= last) {
        return Err(HistoryStateError::NextSequenceNotAfterEntries);
    }
    Ok(())
}
