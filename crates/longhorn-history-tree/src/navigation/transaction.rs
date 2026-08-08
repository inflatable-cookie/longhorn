//! Navigation transaction port and receipts.

use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryNavigationStep, HistoryNavigationTransactionFailure, HistoryPolicy,
    MAXIMUM_HISTORY_NAVIGATION_STEPS,
};

use crate::{ForkBranchId, ForkHistory};

use super::ForkNavigationPlan;
/// Consumer-owned atomic product transaction for one complete graph route.

pub trait ForkNavigationTransaction<P> {
    /// Product apply or rollback failure.
    type Error;

    /// Applies the complete mixed route atomically.
    fn apply(
        &mut self,
        plan: &ForkNavigationPlan<P>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>>;
}

/// Successful graph navigation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkNavigationReceipt {
    pub(crate) history_id: HistoryId,
    pub(crate) previous_revision: HistoryRevision,
    pub(crate) committed_revision: HistoryRevision,
    pub(crate) plan_id: HistoryPlanId,
    pub(crate) source_node_id: Option<HistoryEntryId>,
    pub(crate) target_node_id: Option<HistoryEntryId>,
    pub(crate) target_branch_id: ForkBranchId,
    pub(crate) moved_entry_ids: Vec<HistoryEntryId>,
}

impl ForkNavigationReceipt {
    /// Returns the graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the source graph revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the committed graph revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the committed plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the source node, or root.
    #[must_use]
    pub const fn source_node_id(&self) -> Option<&HistoryEntryId> {
        self.source_node_id.as_ref()
    }

    /// Returns the target node, or root.
    #[must_use]
    pub const fn target_node_id(&self) -> Option<&HistoryEntryId> {
        self.target_node_id.as_ref()
    }

    /// Returns the selected first-class branch.
    #[must_use]
    pub const fn target_branch_id(&self) -> &ForkBranchId {
        &self.target_branch_id
    }

    /// Returns entry identities in apply order.
    #[must_use]
    pub fn moved_entry_ids(&self) -> &[HistoryEntryId] {
        &self.moved_entry_ids
    }
}
