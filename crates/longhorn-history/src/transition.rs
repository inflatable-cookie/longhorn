use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};

use crate::{
    HistoryEntrySequence, HistoryLimits, HistoryNavigationDirection, HistoryNavigationPosition,
    HistoryPruningReceipt, HistoryRecordOutcome, HistoryRetainedBaseline, LinearHistory,
};

/// One payload-free record effect committed by the history authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryRecordTransitionEffect {
    /// One fresh entry was retained.
    Added {
        /// Retained entry identity.
        entry_id: HistoryEntryId,
        /// Allocated insertion sequence.
        sequence: HistoryEntrySequence,
    },
    /// Coalescing replaced one retained entry in place.
    Replaced {
        /// Retained entry identity.
        entry_id: HistoryEntryId,
        /// Retained insertion sequence.
        sequence: HistoryEntrySequence,
    },
    /// Coalescing removed one retained entry.
    Removed {
        /// Removed entry identity.
        entry_id: HistoryEntryId,
        /// Removed insertion sequence.
        sequence: HistoryEntrySequence,
    },
}

impl HistoryRecordTransitionEffect {
    pub(crate) fn from_outcome(outcome: &HistoryRecordOutcome) -> Option<Self> {
        match outcome {
            HistoryRecordOutcome::IgnoredNoOp { .. } => None,
            HistoryRecordOutcome::Added { entry_id, sequence } => Some(Self::Added {
                entry_id: entry_id.clone(),
                sequence: *sequence,
            }),
            HistoryRecordOutcome::Replaced { entry_id, sequence } => Some(Self::Replaced {
                entry_id: entry_id.clone(),
                sequence: *sequence,
            }),
            HistoryRecordOutcome::Removed { entry_id, sequence } => Some(Self::Removed {
                entry_id: entry_id.clone(),
                sequence: *sequence,
            }),
        }
    }
}

/// Payload-free kind-specific evidence for one committed structural transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryCommittedTransitionKind {
    /// A record was added or coalesced after product success.
    Record {
        /// Exact retained record effect.
        effect: HistoryRecordTransitionEffect,
        /// Future ids cleared in ascending insertion order.
        cleared_future: Vec<HistoryEntryId>,
        /// Retention pruning committed in the same revision.
        pruning: HistoryPruningReceipt,
    },
    /// One checked navigation plan committed after product success.
    Navigation {
        /// Committed plan identity.
        plan_id: HistoryPlanId,
        /// Navigation direction.
        direction: HistoryNavigationDirection,
        /// Entries moved in product-apply order.
        moved_entry_ids: Vec<HistoryEntryId>,
        /// Admitted source position.
        source_position: HistoryNavigationPosition,
        /// Authoritative resulting position.
        authoritative_position: HistoryNavigationPosition,
    },
    /// Retention limits changed.
    LimitsChanged {
        /// Replaced limits.
        previous_limits: HistoryLimits,
        /// Authoritative limits.
        authoritative_limits: HistoryLimits,
        /// Exact pruning committed with the change.
        pruning: HistoryPruningReceipt,
    },
    /// A complete persisted linear authority was accepted.
    Imported {
        /// Structural version found in the source.
        source_structural_version: u32,
        /// Structural version accepted by this implementation.
        structural_version: u32,
        /// Registered consumer codec family.
        payload_codec_family: String,
        /// Payload codec version found in the source.
        source_payload_codec_version: u32,
        /// Payload codec version accepted by this implementation.
        payload_codec_version: u32,
        /// Number of applied entries accepted.
        applied_entries: u64,
        /// Number of future entries accepted.
        future_entries: u64,
    },
    /// A consumer explicitly discarded an unusable persisted history.
    DiscardedPersistence {
        /// Caller-owned recovery reason.
        reason: HistoryDiscardReason,
    },
    /// The consumer committed a reset of retained structural history.
    Reset {
        /// Oldest-first applied entries removed.
        removed_applied: Vec<HistoryEntryId>,
        /// Next-redo-first future entries removed.
        removed_future: Vec<HistoryEntryId>,
        /// Durable baseline replaced by the reset.
        previous_baseline: HistoryRetainedBaseline,
    },
}

/// One authoritative, payload-free structural transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryCommittedTransition {
    history_id: HistoryId,
    previous_revision: Option<HistoryRevision>,
    committed_revision: HistoryRevision,
    kind: HistoryCommittedTransitionKind,
}

impl HistoryCommittedTransition {
    pub(crate) const fn new(
        history_id: HistoryId,
        previous_revision: Option<HistoryRevision>,
        committed_revision: HistoryRevision,
        kind: HistoryCommittedTransitionKind,
    ) -> Self {
        Self {
            history_id,
            previous_revision,
            committed_revision,
            kind,
        }
    }

    /// Returns the owning history authority.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the prior in-memory revision, absent for load recovery.
    #[must_use]
    pub const fn previous_revision(&self) -> Option<HistoryRevision> {
        self.previous_revision
    }

    /// Returns the authoritative resulting revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns kind-specific structural evidence.
    #[must_use]
    pub const fn kind(&self) -> &HistoryCommittedTransitionKind {
        &self.kind
    }
}

/// Consumer-selected reason for visibly discarding persisted history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryDiscardReason {
    /// Source bytes were corrupt or failed structural invariants.
    CorruptSource,
    /// Source versions, identities, or payload policy were incompatible.
    IncompatibleSource,
    /// A visible consumer migration deliberately starts fresh history.
    ConsumerMigration,
}

/// Successful committed structural reset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryResetReceipt {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    removed_applied: Vec<HistoryEntryId>,
    removed_future: Vec<HistoryEntryId>,
    previous_baseline: HistoryRetainedBaseline,
    transition: Option<HistoryCommittedTransition>,
}

impl HistoryResetReceipt {
    /// Returns the admitted source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the resulting revision, unchanged for an empty reset.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns oldest-first applied entries removed by the reset.
    #[must_use]
    pub fn removed_applied(&self) -> &[HistoryEntryId] {
        &self.removed_applied
    }

    /// Returns next-redo-first future entries removed by the reset.
    #[must_use]
    pub fn removed_future(&self) -> &[HistoryEntryId] {
        &self.removed_future
    }

    /// Returns durable retained-baseline evidence removed by the reset.
    #[must_use]
    pub const fn previous_baseline(&self) -> &HistoryRetainedBaseline {
        &self.previous_baseline
    }

    /// Returns the committed transition, absent when reset changed nothing.
    #[must_use]
    pub const fn transition(&self) -> Option<&HistoryCommittedTransition> {
        self.transition.as_ref()
    }
}

impl<P> LinearHistory<P> {
    /// Resets structural history after the consumer has committed its model reset.
    ///
    /// Insertion sequence remains monotonic across the reset. An already-empty
    /// authority returns an unchanged receipt and emits no transition.
    pub fn reset_committed(
        &mut self,
        expected_revision: HistoryRevision,
    ) -> Result<HistoryResetReceipt, HistoryResetError> {
        if expected_revision != self.state.revision {
            return Err(HistoryResetError::StaleRevision {
                expected: expected_revision,
                actual: self.state.revision,
            });
        }

        let removed_applied = self
            .state
            .applied
            .iter()
            .map(|entry| entry.entry_id().clone())
            .collect::<Vec<_>>();
        let removed_future = self
            .state
            .future
            .iter()
            .rev()
            .map(|entry| entry.entry_id().clone())
            .collect::<Vec<_>>();
        let previous_baseline = self.state.retained_baseline.clone();
        let changed = !removed_applied.is_empty()
            || !removed_future.is_empty()
            || previous_baseline != HistoryRetainedBaseline::EMPTY;
        if !changed {
            return Ok(HistoryResetReceipt {
                previous_revision: expected_revision,
                committed_revision: expected_revision,
                removed_applied,
                removed_future,
                previous_baseline,
                transition: None,
            });
        }

        let committed_revision = expected_revision
            .checked_next()
            .map_err(|_| HistoryResetError::RevisionOverflow)?;
        self.state.applied.clear();
        self.state.future.clear();
        self.state.retained_baseline = HistoryRetainedBaseline::EMPTY;
        self.state.revision = committed_revision;
        self.close_transient_group(crate::HistoryGroupCloseReason::AuthorityChange);

        let transition = HistoryCommittedTransition::new(
            self.state.history_id.clone(),
            Some(expected_revision),
            committed_revision,
            HistoryCommittedTransitionKind::Reset {
                removed_applied: removed_applied.clone(),
                removed_future: removed_future.clone(),
                previous_baseline: previous_baseline.clone(),
            },
        );
        Ok(HistoryResetReceipt {
            previous_revision: expected_revision,
            committed_revision,
            removed_applied,
            removed_future,
            previous_baseline,
            transition: Some(transition),
        })
    }
}

/// Rejected structural reset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryResetError {
    /// The request targeted a stale structural revision.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// The structural revision cannot advance.
    RevisionOverflow,
}

impl fmt::Display for HistoryResetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "expected history revision {}; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::RevisionOverflow => formatter.write_str("history revision cannot advance"),
        }
    }
}

impl Error for HistoryResetError {}
