//! Navigation transaction port and receipts.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};

use crate::HistoryCommittedTransition;

use super::{HistoryNavigationDirection, HistoryNavigationPlan, HistoryNavigationPosition};

/// Consumer-owned atomic product transaction for one complete plan.
pub trait HistoryNavigationTransaction<P> {
    /// Product apply or rollback failure.
    type Error;

    /// Applies every step atomically or returns exact rollback evidence.
    fn apply(
        &mut self,
        plan: &HistoryNavigationPlan<P>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>>;
}

/// Consumer transaction failure evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationTransactionFailure<E> {
    /// Product apply failed and the exact source model was restored.
    RolledBack {
        /// Product apply failure.
        error: E,
    },
    /// Product apply failed and exact rollback also failed.
    RollbackFailed {
        /// Product apply failure.
        error: E,
        /// Rollback failure.
        rollback_error: E,
    },
}

/// Successful committed linear navigation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationReceipt {
    pub(crate) history_id: HistoryId,
    pub(crate) plan_id: HistoryPlanId,
    pub(crate) previous_revision: HistoryRevision,
    pub(crate) committed_revision: HistoryRevision,
    pub(crate) direction: HistoryNavigationDirection,
    pub(crate) moved_entry_ids: Vec<HistoryEntryId>,
    pub(crate) source_position: HistoryNavigationPosition,
    pub(crate) authoritative_position: HistoryNavigationPosition,
    pub(crate) transition: HistoryCommittedTransition,
}

impl HistoryNavigationReceipt {
    /// Returns the owning history authority.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the committed plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the admitted source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the committed successor revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the committed direction.
    #[must_use]
    pub const fn direction(&self) -> HistoryNavigationDirection {
        self.direction
    }

    /// Returns moved entry ids in product-apply order.
    #[must_use]
    pub fn moved_entry_ids(&self) -> &[HistoryEntryId] {
        &self.moved_entry_ids
    }

    /// Returns the admitted source position.
    #[must_use]
    pub const fn source_position(&self) -> &HistoryNavigationPosition {
        &self.source_position
    }

    /// Returns authoritative committed position metadata.
    #[must_use]
    pub const fn authoritative_position(&self) -> &HistoryNavigationPosition {
        &self.authoritative_position
    }

    /// Returns the committed structural transition.
    #[must_use]
    pub const fn transition(&self) -> &HistoryCommittedTransition {
        &self.transition
    }
}
