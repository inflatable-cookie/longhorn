use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet},
    error::Error,
    fmt,
};

use longhorn_core::{HistoryEntryId, HistoryRevision};
use longhorn_history::{HistoryEntrySequence, MAXIMUM_HISTORY_ENCODED_WEIGHT};

use crate::{ForkBranchId, ForkCheckpointId, ForkHistory, MAXIMUM_FORK_NODES};

/// Count and exact encoded-weight budgets for graph pruning.
///
/// These bound the **unprotected** share, not the whole graph. A protected
/// entry -- one on the current branch or on a pinned branch -- is a core
/// record of the project and is never counted against a budget or removed by
/// one. Size these for the transient history you want to keep; a graph with a
/// large pinned set will exceed them and that is correct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkRetentionLimits {
    maximum_entries: usize,
    maximum_encoded_weight: u64,
}

impl ForkRetentionLimits {
    /// Validates nonzero budgets against defensive hard ceilings.
    pub const fn new(
        maximum_entries: usize,
        maximum_encoded_weight: u64,
    ) -> Result<Self, ForkRetentionError> {
        if maximum_entries == 0 || maximum_encoded_weight == 0 {
            return Err(ForkRetentionError::ZeroLimit);
        }
        if maximum_entries > MAXIMUM_FORK_NODES {
            return Err(ForkRetentionError::EntryLimitTooLarge {
                maximum: MAXIMUM_FORK_NODES,
                actual: maximum_entries,
            });
        }
        if maximum_encoded_weight > MAXIMUM_HISTORY_ENCODED_WEIGHT {
            return Err(ForkRetentionError::EncodedWeightLimitTooLarge {
                maximum: MAXIMUM_HISTORY_ENCODED_WEIGHT,
                actual: maximum_encoded_weight,
            });
        }
        Ok(Self {
            maximum_entries,
            maximum_encoded_weight,
        })
    }

    /// Returns the maximum retained node count.
    #[must_use]
    pub const fn maximum_entries(self) -> usize {
        self.maximum_entries
    }

    /// Returns the maximum retained encoded payload weight.
    #[must_use]
    pub const fn maximum_encoded_weight(self) -> u64 {
        self.maximum_encoded_weight
    }
}

/// One deterministically removed immutable node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkPrunedNode {
    entry_id: HistoryEntryId,
    sequence: HistoryEntrySequence,
    encoded_weight: u64,
}

impl ForkPrunedNode {
    /// Returns the removed entry identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns the original insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> HistoryEntrySequence {
        self.sequence
    }

    /// Returns the removed consumer-measured weight.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }
}

/// The part of a graph a budget governs.
///
/// Protected entries are excluded: once protected, an entry is a core record
/// of the project rather than transient history, so the budget bounds what is
/// left. This is also why "the protected lineage alone exceeds the budget"
/// stopped being a condition -- it cannot arise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UnprotectedShare {
    entry_count: usize,
    encoded_weight: u64,
}

/// What one removal pass took, before it is shaped into a receipt.
struct Removal {
    pruned_nodes: Vec<ForkPrunedNode>,
    removed_branches: Vec<ForkBranchId>,
    removed_checkpoints: Vec<ForkCheckpointId>,
    retained_encoded_weight: u64,
}

/// Successful bounded pruning transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkPruningReceipt {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    pruned_nodes: Vec<ForkPrunedNode>,
    removed_branches: Vec<ForkBranchId>,
    removed_checkpoints: Vec<ForkCheckpointId>,
    retained_entry_count: usize,
    retained_encoded_weight: u64,
    unprotected_entry_count: usize,
    unprotected_encoded_weight: u64,
}

impl ForkPruningReceipt {
    /// Returns the source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the committed revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns removed nodes in deterministic leaf-pruning order.
    #[must_use]
    pub fn pruned_nodes(&self) -> &[ForkPrunedNode] {
        &self.pruned_nodes
    }

    /// Returns removed anonymous, unpinned branch references.
    #[must_use]
    pub fn removed_branches(&self) -> &[ForkBranchId] {
        &self.removed_branches
    }

    /// Returns opaque checkpoint refs invalidated by pruning.
    #[must_use]
    pub fn removed_checkpoints(&self) -> &[ForkCheckpointId] {
        &self.removed_checkpoints
    }

    /// Returns final retained node count.
    #[must_use]
    pub const fn retained_entry_count(&self) -> usize {
        self.retained_entry_count
    }

    /// Returns final retained encoded weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }

    /// Returns the entries the budget governs -- everything unprotected.
    ///
    /// Beside the retained totals rather than instead of them, because the two
    /// answer different questions: how large the graph is, and how much of it
    /// a budget can still reach.
    #[must_use]
    pub const fn unprotected_entry_count(&self) -> usize {
        self.unprotected_entry_count
    }

    /// Returns the encoded weight the budget governs.
    #[must_use]
    pub const fn unprotected_encoded_weight(&self) -> u64 {
        self.unprotected_encoded_weight
    }
}

/// Accepted pruning result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkPruningOutcome {
    /// Graph already satisfied the requested budgets.
    Unchanged,
    /// One structural pruning transition committed.
    Pruned(ForkPruningReceipt),
}

impl<P> ForkHistory<P> {
    /// Prunes oldest unprotected leaves until both budgets are met.
    pub fn prune_to(
        &mut self,
        expected_revision: HistoryRevision,
        limits: ForkRetentionLimits,
    ) -> Result<ForkPruningOutcome, ForkRetentionError> {
        if expected_revision != self.revision {
            return Err(ForkRetentionError::StaleRevision {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        let protected = self.protected_lineage()?;
        let unprotected = self.unprotected_share(&protected)?;
        if unprotected.entry_count <= limits.maximum_entries()
            && unprotected.encoded_weight <= limits.maximum_encoded_weight()
        {
            return Ok(ForkPruningOutcome::Unchanged);
        }

        let prune_ids = self.plan_pruning(limits, &protected, unprotected);
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkRetentionError::RevisionOverflow)?;
        // Keep a reference to any surviving part of a branch whose tip is pruned.
        let removed: HashSet<_> = prune_ids.iter().cloned().collect();
        for branch in self.branches.values_mut() {
            let mut head = branch.head_entry_id().cloned();
            while head.as_ref().is_some_and(|id| removed.contains(id)) {
                head = self.nodes[head.as_ref().expect("removed head exists")]
                    .parent_entry_id()
                    .cloned();
            }
            if let Some(head) = head
                && !protected.contains(&head)
            {
                branch.set_head(head);
            }
        }
        let removal = self.remove_in_leaf_order(prune_ids);

        self.revision = committed_revision;
        self.retained_encoded_weight = removal.retained_encoded_weight;
        // Recomputed after the removal, not carried through it: pruning can
        // take a branch with it, which changes what is protected.
        let remaining_share = self.remaining_unprotected_share();
        Ok(ForkPruningOutcome::Pruned(ForkPruningReceipt {
            previous_revision: expected_revision,
            committed_revision,
            pruned_nodes: removal.pruned_nodes,
            removed_branches: removal.removed_branches,
            removed_checkpoints: removal.removed_checkpoints,
            retained_entry_count: self.nodes.len(),
            retained_encoded_weight: removal.retained_encoded_weight,
            unprotected_entry_count: remaining_share.entry_count,
            unprotected_encoded_weight: remaining_share.encoded_weight,
        }))
    }

    /// Deletes one continuation and everything below it. Irreversible.
    ///
    /// Takes the handle `ForkNavigationTarget::CheckoutContinuation` takes --
    /// a fork's first entry -- so the control that switches to a fork and the
    /// control that removes it name the same thing the same way.
    ///
    /// The operator knows things the budget does not: which fork was a
    /// mistake, and which was explored and rejected. `prune_to` cannot act on
    /// that, because it chooses by age against a budget and erodes from the
    /// tip. This chooses by name and takes the whole subtree.
    ///
    /// There is no undo. A delete that could be undone would have to keep what
    /// it deleted, which is the state the operator asked to leave.
    pub fn delete_continuation(
        &mut self,
        expected_revision: HistoryRevision,
        entry_id: &HistoryEntryId,
    ) -> Result<ForkPruningReceipt, ForkRetentionError> {
        if expected_revision != self.revision {
            return Err(ForkRetentionError::StaleRevision {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if !self.nodes.contains_key(entry_id) {
            return Err(ForkRetentionError::UnknownEntry(entry_id.clone()));
        }
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkRetentionError::RevisionOverflow)?;

        // Leaves first, so every node is childless by the time it is removed
        // and the shared removal pass holds.
        let order = self.subtree_leaf_first(entry_id);
        let members: BTreeSet<_> = order.iter().cloned().collect();

        // Standing inside it. Refused rather than combined with a navigation
        // the operator did not ask for.
        if let Some(current) = self.current_node_id.as_ref()
            && members.contains(current)
        {
            return Err(ForkRetentionError::CurrentNodeInSubtree(entry_id.clone()));
        }

        // On the active line, including its futures. The picker never offers
        // this one -- it is the row already shown inline -- so reaching here
        // means a caller asked for something the operator could not have
        // clicked.
        let default = self
            .default_lineage()
            .map_err(|_| ForkRetentionError::InvalidTopology)?;
        if default.iter().any(|candidate| members.contains(candidate)) {
            return Err(ForkRetentionError::EntryOnCurrentPath(entry_id.clone()));
        }

        // Pinned means protect, and it has to mean it against an explicit
        // request as well as against a budget. Unpinning first makes removing
        // a pinned fork two deliberate acts.
        for branch in self.branches.values() {
            if branch.metadata().pinned()
                && branch
                    .head_entry_id()
                    .is_some_and(|head| members.contains(head))
            {
                return Err(ForkRetentionError::PinnedBranchInSubtree(
                    branch.branch_id().clone(),
                ));
            }
        }

        // The current branch survives even when its head is inside: it shrinks
        // back to where the operator stands, which the checks above prove is
        // at or above the subtree. This is what lets the last continuation of
        // the root be deleted -- the branch is left holding no entry, which is
        // where every history starts.
        let current_branch_shrinks = self
            .branches
            .get(&self.current_branch_id)
            .and_then(|branch| branch.head_entry_id())
            .is_some_and(|head| members.contains(head));
        if current_branch_shrinks {
            let position = self.current_node_id.clone();
            let branch = self
                .branches
                .get_mut(&self.current_branch_id)
                .expect("validated graph always has its current branch");
            match position {
                Some(entry) => branch.set_head(entry),
                None => branch.clear_head(),
            }
        }

        let removal = self.remove_in_leaf_order(order);
        self.revision = committed_revision;
        self.retained_encoded_weight = removal.retained_encoded_weight;
        let remaining_share = self.remaining_unprotected_share();
        Ok(ForkPruningReceipt {
            previous_revision: expected_revision,
            committed_revision,
            pruned_nodes: removal.pruned_nodes,
            removed_branches: removal.removed_branches,
            removed_checkpoints: removal.removed_checkpoints,
            retained_entry_count: self.nodes.len(),
            retained_encoded_weight: removal.retained_encoded_weight,
            unprotected_entry_count: remaining_share.entry_count,
            unprotected_encoded_weight: remaining_share.encoded_weight,
        })
    }

    /// One entry and its descendants, deepest first.
    ///
    /// Pre-order reversed: a node's descendants all follow it in pre-order, so
    /// they all precede it here.
    fn subtree_leaf_first(&self, entry_id: &HistoryEntryId) -> Vec<HistoryEntryId> {
        let mut order = Vec::new();
        let mut stack = vec![entry_id.clone()];
        while let Some(current) = stack.pop() {
            stack.extend(self.child_ids(Some(&current)).iter().cloned());
            order.push(current);
        }
        order.reverse();
        order
    }

    /// Removes nodes already ordered so that every node is a leaf by the time
    /// it is reached. Shared by budget pruning and explicit deletion: the two
    /// choose different nodes, but removing one means the same thing either
    /// way -- detach from the parent, repair its preference, and take the
    /// checkpoints and branch heads that pointed here.
    fn remove_in_leaf_order(&mut self, order: Vec<HistoryEntryId>) -> Removal {
        let mut pruned_nodes = Vec::with_capacity(order.len());
        let mut removed_set = HashSet::with_capacity(order.len());
        let mut affected_parents = BTreeSet::new();
        let mut retained_encoded_weight = self.retained_encoded_weight;

        for entry_id in order {
            let node = self
                .nodes
                .remove(&entry_id)
                .expect("pruning plan names retained nodes");
            let parent = node.parent_entry_id().cloned();
            affected_parents.insert(parent);
            self.children.remove(&Some(entry_id.clone()));
            self.preferred_children.remove(&Some(entry_id.clone()));
            retained_encoded_weight = retained_encoded_weight
                .checked_sub(node.encoded_weight())
                .expect("pruned node weight is retained");
            removed_set.insert(entry_id.clone());
            pruned_nodes.push(ForkPrunedNode {
                entry_id,
                sequence: node.sequence(),
                encoded_weight: node.encoded_weight(),
            });
        }

        // Repair each affected parent's child list and preferred child once in O(deg)
        // rather than scanning per removed node.
        for parent in affected_parents {
            let mut removed_preference = false;
            if let Some(preferred) = self.preferred_children.get(&parent)
                && removed_set.contains(preferred)
            {
                removed_preference = true;
            }
            let mut replacement_preference = None;
            if let Some(children) = self.children.get_mut(&parent) {
                children.retain(|child| !removed_set.contains(child));
                if children.is_empty() {
                    self.children.remove(&parent);
                } else if removed_preference {
                    replacement_preference = children.last().cloned();
                }
            }
            if removed_preference {
                if let Some(replacement) = replacement_preference {
                    self.preferred_children.insert(parent, replacement);
                } else {
                    self.preferred_children.remove(&parent);
                }
            }
        }

        let mut removed_checkpoints = Vec::new();
        self.checkpoints.retain(|checkpoint_id, checkpoint| {
            let remove = checkpoint
                .after_entry_id()
                .is_some_and(|entry_id| removed_set.contains(entry_id));
            if remove {
                removed_checkpoints.push(checkpoint_id.clone());
            }
            !remove
        });

        let mut removed_branches = Vec::new();
        self.branches.retain(|branch_id, branch| {
            let remove = branch
                .head_entry_id()
                .is_some_and(|entry_id| removed_set.contains(entry_id));
            if remove {
                removed_branches.push(branch_id.clone());
            }
            !remove
        });

        removed_branches.sort();
        removed_checkpoints.sort();
        Removal {
            pruned_nodes,
            removed_branches,
            removed_checkpoints,
            retained_encoded_weight,
        }
    }

    fn protected_lineage(&self) -> Result<BTreeSet<HistoryEntryId>, ForkRetentionError> {
        let mut protected = BTreeSet::new();
        // Walk each protected head upward only until the first already-protected
        // ancestor. Because protection is ancestor-closed, every node in the
        // graph is visited at most once across all pinned branches (O(N + B)).
        let mut mark_ancestors =
            |start: Option<&HistoryEntryId>| -> Result<(), ForkRetentionError> {
                let mut current = start;
                let mut steps = 0usize;
                while let Some(entry_id) = current {
                    if !protected.insert(entry_id.clone()) {
                        break;
                    }
                    if steps >= self.nodes.len() {
                        return Err(ForkRetentionError::InvalidTopology);
                    }
                    let node = self
                        .nodes
                        .get(entry_id)
                        .ok_or(ForkRetentionError::InvalidTopology)?;
                    current = node.parent_entry_id();
                    steps += 1;
                }
                Ok(())
            };
        // Pinned and current only. A name used to imply protection, from when
        // a name meant an operator had typed one; auto-naming turned that into
        // "protect everything", which is why a fully-named graph could never
        // be pruned. `pinned` is the field that means protect.
        for branch in self.branches.values().filter(|branch| {
            branch.branch_id() == &self.current_branch_id || branch.metadata().pinned()
        }) {
            mark_ancestors(branch.head_entry_id())?;
        }
        mark_ancestors(self.current_node_id.as_ref())?;
        Ok(protected)
    }

    fn plan_pruning(
        &self,
        limits: ForkRetentionLimits,
        protected: &BTreeSet<HistoryEntryId>,
        unprotected: UnprotectedShare,
    ) -> Vec<HistoryEntryId> {
        let mut child_counts: BTreeMap<&HistoryEntryId, usize> = self
            .nodes
            .keys()
            .map(|entry_id| {
                let count = self
                    .children
                    .get(&Some(entry_id.clone()))
                    .map_or(0, Vec::len);
                (entry_id, count)
            })
            .collect();
        let mut leaves: BinaryHeap<Reverse<(HistoryEntrySequence, &HistoryEntryId)>> = self
            .nodes
            .iter()
            .filter(|(entry_id, _)| !protected.contains(*entry_id))
            .filter(|(entry_id, _)| child_counts.get(*entry_id).copied().unwrap_or(0) == 0)
            .map(|(entry_id, node)| Reverse((node.sequence(), entry_id)))
            .collect();

        // The budget bounds the unprotected share, not the graph. A protected
        // entry is a core record of the project, not a transient budgeted
        // thing, so pinning may grow a graph without bound and that is the
        // operator's choice.
        let mut count = unprotected.entry_count;
        let mut weight = unprotected.encoded_weight;
        let mut removals = Vec::new();

        while count > limits.maximum_entries() || weight > limits.maximum_encoded_weight() {
            let Reverse((_, candidate)) = leaves.pop().expect(
                "an unprotected entry always has an unprotected leaf below it: protection \
                 is by lineage, so the protected set is ancestor-closed and no unprotected \
                 entry can have a protected descendant",
            );
            let node = self.nodes.get(candidate).expect("candidate exists");
            count -= 1;
            weight = weight
                .checked_sub(node.encoded_weight())
                .expect("candidate weight is retained");
            if let Some(parent) = node.parent_entry_id()
                && let Some(child_count) = child_counts.get_mut(parent)
            {
                *child_count -= 1;
                if *child_count == 0 && !protected.contains(parent) {
                    let parent_node = self.nodes.get(parent).expect("parent node exists");
                    leaves.push(Reverse((parent_node.sequence(), parent)));
                }
            }
            removals.push(candidate.clone());
        }
        removals
    }

    fn unprotected_share(
        &self,
        protected: &BTreeSet<HistoryEntryId>,
    ) -> Result<UnprotectedShare, ForkRetentionError> {
        let protected_weight = self.protected_weight(protected)?;
        Ok(UnprotectedShare {
            entry_count: self
                .nodes
                .len()
                .checked_sub(protected.len())
                .expect("protected lineage is a subset of retained nodes"),
            encoded_weight: self
                .retained_encoded_weight
                .checked_sub(protected_weight)
                .expect("protected lineage weight cannot exceed retained encoded weight"),
        })
    }

    /// The unprotected share after a removal has already committed.
    ///
    /// Infallible by construction: a refusal here would report a failure for a
    /// change that is already durable. The walks below only fail on topology
    /// that a validated graph cannot hold, and removing whole subtrees keeps it
    /// valid, so an error here is a broken invariant, not an operator outcome.
    fn remaining_unprotected_share(&self) -> UnprotectedShare {
        let protected = self
            .protected_lineage()
            .expect("a committed removal keeps the graph topology valid");
        self.unprotected_share(&protected)
            .expect("a committed removal keeps protected weight within retained weight")
    }

    fn protected_weight(
        &self,
        protected: &BTreeSet<HistoryEntryId>,
    ) -> Result<u64, ForkRetentionError> {
        protected
            .iter()
            .try_fold(0_u64, |total, entry_id| {
                total.checked_add(
                    self.nodes
                        .get(entry_id)
                        .expect("protected node exists")
                        .encoded_weight(),
                )
            })
            .ok_or(ForkRetentionError::WeightOverflow)
    }
}

/// Rejected graph retention operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkRetentionError {
    /// One configured budget was zero.
    ZeroLimit,
    /// Entry budget exceeded its defensive hard limit.
    EntryLimitTooLarge {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
    /// Encoded-weight budget exceeded its defensive hard limit.
    EncodedWeightLimitTooLarge {
        /// Defensive ceiling.
        maximum: u64,
        /// Supplied limit.
        actual: u64,
    },
    /// Request revision was stale.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Deletion named an entry that is not retained.
    UnknownEntry(HistoryEntryId),
    /// Deletion would remove the entry the operator is standing on, or one
    /// above it. That is a delete plus a navigation nobody asked for.
    CurrentNodeInSubtree(HistoryEntryId),
    /// Deletion would remove part of the active line, including its futures.
    EntryOnCurrentPath(HistoryEntryId),
    /// Deletion would remove a pinned branch. Unpin it first.
    PinnedBranchInSubtree(ForkBranchId),
    /// Retained weight overflowed.
    WeightOverflow,
    /// Topology could not produce finite protected lineages.
    InvalidTopology,
    /// Revision could not advance.
    RevisionOverflow,
}

impl fmt::Display for ForkRetentionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit => formatter.write_str("fork retention limit is zero"),
            Self::EntryLimitTooLarge { maximum, actual } => write!(
                formatter,
                "fork retention entry limit {actual} exceeds defensive maximum {maximum}"
            ),
            Self::EncodedWeightLimitTooLarge { maximum, actual } => write!(
                formatter,
                "fork retention encoded-weight limit {actual} exceeds defensive maximum {maximum}"
            ),
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "fork history revision {} is stale; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::UnknownEntry(id) => {
                write!(formatter, "fork history entry {id} is not retained")
            }
            Self::CurrentNodeInSubtree(id) => write!(
                formatter,
                "deletion of fork history entry {id} would remove the current node"
            ),
            Self::EntryOnCurrentPath(id) => write!(
                formatter,
                "deletion of fork history entry {id} would remove part of the active line"
            ),
            Self::PinnedBranchInSubtree(id) => write!(
                formatter,
                "deletion would remove pinned fork branch {id}; unpin it first"
            ),
            Self::WeightOverflow => formatter.write_str("fork retention weight overflowed"),
            Self::InvalidTopology => formatter
                .write_str("fork history topology could not produce finite protected lineages"),
            Self::RevisionOverflow => formatter.write_str("fork history revision cannot advance"),
        }
    }
}

impl Error for ForkRetentionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_id(value: &str) -> HistoryEntryId {
        HistoryEntryId::new(value).expect("fixture entry id")
    }

    #[test]
    fn fork_retention_error_messages_are_hand_written() {
        let cases: [(ForkRetentionError, &str); 11] = [
            (
                ForkRetentionError::ZeroLimit,
                "fork retention limit is zero",
            ),
            (
                ForkRetentionError::EntryLimitTooLarge {
                    maximum: 1_000,
                    actual: 1_001,
                },
                "fork retention entry limit 1001 exceeds defensive maximum 1000",
            ),
            (
                ForkRetentionError::EncodedWeightLimitTooLarge {
                    maximum: 1_000_000,
                    actual: 1_000_001,
                },
                "fork retention encoded-weight limit 1000001 exceeds defensive maximum 1000000",
            ),
            (
                ForkRetentionError::StaleRevision {
                    expected: HistoryRevision::new(2),
                    actual: HistoryRevision::new(5),
                },
                "fork history revision 2 is stale; current revision is 5",
            ),
            (
                ForkRetentionError::UnknownEntry(entry_id("entry:a")),
                "fork history entry entry:a is not retained",
            ),
            (
                ForkRetentionError::CurrentNodeInSubtree(entry_id("entry:b")),
                "deletion of fork history entry entry:b would remove the current node",
            ),
            (
                ForkRetentionError::EntryOnCurrentPath(entry_id("entry:c")),
                "deletion of fork history entry entry:c would remove part of the active line",
            ),
            (
                ForkRetentionError::PinnedBranchInSubtree(
                    ForkBranchId::new("branch:pinned").expect("fixture branch id"),
                ),
                "deletion would remove pinned fork branch branch:pinned; unpin it first",
            ),
            (
                ForkRetentionError::WeightOverflow,
                "fork retention weight overflowed",
            ),
            (
                ForkRetentionError::InvalidTopology,
                "fork history topology could not produce finite protected lineages",
            ),
            (
                ForkRetentionError::RevisionOverflow,
                "fork history revision cannot advance",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
