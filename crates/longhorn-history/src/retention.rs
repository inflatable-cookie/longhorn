use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryRevision};

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryEntry, HistoryEntrySequence,
    HistoryLimits, LinearHistory,
};

/// Durable evidence for the product state before the oldest retained entry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HistoryRetainedBaseline {
    pruned_entry_count: u64,
    pruned_encoded_weight: u64,
    last_pruned_entry_id: Option<HistoryEntryId>,
    last_pruned_sequence: Option<HistoryEntrySequence>,
}

impl HistoryRetainedBaseline {
    /// Empty retained baseline.
    pub const EMPTY: Self = Self {
        pruned_entry_count: 0,
        pruned_encoded_weight: 0,
        last_pruned_entry_id: None,
        last_pruned_sequence: None,
    };

    /// Constructs explicit imported baseline evidence.
    #[must_use]
    pub const fn new(
        pruned_entry_count: u64,
        pruned_encoded_weight: u64,
        last_pruned_entry_id: Option<HistoryEntryId>,
        last_pruned_sequence: Option<HistoryEntrySequence>,
    ) -> Self {
        Self {
            pruned_entry_count,
            pruned_encoded_weight,
            last_pruned_entry_id,
            last_pruned_sequence,
        }
    }

    /// Returns the number of applied entries absorbed into the baseline.
    #[must_use]
    pub const fn pruned_entry_count(&self) -> u64 {
        self.pruned_entry_count
    }

    /// Returns cumulative encoded weight absorbed into the baseline.
    #[must_use]
    pub const fn pruned_encoded_weight(&self) -> u64 {
        self.pruned_encoded_weight
    }

    /// Returns the last entry absorbed into the baseline.
    #[must_use]
    pub const fn last_pruned_entry_id(&self) -> Option<&HistoryEntryId> {
        self.last_pruned_entry_id.as_ref()
    }

    /// Returns the last insertion sequence absorbed into the baseline.
    #[must_use]
    pub const fn last_pruned_sequence(&self) -> Option<HistoryEntrySequence> {
        self.last_pruned_sequence
    }

    /// Returns whether retained history starts after an advanced baseline.
    #[must_use]
    pub const fn is_advanced(&self) -> bool {
        self.pruned_entry_count != 0
    }

    pub(crate) fn checked_advance<P>(
        &self,
        entries: &[HistoryEntry<P>],
    ) -> Result<Self, HistoryRetentionError> {
        let added_count =
            u64::try_from(entries.len()).map_err(|_| HistoryRetentionError::BaselineOverflow)?;
        let pruned_entry_count = self
            .pruned_entry_count
            .checked_add(added_count)
            .ok_or(HistoryRetentionError::BaselineOverflow)?;
        let added_weight = entries.iter().try_fold(0_u64, |total, entry| {
            total
                .checked_add(entry.encoded_weight())
                .ok_or(HistoryRetentionError::EncodedWeightOverflow)
        })?;
        let pruned_encoded_weight = self
            .pruned_encoded_weight
            .checked_add(added_weight)
            .ok_or(HistoryRetentionError::BaselineOverflow)?;
        let last = entries.last();
        Ok(Self {
            pruned_entry_count,
            pruned_encoded_weight,
            last_pruned_entry_id: last
                .map(|entry| entry.entry_id().clone())
                .or_else(|| self.last_pruned_entry_id.clone()),
            last_pruned_sequence: last
                .map(HistoryEntry::sequence)
                .or(self.last_pruned_sequence),
        })
    }
}

/// Exact metadata for one entry removed by retention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPrunedEntry {
    entry_id: HistoryEntryId,
    sequence: HistoryEntrySequence,
    encoded_weight: u64,
}

impl HistoryPrunedEntry {
    pub(crate) fn from_entry<P>(entry: &HistoryEntry<P>) -> Self {
        Self {
            entry_id: entry.entry_id().clone(),
            sequence: entry.sequence(),
            encoded_weight: entry.encoded_weight(),
        }
    }

    /// Returns the removed entry identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns the removed entry insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> HistoryEntrySequence {
        self.sequence
    }

    /// Returns the removed entry encoded weight.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }
}

/// Exact applied-prefix and future-tail removals from retention.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HistoryPruningReceipt {
    advanced_baseline: Vec<HistoryPrunedEntry>,
    discarded_future: Vec<HistoryPrunedEntry>,
}

impl HistoryPruningReceipt {
    pub(crate) fn new(
        advanced_baseline: Vec<HistoryPrunedEntry>,
        discarded_future: Vec<HistoryPrunedEntry>,
    ) -> Self {
        Self {
            advanced_baseline,
            discarded_future,
        }
    }

    /// Returns oldest-first applied entries absorbed into the baseline.
    #[must_use]
    pub fn advanced_baseline(&self) -> &[HistoryPrunedEntry] {
        &self.advanced_baseline
    }

    /// Returns farthest-first future entries made unreachable.
    #[must_use]
    pub fn discarded_future(&self) -> &[HistoryPrunedEntry] {
        &self.discarded_future
    }

    /// Returns whether retention removed no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.advanced_baseline.is_empty() && self.discarded_future.is_empty()
    }
}

/// Committed retention-limit change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryLimitChangeReceipt {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    previous_limits: HistoryLimits,
    authoritative_limits: HistoryLimits,
    pruning: HistoryPruningReceipt,
    retained_encoded_weight: u64,
    transition: Option<HistoryCommittedTransition>,
}

impl HistoryLimitChangeReceipt {
    /// Returns the admitted source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the authoritative resulting revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the replaced limits.
    #[must_use]
    pub const fn previous_limits(&self) -> HistoryLimits {
        self.previous_limits
    }

    /// Returns the committed limits.
    #[must_use]
    pub const fn authoritative_limits(&self) -> HistoryLimits {
        self.authoritative_limits
    }

    /// Returns exact pruning evidence.
    #[must_use]
    pub const fn pruning(&self) -> &HistoryPruningReceipt {
        &self.pruning
    }

    /// Returns authoritative retained encoded weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }

    /// Returns the committed transition, absent when limits were unchanged.
    #[must_use]
    pub const fn transition(&self) -> Option<&HistoryCommittedTransition> {
        self.transition.as_ref()
    }
}

impl<P> LinearHistory<P> {
    /// Returns total retained consumer-measured encoded payload weight.
    pub fn retained_encoded_weight(&self) -> Result<u64, HistoryRetentionError> {
        retained_weight(&self.state.applied, &self.state.future)
    }

    /// Changes count, encoded-weight, and label limits with exact pruning.
    pub fn change_limits(
        &mut self,
        expected_revision: HistoryRevision,
        new_limits: HistoryLimits,
    ) -> Result<HistoryLimitChangeReceipt, HistoryLimitChangeError> {
        if expected_revision != self.state.revision {
            return Err(HistoryLimitChangeError::StaleRevision {
                expected: expected_revision,
                actual: self.state.revision,
            });
        }
        if self.limits == new_limits {
            return Ok(HistoryLimitChangeReceipt {
                previous_revision: self.state.revision,
                committed_revision: self.state.revision,
                previous_limits: self.limits,
                authoritative_limits: self.limits,
                pruning: HistoryPruningReceipt::default(),
                retained_encoded_weight: self
                    .retained_encoded_weight()
                    .map_err(HistoryLimitChangeError::Retention)?,
                transition: None,
            });
        }
        if let Some(entry) = self
            .state
            .applied
            .iter()
            .chain(&self.state.future)
            .find(|entry| entry.metadata().label().len_bytes() > new_limits.maximum_label_bytes())
        {
            return Err(HistoryLimitChangeError::RetainedLabelTooLong {
                entry_id: entry.entry_id().clone(),
                maximum: new_limits.maximum_label_bytes(),
                actual: entry.metadata().label().len_bytes(),
            });
        }
        let committed_revision = self
            .state
            .revision
            .checked_next()
            .map_err(|_| HistoryLimitChangeError::RevisionOverflow)?;
        let mut retained_count = self.state.applied.len() + self.state.future.len();
        let mut retained_weight = self
            .retained_encoded_weight()
            .map_err(HistoryLimitChangeError::Retention)?;
        let mut applied_to_prune = 0;
        while retained_count > new_limits.maximum_entries()
            || retained_weight > new_limits.maximum_encoded_weight()
        {
            let Some(entry) = self.state.applied.get(applied_to_prune) else {
                break;
            };
            retained_count -= 1;
            retained_weight -= entry.encoded_weight();
            applied_to_prune += 1;
        }
        let prospective_baseline = self
            .state
            .retained_baseline
            .checked_advance(&self.state.applied[..applied_to_prune])
            .map_err(HistoryLimitChangeError::Retention)?;

        let mut future_to_discard = 0;
        while retained_count > new_limits.maximum_entries()
            || retained_weight > new_limits.maximum_encoded_weight()
        {
            let entry = self
                .state
                .future
                .get(future_to_discard)
                .ok_or(HistoryLimitChangeError::ImpossibleLimits)?;
            retained_count -= 1;
            retained_weight -= entry.encoded_weight();
            future_to_discard += 1;
        }

        let advanced_baseline = self.state.applied[..applied_to_prune]
            .iter()
            .map(HistoryPrunedEntry::from_entry)
            .collect();
        let discarded_future = self.state.future[..future_to_discard]
            .iter()
            .map(HistoryPrunedEntry::from_entry)
            .collect();
        self.state.applied.drain(..applied_to_prune);
        self.state.future.drain(..future_to_discard);
        self.state.retained_baseline = prospective_baseline;
        let previous_limits = self.limits;
        self.limits = new_limits;
        self.state.revision = committed_revision;
        self.close_transient_group(crate::HistoryGroupCloseReason::AuthorityChange);
        let pruning = HistoryPruningReceipt::new(advanced_baseline, discarded_future);
        let transition = HistoryCommittedTransition::new(
            self.state.history_id.clone(),
            Some(expected_revision),
            committed_revision,
            HistoryCommittedTransitionKind::LimitsChanged {
                previous_limits,
                authoritative_limits: new_limits,
                pruning: pruning.clone(),
            },
        );

        Ok(HistoryLimitChangeReceipt {
            previous_revision: expected_revision,
            committed_revision,
            previous_limits,
            authoritative_limits: new_limits,
            pruning,
            retained_encoded_weight: retained_weight,
            transition: Some(transition),
        })
    }
}

pub(crate) fn retained_weight<P>(
    applied: &[HistoryEntry<P>],
    future: &[HistoryEntry<P>],
) -> Result<u64, HistoryRetentionError> {
    applied
        .iter()
        .chain(future)
        .try_fold(0_u64, |total, entry| {
            total
                .checked_add(entry.encoded_weight())
                .ok_or(HistoryRetentionError::EncodedWeightOverflow)
        })
}

/// Invalid retained-weight or baseline arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryRetentionError {
    /// Retained encoded weights overflowed.
    EncodedWeightOverflow,
    /// Cumulative retained-baseline evidence overflowed.
    BaselineOverflow,
}

impl fmt::Display for HistoryRetentionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EncodedWeightOverflow => formatter.write_str("history encoded weight overflowed"),
            Self::BaselineOverflow => formatter.write_str("history retained baseline overflowed"),
        }
    }
}

impl Error for HistoryRetentionError {}

/// Rejected retention-limit change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryLimitChangeError {
    /// The request targeted a stale structural revision.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// The structural revision cannot advance.
    RevisionOverflow,
    /// Existing metadata cannot satisfy the new label bound.
    RetainedLabelTooLong {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Requested maximum.
        maximum: usize,
        /// Retained byte length.
        actual: usize,
    },
    /// No safe applied-prefix or future-tail pruning can satisfy the limits.
    ImpossibleLimits,
    /// Retained arithmetic failed closed.
    Retention(HistoryRetentionError),
}

impl fmt::Display for HistoryLimitChangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "expected history revision {}; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::RevisionOverflow => formatter.write_str("history revision cannot advance"),
            Self::RetainedLabelTooLong {
                entry_id,
                maximum,
                actual,
            } => write!(
                formatter,
                "history entry {entry_id} label is {actual} bytes; requested maximum is {maximum}"
            ),
            Self::ImpossibleLimits => {
                formatter.write_str("history limits cannot be satisfied safely")
            }
            Self::Retention(error) => write!(formatter, "history retention failed: {error}"),
        }
    }
}

impl Error for HistoryLimitChangeError {}
