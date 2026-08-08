//! Fork-history structural import state.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::HistoryEntrySequence;

use crate::{ForkBranch, ForkBranchId, ForkCheckpoint, ForkHistoryNode};

/// Defensive hard ceiling for retained fork-history nodes.
pub const MAXIMUM_FORK_NODES: usize = 65_536;
/// Defensive hard ceiling for stable branch references.
pub const MAXIMUM_FORK_BRANCHES: usize = 4_096;

/// One preferred redo edge supplied to structural-state validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkPreferredChild {
    pub(crate) parent_entry_id: Option<HistoryEntryId>,
    pub(crate) child_entry_id: HistoryEntryId,
}

impl ForkPreferredChild {
    /// Constructs one preferred direct-child relation.
    #[must_use]
    pub const fn new(
        parent_entry_id: Option<HistoryEntryId>,
        child_entry_id: HistoryEntryId,
    ) -> Self {
        Self {
            parent_entry_id,
            child_entry_id,
        }
    }

    /// Returns the parent node, or the graph root.
    #[must_use]
    pub const fn parent_entry_id(&self) -> Option<&HistoryEntryId> {
        self.parent_entry_id.as_ref()
    }

    /// Returns the preferred direct child.
    #[must_use]
    pub const fn child_entry_id(&self) -> &HistoryEntryId {
        &self.child_entry_id
    }
}

/// Complete untrusted structural state offered to [`ForkHistory::from_state`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkHistoryState<P> {
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) nodes: Vec<ForkHistoryNode<P>>,
    pub(crate) branches: Vec<ForkBranch>,
    pub(crate) current_branch_id: ForkBranchId,
    pub(crate) current_node_id: Option<HistoryEntryId>,
    pub(crate) preferred_children: Vec<ForkPreferredChild>,
    pub(crate) checkpoints: Vec<ForkCheckpoint>,
    pub(crate) next_sequence: HistoryEntrySequence,
}

impl<P> ForkHistoryState<P> {
    /// Starts a structural-state import description.
    #[must_use]
    pub const fn new(
        history_id: HistoryId,
        revision: HistoryRevision,
        current_branch_id: ForkBranchId,
        current_node_id: Option<HistoryEntryId>,
        next_sequence: HistoryEntrySequence,
    ) -> Self {
        Self {
            history_id,
            revision,
            nodes: Vec::new(),
            branches: Vec::new(),
            current_branch_id,
            current_node_id,
            preferred_children: Vec::new(),
            checkpoints: Vec::new(),
            next_sequence,
        }
    }

    /// Supplies immutable nodes for validation.
    #[must_use]
    pub fn with_nodes(mut self, nodes: Vec<ForkHistoryNode<P>>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Supplies stable branch references for validation.
    #[must_use]
    pub fn with_branches(mut self, branches: Vec<ForkBranch>) -> Self {
        self.branches = branches;
        self
    }

    /// Supplies preferred direct-child relations for validation.
    #[must_use]
    pub fn with_preferred_children(mut self, preferred_children: Vec<ForkPreferredChild>) -> Self {
        self.preferred_children = preferred_children;
        self
    }

    /// Supplies opaque checkpoint references for validation.
    #[must_use]
    pub fn with_checkpoints(mut self, checkpoints: Vec<ForkCheckpoint>) -> Self {
        self.checkpoints = checkpoints;
        self
    }

    /// Returns the graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the structural revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the supplied nodes.
    #[must_use]
    pub fn nodes(&self) -> &[ForkHistoryNode<P>] {
        &self.nodes
    }

    /// Returns the supplied branches.
    #[must_use]
    pub fn branches(&self) -> &[ForkBranch] {
        &self.branches
    }

    /// Returns the current branch identity.
    #[must_use]
    pub const fn current_branch_id(&self) -> &ForkBranchId {
        &self.current_branch_id
    }

    /// Returns the current node, or the graph root.
    #[must_use]
    pub const fn current_node_id(&self) -> Option<&HistoryEntryId> {
        self.current_node_id.as_ref()
    }

    /// Returns preferred direct-child relations.
    #[must_use]
    pub fn preferred_children(&self) -> &[ForkPreferredChild] {
        &self.preferred_children
    }

    /// Returns supplied checkpoint references.
    #[must_use]
    pub fn checkpoints(&self) -> &[ForkCheckpoint] {
        &self.checkpoints
    }

    /// Returns the next insertion sequence.
    #[must_use]
    pub const fn next_sequence(&self) -> HistoryEntrySequence {
        self.next_sequence
    }
}
