use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};

use crate::{
    AppliedHistoryRecord, HistoryCoalesce, HistoryEntry, HistoryEntrySequence, HistoryLimits,
    HistoryPolicy,
};

/// Structural state for one typed linear history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearHistoryState<P> {
    history_id: HistoryId,
    revision: HistoryRevision,
    next_sequence: HistoryEntrySequence,
    applied: Vec<HistoryEntry<P>>,
    future: Vec<HistoryEntry<P>>,
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
    limits: HistoryLimits,
    state: LinearHistoryState<P>,
}

impl<P> LinearHistory<P> {
    /// Constructs an empty history.
    #[must_use]
    pub const fn new(history_id: HistoryId, limits: HistoryLimits) -> Self {
        Self {
            limits,
            state: LinearHistoryState {
                history_id,
                revision: HistoryRevision::INITIAL,
                next_sequence: HistoryEntrySequence::FIRST,
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
        validate_state(limits, &state)?;
        Ok(Self { limits, state })
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
        if contains_entry_id(&self.state, record.entry_id()) {
            return Err(HistoryRecordError::DuplicateEntryId(
                record.entry_id().clone(),
            ));
        }

        let (expected_revision, entry_id, metadata, payload) = record.into_parts();
        if policy.is_noop(&payload) {
            return Ok(HistoryRecordResult {
                previous_revision: expected_revision,
                committed_revision: expected_revision,
                outcome: HistoryRecordOutcome::IgnoredNoOp { entry_id },
                cleared_future: Vec::new(),
            });
        }

        let committed_revision = self
            .state
            .revision
            .checked_next()
            .map_err(|_| HistoryRecordError::RevisionOverflow)?;
        let coalesce = if self
            .state
            .applied
            .last()
            .is_some_and(|previous| previous.metadata().group_id() == metadata.group_id())
        {
            let previous = self.state.applied.last().expect("checked above");
            policy
                .coalesce(previous.payload(), &payload)
                .map_err(HistoryRecordError::Policy)?
        } else {
            HistoryCoalesce::KeepSeparate
        };

        if let HistoryCoalesce::Replace(merged) = &coalesce {
            if policy.is_noop(merged) {
                return Err(HistoryRecordError::CoalescedPayloadIsNoOp);
            }
        }

        let next_sequence = match coalesce {
            HistoryCoalesce::KeepSeparate => {
                if self.state.applied.len() >= self.limits.maximum_entries() {
                    return Err(HistoryRecordError::EntryLimitReached {
                        maximum: self.limits.maximum_entries(),
                    });
                }
                Some(
                    self.state
                        .next_sequence
                        .checked_next()
                        .map_err(|_| HistoryRecordError::SequenceOverflow)?,
                )
            }
            HistoryCoalesce::Replace(_) | HistoryCoalesce::Remove => None,
        };

        let cleared_future = self
            .state
            .future
            .iter()
            .rev()
            .map(|entry| entry.entry_id().clone())
            .collect();
        self.state.future.clear();

        let outcome = match coalesce {
            HistoryCoalesce::KeepSeparate => {
                let sequence = self.state.next_sequence;
                self.state.applied.push(HistoryEntry::new(
                    entry_id.clone(),
                    metadata,
                    sequence,
                    committed_revision,
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
                previous.replace(metadata, committed_revision, merged);
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
        Ok(HistoryRecordResult {
            previous_revision: expected_revision,
            committed_revision,
            outcome,
            cleared_future,
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
    let total = state.applied.len().saturating_add(state.future.len());
    if total > limits.maximum_entries() {
        return Err(HistoryStateError::TooManyEntries {
            maximum: limits.maximum_entries(),
            actual: total,
        });
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
    /// A separate entry would exceed the configured count before pruning exists.
    EntryLimitReached {
        /// Configured entry maximum.
        maximum: usize,
    },
    /// Consumer policy rejected inverse or coalescing semantics.
    Policy(E),
    /// Coalescing returned a replacement payload that policy calls a no-op.
    CoalescedPayloadIsNoOp,
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
            Self::EntryLimitReached { maximum } => write!(
                formatter,
                "history already retains the configured maximum of {maximum} entries"
            ),
            Self::Policy(error) => write!(formatter, "history policy rejected record: {error}"),
            Self::CoalescedPayloadIsNoOp => formatter
                .write_str("history policy must return removal instead of a no-op replacement"),
        }
    }
}

impl<E> Error for HistoryRecordError<E> where E: Error + 'static {}

/// Invalid structural history state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryStateError {
    /// Applied plus future entries exceeded the configured count.
    TooManyEntries {
        /// Configured maximum.
        maximum: usize,
        /// Supplied total.
        actual: usize,
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
}

impl fmt::Display for HistoryStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyEntries { maximum, actual } => write!(
                formatter,
                "history state has {actual} entries; configured maximum is {maximum}"
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
        }
    }
}

impl Error for HistoryStateError {}
