//! Navigation targets and plans.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};
use longhorn_history::HistoryNavigationStep;

use crate::ForkBranchId;
/// Stable graph navigation intent.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkNavigationTarget {
    /// Move to the parent of the current node.
    Undo,
    /// Move to the deterministic preferred child.
    Redo,
    /// Move to one stable entry on one first-class branch.
    Checkout {
        /// Stable branch reference.
        branch_id: ForkBranchId,
        /// Stable target entry.
        entry_id: HistoryEntryId,
    },
    /// Move to a branch's root, holding no entry.
    ///
    /// A separate target rather than an optional `entry_id` on `Checkout`.
    /// Optional would make every match site handle a combination that means
    /// something for one branch state and nothing for the other, and callers
    /// would have to know that `None` is the root rather than "unspecified".
    ///
    /// This is the position a nascent branch starts in and the one a
    /// root-only switch asks for. It is already representable everywhere
    /// downstream -- the resolver returns `Option<HistoryEntryId>` and the
    /// plan compares it against a current node that is equally optional --
    /// so only the target could not say it.
    CheckoutBranchRoot {
        /// Stable branch reference.
        branch_id: ForkBranchId,
    },
}

/// Immutable mixed undo/redo route bound to exact graph authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkNavigationPlan<P> {
    pub(crate) history_id: HistoryId,
    pub(crate) history_revision: HistoryRevision,
    pub(crate) plan_id: HistoryPlanId,
    pub(crate) target: ForkNavigationTarget,
    pub(crate) source_branch_id: ForkBranchId,
    pub(crate) target_branch_id: ForkBranchId,
    pub(crate) source_node_id: Option<HistoryEntryId>,
    pub(crate) target_node_id: Option<HistoryEntryId>,
    pub(crate) lowest_common_ancestor: Option<HistoryEntryId>,
    pub(crate) steps: Vec<HistoryNavigationStep<P>>,
}

impl<P> ForkNavigationPlan<P> {
    /// Returns the bound graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the exact source revision.
    #[must_use]
    pub const fn history_revision(&self) -> HistoryRevision {
        self.history_revision
    }

    /// Returns the injected plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the requested navigation intent.
    #[must_use]
    pub const fn target(&self) -> &ForkNavigationTarget {
        &self.target
    }

    /// Returns the source branch.
    #[must_use]
    pub const fn source_branch_id(&self) -> &ForkBranchId {
        &self.source_branch_id
    }

    /// Returns the target branch.
    #[must_use]
    pub const fn target_branch_id(&self) -> &ForkBranchId {
        &self.target_branch_id
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

    /// Returns the route's lowest common ancestor, or root.
    #[must_use]
    pub const fn lowest_common_ancestor(&self) -> Option<&HistoryEntryId> {
        self.lowest_common_ancestor.as_ref()
    }

    /// Returns the complete ordered product payload batch.
    #[must_use]
    pub fn steps(&self) -> &[HistoryNavigationStep<P>] {
        &self.steps
    }
}
