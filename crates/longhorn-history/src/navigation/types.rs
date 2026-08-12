//! Navigation request, target, step, and position types.

use longhorn_core::{HistoryEntryId, HistoryPlanId, HistoryRevision};

use crate::HistoryLabel;

/// Stable navigation intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationTarget {
    /// Move one entry toward the retained baseline.
    Undo,
    /// Move one entry toward the newest retained state.
    Redo,
    /// Make one stable entry the current applied entry.
    Checkout {
        /// Entry identity, never a presentation index.
        entry_id: HistoryEntryId,
    },
    /// Return to the position before the oldest retained entry.
    ///
    /// The state the operator started from, and the only one with no entry to
    /// name it -- so `Checkout` cannot express it and reaching it meant one
    /// undo per entry. A separate variant rather than an optional `entry_id`,
    /// for the reason the fork domain's `CheckoutBranchRoot` is separate: an
    /// optional field makes every match site handle a combination that means
    /// something for one state and nothing for the other.
    ///
    /// "Oldest retained", not "first ever". Retention prunes from the oldest
    /// end and records what it took in the baseline projection, so after a
    /// prune this is a baseline rather than the origin. The page says which;
    /// this target only moves.
    CheckoutRoot,
}

/// One injected, revision-bound navigation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationRequest {
    pub(crate) plan_id: HistoryPlanId,
    pub(crate) expected_revision: HistoryRevision,
    pub(crate) target: HistoryNavigationTarget,
}

impl HistoryNavigationRequest {
    /// Constructs a navigation request.
    #[must_use]
    pub const fn new(
        plan_id: HistoryPlanId,
        expected_revision: HistoryRevision,
        target: HistoryNavigationTarget,
    ) -> Self {
        Self {
            plan_id,
            expected_revision,
            target,
        }
    }

    /// Returns the injected plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the exact source revision required for planning.
    #[must_use]
    pub const fn expected_revision(&self) -> HistoryRevision {
        self.expected_revision
    }

    /// Returns the requested stable target.
    #[must_use]
    pub const fn target(&self) -> &HistoryNavigationTarget {
        &self.target
    }
}

/// Direction of one planned linear navigation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryNavigationDirection {
    /// Move toward the retained baseline.
    Undo,
    /// Move toward the newest retained state.
    Redo,
    /// Remain at the same entry while committing an explicit checkout.
    Stationary,
}

/// One ordered typed payload application in a navigation batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationStep<P> {
    /// Apply the consumer-produced inverse of one current entry.
    Undo {
        /// Entry being unapplied.
        entry_id: HistoryEntryId,
        /// Typed inverse payload.
        payload: P,
    },
    /// Apply one retained forward payload.
    Redo {
        /// Entry being reapplied.
        entry_id: HistoryEntryId,
        /// Typed forward payload.
        payload: P,
    },
}

impl<P> HistoryNavigationStep<P> {
    /// Returns the entry affected by this step.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        match self {
            Self::Undo { entry_id, .. } | Self::Redo { entry_id, .. } => entry_id,
        }
    }

    /// Returns the typed consumer payload to apply.
    #[must_use]
    pub const fn payload(&self) -> &P {
        match self {
            Self::Undo { payload, .. } | Self::Redo { payload, .. } => payload,
        }
    }

    /// Returns this step's direction.
    #[must_use]
    pub const fn direction(&self) -> HistoryNavigationDirection {
        match self {
            Self::Undo { .. } => HistoryNavigationDirection::Undo,
            Self::Redo { .. } => HistoryNavigationDirection::Redo,
        }
    }
}

/// Bounded authoritative position metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationPosition {
    pub(crate) applied_depth: usize,
    pub(crate) future_depth: usize,
    pub(crate) current_entry_id: Option<HistoryEntryId>,
    pub(crate) next_undo_label: Option<HistoryLabel>,
    pub(crate) next_redo_entry_id: Option<HistoryEntryId>,
    pub(crate) next_redo_label: Option<HistoryLabel>,
}

impl HistoryNavigationPosition {
    /// Returns the number of applied entries.
    #[must_use]
    pub const fn applied_depth(&self) -> usize {
        self.applied_depth
    }

    /// Returns the number of future entries.
    #[must_use]
    pub const fn future_depth(&self) -> usize {
        self.future_depth
    }

    /// Returns the current applied entry.
    #[must_use]
    pub const fn current_entry_id(&self) -> Option<&HistoryEntryId> {
        self.current_entry_id.as_ref()
    }

    /// Returns the next undo label.
    #[must_use]
    pub const fn next_undo_label(&self) -> Option<&HistoryLabel> {
        self.next_undo_label.as_ref()
    }

    /// Returns the next redo entry.
    #[must_use]
    pub const fn next_redo_entry_id(&self) -> Option<&HistoryEntryId> {
        self.next_redo_entry_id.as_ref()
    }

    /// Returns the next redo label.
    #[must_use]
    pub const fn next_redo_label(&self) -> Option<&HistoryLabel> {
        self.next_redo_label.as_ref()
    }
}
