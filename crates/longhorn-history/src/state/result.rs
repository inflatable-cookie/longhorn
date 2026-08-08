//! Record outcomes and state validation errors.

use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryRevision};

use crate::{
    HistoryCommittedTransition, HistoryEntrySequence, HistoryPruningReceipt, HistoryRetentionError,
};

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
    pub(crate) previous_revision: HistoryRevision,
    pub(crate) committed_revision: HistoryRevision,
    pub(crate) outcome: HistoryRecordOutcome,
    pub(crate) cleared_future: Vec<HistoryEntryId>,
    pub(crate) pruning: HistoryPruningReceipt,
    pub(crate) retained_encoded_weight: u64,
    pub(crate) transition: Option<HistoryCommittedTransition>,
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
