use std::{
    collections::{BTreeSet, VecDeque},
    error::Error,
    fmt,
};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};

use crate::retention::retained_weight;
use crate::{
    AppliedHistoryRecord, HistoryCoalesce, HistoryCoalesceContext, HistoryCommittedTransition,
    HistoryCommittedTransitionKind, HistoryEntry, HistoryEntrySequence, HistoryLimits,
    HistoryNavigationLimits, HistoryPolicy, HistoryProjectionLimits, HistoryPrunedEntry,
    HistoryPruningReceipt, HistoryRecordTransitionEffect, HistoryRetainedBaseline,
    HistoryRetentionError,
};

/// Structural state for one typed linear history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearHistoryState<P> {
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) next_sequence: HistoryEntrySequence,
    pub(crate) retained_baseline: HistoryRetainedBaseline,
    pub(crate) applied: Vec<HistoryEntry<P>>,
    pub(crate) future: Vec<HistoryEntry<P>>,
}

impl<P> LinearHistoryState<P> {
    /// Constructs state that [`LinearHistory::from_state`] will validate.
    #[must_use]
    pub const fn new(
        history_id: HistoryId,
        revision: HistoryRevision,
        next_sequence: HistoryEntrySequence,
        applied: Vec<HistoryEntry<P>>,
        future: Vec<HistoryEntry<P>>,
    ) -> Self {
        Self {
            history_id,
            revision,
            next_sequence,
            retained_baseline: HistoryRetainedBaseline::EMPTY,
            applied,
            future,
        }
    }

    /// Constructs state with explicit retained-baseline evidence.
    #[must_use]
    pub const fn with_retained_baseline(
        history_id: HistoryId,
        revision: HistoryRevision,
        next_sequence: HistoryEntrySequence,
        retained_baseline: HistoryRetainedBaseline,
        applied: Vec<HistoryEntry<P>>,
        future: Vec<HistoryEntry<P>>,
    ) -> Self {
        Self {
            history_id,
            revision,
            next_sequence,
            retained_baseline,
            applied,
            future,
        }
    }

    /// Returns the history authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the structural revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the next insertion sequence.
    #[must_use]
    pub const fn next_sequence(&self) -> HistoryEntrySequence {
        self.next_sequence
    }

    /// Returns durable retained-baseline evidence.
    #[must_use]
    pub const fn retained_baseline(&self) -> &HistoryRetainedBaseline {
        &self.retained_baseline
    }

    /// Returns applied entries from oldest to current.
    #[must_use]
    pub fn applied(&self) -> &[HistoryEntry<P>] {
        &self.applied
    }

    /// Returns future entries from farthest to next-redo.
    #[must_use]
    pub fn future(&self) -> &[HistoryEntry<P>] {
        &self.future
    }
}

/// Validated mutable linear history state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearHistory<P> {
    pub(crate) limits: HistoryLimits,
    pub(crate) navigation_limits: HistoryNavigationLimits,
    pub(crate) projection_limits: HistoryProjectionLimits,
    pub(crate) recent_committed_plan_ids: VecDeque<HistoryPlanId>,
    pub(crate) active_group: Option<crate::HistoryActiveGroup>,
    pub(crate) coalescing_open: bool,
    pub(crate) state: LinearHistoryState<P>,
}

impl<P> LinearHistory<P> {
    /// Constructs an empty history.
    #[must_use]
    pub const fn new(history_id: HistoryId, limits: HistoryLimits) -> Self {
        Self::with_runtime_limits(
            history_id,
            limits,
            HistoryNavigationLimits::DEFAULT,
            HistoryProjectionLimits::DEFAULT,
        )
    }

    /// Constructs an empty history with explicit navigation limits.
    #[must_use]
    pub const fn with_navigation_limits(
        history_id: HistoryId,
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
    ) -> Self {
        Self::with_runtime_limits(
            history_id,
            limits,
            navigation_limits,
            HistoryProjectionLimits::DEFAULT,
        )
    }

    /// Constructs an empty history with all pure runtime limits explicit.
    #[must_use]
    pub const fn with_runtime_limits(
        history_id: HistoryId,
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
        projection_limits: HistoryProjectionLimits,
    ) -> Self {
        Self {
            limits,
            navigation_limits,
            projection_limits,
            recent_committed_plan_ids: VecDeque::new(),
            active_group: None,
            coalescing_open: false,
            state: LinearHistoryState {
                history_id,
                revision: HistoryRevision::INITIAL,
                next_sequence: HistoryEntrySequence::FIRST,
                retained_baseline: HistoryRetainedBaseline::EMPTY,
                applied: Vec::new(),
                future: Vec::new(),
            },
        }
    }

    /// Validates and accepts decoded structural state.
    pub fn from_state(
        limits: HistoryLimits,
        state: LinearHistoryState<P>,
    ) -> Result<Self, HistoryStateError> {
        Self::from_state_with_navigation_limits(limits, HistoryNavigationLimits::DEFAULT, state)
    }

    /// Validates decoded state with explicit transient navigation limits.
    pub fn from_state_with_navigation_limits(
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
        state: LinearHistoryState<P>,
    ) -> Result<Self, HistoryStateError> {
        Self::from_state_with_runtime_limits(
            limits,
            navigation_limits,
            HistoryProjectionLimits::DEFAULT,
            state,
        )
    }

    /// Validates decoded state with all transient runtime limits explicit.
    pub fn from_state_with_runtime_limits(
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
        projection_limits: HistoryProjectionLimits,
        state: LinearHistoryState<P>,
    ) -> Result<Self, HistoryStateError> {
        validate_state(limits, &state)?;
        Ok(Self {
            limits,
            navigation_limits,
            projection_limits,
            recent_committed_plan_ids: VecDeque::new(),
            active_group: None,
            coalescing_open: false,
            state,
        })
    }

    /// Consumes the authority and returns its structural state.
    #[must_use]
    pub fn into_state(self) -> LinearHistoryState<P> {
        self.state
    }

    /// Returns configured limits.
    #[must_use]
    pub const fn limits(&self) -> HistoryLimits {
        self.limits
    }

    /// Returns transient navigation and duplicate-plan limits.
    #[must_use]
    pub const fn navigation_limits(&self) -> HistoryNavigationLimits {
        self.navigation_limits
    }

    /// Returns configured projection limits.
    #[must_use]
    pub const fn projection_limits(&self) -> HistoryProjectionLimits {
        self.projection_limits
    }

    /// Returns the history authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.state.history_id
    }

    /// Returns the current structural revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.state.revision
    }

    /// Returns the next insertion sequence.
    #[must_use]
    pub const fn next_sequence(&self) -> HistoryEntrySequence {
        self.state.next_sequence
    }

    /// Returns durable retained-baseline evidence.
    #[must_use]
    pub const fn retained_baseline(&self) -> &HistoryRetainedBaseline {
        &self.state.retained_baseline
    }

    /// Returns applied entries from oldest to current.
    #[must_use]
    pub fn applied(&self) -> &[HistoryEntry<P>] {
        &self.state.applied
    }

    /// Returns future entries from farthest to next-redo.
    #[must_use]
    pub fn future(&self) -> &[HistoryEntry<P>] {
        &self.state.future
    }

    /// Returns the current entry.
    #[must_use]
    pub fn current(&self) -> Option<&HistoryEntry<P>> {
        self.state.applied.last()
    }

    /// Returns the next entry that undo navigation would target.
    #[must_use]
    pub fn next_undo(&self) -> Option<&HistoryEntry<P>> {
        self.state.applied.last()
    }

    /// Returns the next entry that redo navigation would apply.
    #[must_use]
    pub fn next_redo(&self) -> Option<&HistoryEntry<P>> {
        self.state.future.last()
    }

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
        if let HistoryCoalesce::Replace(merged) = &coalesce {
            if policy.is_noop(merged) {
                return Err(HistoryRecordError::CoalescedPayloadIsNoOp);
            }
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

fn contains_entry_id<P>(state: &LinearHistoryState<P>, entry_id: &HistoryEntryId) -> bool {
    state
        .applied
        .iter()
        .chain(&state.future)
        .any(|entry| entry.entry_id() == entry_id)
}

fn validate_state<P>(
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
    ) {
        if baseline_sequence >= first_retained.sequence() {
            return Err(HistoryStateError::InvalidRetainedBaseline);
        }
    }
    if prior_sequence.is_some_and(|last| state.next_sequence <= last) {
        return Err(HistoryStateError::NextSequenceNotAfterEntries);
    }
    Ok(())
}

/// Explicit structural result of one record attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryRecordOutcome {
    /// The consumer policy classified the incoming payload as a no-op.
    IgnoredNoOp {
        /// Injected identity that was not retained.
        entry_id: HistoryEntryId,
    },
    /// A new entry was appended.
    Added {
        /// Retained entry identity.
        entry_id: HistoryEntryId,
        /// Allocated insertion sequence.
        sequence: HistoryEntrySequence,
    },
    /// The current entry was replaced while retaining identity and sequence.
    Replaced {
        /// Retained prior entry identity.
        entry_id: HistoryEntryId,
        /// Retained prior insertion sequence.
        sequence: HistoryEntrySequence,
    },
    /// Adjacent payloads removed the current entry.
    Removed {
        /// Removed prior entry identity.
        entry_id: HistoryEntryId,
        /// Removed prior insertion sequence.
        sequence: HistoryEntrySequence,
    },
}

/// Structural result and exact future clearing from one record attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRecordResult {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    outcome: HistoryRecordOutcome,
    cleared_future: Vec<HistoryEntryId>,
    pruning: HistoryPruningReceipt,
    retained_encoded_weight: u64,
    transition: Option<HistoryCommittedTransition>,
}

impl HistoryRecordResult {
    /// Returns the admitted source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the resulting revision, unchanged for an ignored no-op.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the structural entry effect.
    #[must_use]
    pub const fn outcome(&self) -> &HistoryRecordOutcome {
        &self.outcome
    }

    /// Returns cleared future ids in ascending insertion order.
    #[must_use]
    pub fn cleared_future(&self) -> &[HistoryEntryId] {
        &self.cleared_future
    }

    /// Returns exact retention pruning committed with this record.
    #[must_use]
    pub const fn pruning(&self) -> &HistoryPruningReceipt {
        &self.pruning
    }

    /// Returns authoritative retained encoded weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }

    /// Returns the committed structural transition, absent for an ignored no-op.
    #[must_use]
    pub const fn transition(&self) -> Option<&HistoryCommittedTransition> {
        self.transition.as_ref()
    }
}

/// Rejected structural record attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryRecordError<E> {
    /// The request did not target the current history revision.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current authoritative revision.
        actual: HistoryRevision,
    },
    /// The history revision could not advance.
    RevisionOverflow,
    /// The insertion sequence could not advance.
    SequenceOverflow,
    /// The injected entry identity is already retained.
    DuplicateEntryId(HistoryEntryId),
    /// The label exceeds this history's configured bound.
    LabelTooLong {
        /// Configured maximum bytes.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// A single retained payload cannot fit the configured weight budget.
    PayloadWeightExceedsLimit {
        /// Configured encoded-weight maximum.
        maximum: u64,
        /// Consumer-measured payload weight.
        actual: u64,
    },
    /// Consumer policy rejected inverse or coalescing semantics.
    Policy(E),
    /// Coalescing returned a replacement payload that policy calls a no-op.
    CoalescedPayloadIsNoOp,
    /// Committed group metadata bypassed the explicit group lifecycle.
    CommittedGroupRequiresLifecycle,
    /// An ordinary record attempted to bypass an active group.
    ActiveGroupOpen(longhorn_core::HistoryGroupId),
    /// A continuing group did not produce one atomic retained payload.
    GroupPolicyKeptSeparate,
    /// No safe applied-prefix pruning could admit the record.
    ImpossibleRetention,
    /// Retained-weight or baseline arithmetic failed closed.
    Retention(HistoryRetentionError),
}

impl<E: fmt::Display> fmt::Display for HistoryRecordError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "expected history revision {}; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::RevisionOverflow => formatter.write_str("history revision cannot advance"),
            Self::SequenceOverflow => formatter.write_str("history entry sequence cannot advance"),
            Self::DuplicateEntryId(entry_id) => {
                write!(formatter, "history entry id {entry_id} is already retained")
            }
            Self::LabelTooLong { maximum, actual } => write!(
                formatter,
                "history label is {actual} bytes; configured maximum is {maximum}"
            ),
            Self::PayloadWeightExceedsLimit { maximum, actual } => write!(
                formatter,
                "history payload weight is {actual}; configured maximum is {maximum}"
            ),
            Self::Policy(error) => write!(formatter, "history policy rejected record: {error}"),
            Self::CoalescedPayloadIsNoOp => formatter
                .write_str("history policy must return removal instead of a no-op replacement"),
            Self::CommittedGroupRequiresLifecycle => formatter
                .write_str("history record group metadata requires the explicit group lifecycle"),
            Self::ActiveGroupOpen(group_id) => write!(
                formatter,
                "history group {group_id} must close before an ordinary record"
            ),
            Self::GroupPolicyKeptSeparate => formatter.write_str(
                "history group policy must merge or remove a continuing grouped payload",
            ),
            Self::ImpossibleRetention => {
                formatter.write_str("history record cannot satisfy retention safely")
            }
            Self::Retention(error) => write!(formatter, "history retention failed: {error}"),
        }
    }
}

impl<E> Error for HistoryRecordError<E> where E: Error + 'static {}

/// Invalid structural history state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryStateError {
    /// Applied plus future entry count overflowed.
    EntryCountOverflow,
    /// Applied plus future entries exceeded the configured count.
    TooManyEntries {
        /// Configured maximum.
        maximum: usize,
        /// Supplied total.
        actual: usize,
    },
    /// Retained encoded payload weight exceeded the configured maximum.
    TooMuchEncodedWeight {
        /// Configured maximum.
        maximum: u64,
        /// Supplied total.
        actual: u64,
    },
    /// The same entry identity appeared more than once.
    DuplicateEntryId(HistoryEntryId),
    /// One label exceeded the configured bound.
    LabelTooLong {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Configured maximum bytes.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// An entry revision was zero or ahead of the authority.
    InvalidEntryRevision {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Entry revision.
        entry_revision: HistoryRevision,
        /// Authority revision.
        history_revision: HistoryRevision,
    },
    /// Canonical applied-plus-future entry sequences were not strictly increasing.
    SequenceOrder,
    /// The next insertion sequence was not after every retained entry.
    NextSequenceNotAfterEntries,
    /// Retained-baseline fields were inconsistent with retained sequences.
    InvalidRetainedBaseline,
    /// Retained-weight arithmetic failed closed.
    Retention(HistoryRetentionError),
}

impl fmt::Display for HistoryStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryCountOverflow => formatter.write_str("history entry count overflowed"),
            Self::TooManyEntries { maximum, actual } => write!(
                formatter,
                "history state has {actual} entries; configured maximum is {maximum}"
            ),
            Self::TooMuchEncodedWeight { maximum, actual } => write!(
                formatter,
                "history state has encoded weight {actual}; configured maximum is {maximum}"
            ),
            Self::DuplicateEntryId(entry_id) => {
                write!(formatter, "history state repeats entry id {entry_id}")
            }
            Self::LabelTooLong {
                entry_id,
                maximum,
                actual,
            } => write!(
                formatter,
                "history entry {entry_id} label is {actual} bytes; configured maximum is {maximum}"
            ),
            Self::InvalidEntryRevision {
                entry_id,
                entry_revision,
                history_revision,
            } => write!(
                formatter,
                "history entry {entry_id} revision {} is invalid for history revision {}",
                entry_revision.get(),
                history_revision.get()
            ),
            Self::SequenceOrder => {
                formatter.write_str("history entry sequences are not in canonical order")
            }
            Self::NextSequenceNotAfterEntries => {
                formatter.write_str("history next sequence is not after every retained entry")
            }
            Self::InvalidRetainedBaseline => {
                formatter.write_str("history retained baseline is structurally invalid")
            }
            Self::Retention(error) => write!(formatter, "history state retention failed: {error}"),
        }
    }
}

impl Error for HistoryStateError {}
