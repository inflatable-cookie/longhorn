use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use longhorn_core::{HistoryEntryId, HistoryRevision};
use longhorn_history::{HistoryEntrySequence, MAXIMUM_HISTORY_ENCODED_WEIGHT};

use crate::{ForkBranchId, ForkCheckpointId, ForkHistory, MAXIMUM_FORK_NODES};

/// Count and exact encoded-weight budgets for graph pruning.
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
        if self.nodes.len() <= limits.maximum_entries()
            && self.retained_encoded_weight <= limits.maximum_encoded_weight()
        {
            return Ok(ForkPruningOutcome::Unchanged);
        }

        let protected = self.protected_lineage()?;
        let prune_ids = self.plan_pruning(limits, &protected)?;
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkRetentionError::RevisionOverflow)?;
        let mut pruned_nodes = Vec::with_capacity(prune_ids.len());
        let mut removed_branches = Vec::new();
        let mut removed_checkpoints = Vec::new();
        let mut retained_encoded_weight = self.retained_encoded_weight;

        for entry_id in prune_ids {
            let node = self
                .nodes
                .remove(&entry_id)
                .expect("pruning plan names retained nodes");
            let parent = node.parent_entry_id().cloned();
            let removed_preference = self.preferred_children.get(&parent) == Some(&entry_id);
            let mut replacement_preference = None;
            if let Some(children) = self.children.get_mut(&parent) {
                children.retain(|child| child != &entry_id);
                if children.is_empty() {
                    self.children.remove(&parent);
                } else if removed_preference {
                    replacement_preference = children.last().cloned();
                }
            }
            self.children.remove(&Some(entry_id.clone()));
            self.preferred_children.remove(&Some(entry_id.clone()));
            self.preferred_children
                .retain(|_, child| child != &entry_id);
            if let Some(replacement) = replacement_preference {
                self.preferred_children.insert(parent, replacement);
            }

            let checkpoint_ids: Vec<_> = self
                .checkpoints
                .values()
                .filter(|checkpoint| checkpoint.after_entry_id() == Some(&entry_id))
                .map(|checkpoint| checkpoint.checkpoint_id().clone())
                .collect();
            for checkpoint_id in checkpoint_ids {
                self.checkpoints.remove(&checkpoint_id);
                removed_checkpoints.push(checkpoint_id);
            }

            let branch_ids: Vec<_> = self
                .branches
                .values()
                .filter(|branch| branch.head_entry_id() == Some(&entry_id))
                .map(|branch| branch.branch_id().clone())
                .collect();
            for branch_id in branch_ids {
                debug_assert_ne!(branch_id, self.current_branch_id);
                self.branches.remove(&branch_id);
                removed_branches.push(branch_id);
            }
            retained_encoded_weight = retained_encoded_weight
                .checked_sub(node.encoded_weight())
                .expect("pruned node weight is retained");
            pruned_nodes.push(ForkPrunedNode {
                entry_id,
                sequence: node.sequence(),
                encoded_weight: node.encoded_weight(),
            });
        }

        self.revision = committed_revision;
        self.retained_encoded_weight = retained_encoded_weight;
        removed_branches.sort();
        removed_checkpoints.sort();
        Ok(ForkPruningOutcome::Pruned(ForkPruningReceipt {
            previous_revision: expected_revision,
            committed_revision,
            pruned_nodes,
            removed_branches,
            removed_checkpoints,
            retained_entry_count: self.nodes.len(),
            retained_encoded_weight,
        }))
    }

    fn protected_lineage(&self) -> Result<BTreeSet<HistoryEntryId>, ForkRetentionError> {
        let mut protected = BTreeSet::new();
        for branch in self.branches.values().filter(|branch| {
            branch.branch_id() == &self.current_branch_id
                || branch.metadata().name().is_some()
                || branch.metadata().pinned()
        }) {
            protected.extend(
                self.lineage(branch.head_entry_id())
                    .map_err(|_| ForkRetentionError::InvalidTopology)?,
            );
        }
        protected.extend(
            self.lineage(self.current_node_id.as_ref())
                .map_err(|_| ForkRetentionError::InvalidTopology)?,
        );
        Ok(protected)
    }

    fn plan_pruning(
        &self,
        limits: ForkRetentionLimits,
        protected: &BTreeSet<HistoryEntryId>,
    ) -> Result<Vec<HistoryEntryId>, ForkRetentionError> {
        let mut remaining: BTreeSet<_> = self.nodes.keys().cloned().collect();
        let mut child_counts: BTreeMap<_, usize> = self
            .nodes
            .keys()
            .map(|entry_id| {
                let count = self
                    .children
                    .get(&Some(entry_id.clone()))
                    .map_or(0, Vec::len);
                (entry_id.clone(), count)
            })
            .collect();
        let mut count = remaining.len();
        let mut weight = self.retained_encoded_weight;
        let mut removals = Vec::new();

        while count > limits.maximum_entries() || weight > limits.maximum_encoded_weight() {
            let candidate = remaining
                .iter()
                .filter(|entry_id| !protected.contains(*entry_id))
                .filter(|entry_id| child_counts.get(*entry_id).copied().unwrap_or(0) == 0)
                .min_by_key(|entry_id| {
                    let node = self.nodes.get(*entry_id).expect("remaining node exists");
                    (node.sequence(), (*entry_id).clone())
                })
                .cloned()
                .ok_or(ForkRetentionError::ProtectedBudget {
                    protected_entries: protected.len(),
                    protected_encoded_weight: self.protected_weight(protected)?,
                })?;
            let node = self.nodes.get(&candidate).expect("candidate exists");
            count -= 1;
            weight = weight
                .checked_sub(node.encoded_weight())
                .expect("candidate weight is retained");
            remaining.remove(&candidate);
            if let Some(parent) = node.parent_entry_id() {
                if let Some(child_count) = child_counts.get_mut(parent) {
                    *child_count -= 1;
                }
            }
            removals.push(candidate);
        }
        Ok(removals)
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
    /// Protected lineage alone exceeds the requested budget.
    ProtectedBudget {
        /// Protected node count.
        protected_entries: usize,
        /// Protected encoded payload weight.
        protected_encoded_weight: u64,
    },
    /// Retained weight overflowed.
    WeightOverflow,
    /// Topology could not produce finite protected lineages.
    InvalidTopology,
    /// Revision could not advance.
    RevisionOverflow,
}

impl fmt::Display for ForkRetentionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "fork retention rejected: {self:?}")
    }
}

impl Error for ForkRetentionError {}
