//! Validated fork-history graph authority.

use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::{
    HistoryEntryMetadata, HistoryEntrySequence, MAXIMUM_HISTORY_ENCODED_WEIGHT,
};

use crate::{
    ForkBranch, ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkCheckpoint, ForkCheckpointId,
    ForkHistoryError, ForkHistoryNode, ForkHistoryStateError,
};

use super::{
    ForkBranchUpdateReceipt, ForkHistoryState, ForkPreferredChild, ForkRecord, ForkRecordReceipt,
    MAXIMUM_FORK_BRANCHES, MAXIMUM_FORK_NODES, branch_contains, build_children, validate_counts,
    validate_nodes,
};

/// Immutable-node fork graph with stable branch references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkHistory<P> {
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) nodes: BTreeMap<HistoryEntryId, ForkHistoryNode<P>>,
    pub(crate) children: BTreeMap<Option<HistoryEntryId>, Vec<HistoryEntryId>>,
    pub(crate) branches: BTreeMap<ForkBranchId, ForkBranch>,
    pub(crate) current_branch_id: ForkBranchId,
    pub(crate) current_node_id: Option<HistoryEntryId>,
    pub(crate) preferred_children: BTreeMap<Option<HistoryEntryId>, HistoryEntryId>,
    pub(crate) checkpoints: BTreeMap<ForkCheckpointId, ForkCheckpoint>,
    pub(crate) next_sequence: HistoryEntrySequence,
    pub(crate) retained_encoded_weight: u64,
}

impl<P> ForkHistory<P> {
    /// Constructs an empty graph with one injected main branch.
    #[must_use]
    pub fn new(
        history_id: HistoryId,
        main_branch_id: ForkBranchId,
        main_metadata: ForkBranchMetadata,
    ) -> Self {
        let main_branch = ForkBranch::new(main_branch_id.clone(), None, main_metadata);
        Self {
            history_id,
            revision: HistoryRevision::INITIAL,
            nodes: BTreeMap::new(),
            children: BTreeMap::new(),
            branches: BTreeMap::from([(main_branch_id.clone(), main_branch)]),
            current_branch_id: main_branch_id,
            current_node_id: None,
            preferred_children: BTreeMap::new(),
            checkpoints: BTreeMap::new(),
            next_sequence: HistoryEntrySequence::FIRST,
            retained_encoded_weight: 0,
        }
    }

    /// Validates complete structural state before admitting it as authority.
    pub fn from_state(state: ForkHistoryState<P>) -> Result<Self, ForkHistoryStateError> {
        validate_counts(&state)?;

        let mut nodes = BTreeMap::new();
        let mut sequences = BTreeSet::new();
        let mut revisions = BTreeSet::new();
        let mut retained_encoded_weight = 0_u64;
        for node in state.nodes {
            if !sequences.insert(node.sequence().get()) {
                return Err(ForkHistoryStateError::DuplicateSequence(
                    node.sequence().get(),
                ));
            }
            if !revisions.insert(node.committed_revision().get()) {
                return Err(ForkHistoryStateError::DuplicateCommittedRevision(
                    node.committed_revision().get(),
                ));
            }
            retained_encoded_weight = retained_encoded_weight
                .checked_add(node.encoded_weight())
                .filter(|weight| *weight <= MAXIMUM_HISTORY_ENCODED_WEIGHT)
                .ok_or(ForkHistoryStateError::InvalidEncodedWeight)?;
            let entry_id = node.entry_id().clone();
            if nodes.insert(entry_id.clone(), node).is_some() {
                return Err(ForkHistoryStateError::DuplicateNode(entry_id));
            }
        }
        validate_nodes(&nodes, state.revision, state.next_sequence)?;

        let mut branches = BTreeMap::new();
        for branch in state.branches {
            if branch
                .head_entry_id()
                .is_some_and(|entry_id| !nodes.contains_key(entry_id))
            {
                return Err(ForkHistoryStateError::InvalidBranchHead(
                    branch.branch_id().clone(),
                ));
            }
            let branch_id = branch.branch_id().clone();
            if branches.insert(branch_id.clone(), branch).is_some() {
                return Err(ForkHistoryStateError::DuplicateBranch(branch_id));
            }
        }

        let children = build_children(&nodes);
        let mut preferred_children = BTreeMap::new();
        for preferred in state.preferred_children {
            let child_id = preferred.child_entry_id;
            let direct = nodes
                .get(&child_id)
                .is_some_and(|node| node.parent_entry_id() == preferred.parent_entry_id.as_ref());
            if !direct {
                return Err(ForkHistoryStateError::InvalidPreferredChild(child_id));
            }
            if preferred_children
                .insert(preferred.parent_entry_id, child_id)
                .is_some()
            {
                return Err(ForkHistoryStateError::DuplicatePreferredParent);
            }
        }

        let mut checkpoints = BTreeMap::new();
        for checkpoint in state.checkpoints {
            if checkpoint
                .after_entry_id()
                .is_some_and(|entry_id| !nodes.contains_key(entry_id))
            {
                return Err(ForkHistoryStateError::InvalidCheckpoint(
                    checkpoint.checkpoint_id().clone(),
                ));
            }
            let checkpoint_id = checkpoint.checkpoint_id().clone();
            if checkpoints
                .insert(checkpoint_id.clone(), checkpoint)
                .is_some()
            {
                return Err(ForkHistoryStateError::DuplicateCheckpoint(checkpoint_id));
            }
        }

        if !branches.contains_key(&state.current_branch_id) {
            return Err(ForkHistoryStateError::UnknownCurrentBranch(
                state.current_branch_id,
            ));
        }
        if state
            .current_node_id
            .as_ref()
            .is_some_and(|entry_id| !nodes.contains_key(entry_id))
            || !branch_contains(
                &nodes,
                &branches,
                &state.current_branch_id,
                state.current_node_id.as_ref(),
            )
        {
            return Err(ForkHistoryStateError::InvalidCurrentNode);
        }

        Ok(Self {
            history_id: state.history_id,
            revision: state.revision,
            nodes,
            children,
            branches,
            current_branch_id: state.current_branch_id,
            current_node_id: state.current_node_id,
            preferred_children,
            checkpoints,
            next_sequence: state.next_sequence,
            retained_encoded_weight,
        })
    }

    /// Exports complete structural state without serializing consumer payloads.
    #[must_use]
    pub fn into_state(self) -> ForkHistoryState<P> {
        ForkHistoryState {
            history_id: self.history_id,
            revision: self.revision,
            nodes: self.nodes.into_values().collect(),
            branches: self.branches.into_values().collect(),
            current_branch_id: self.current_branch_id,
            current_node_id: self.current_node_id,
            preferred_children: self
                .preferred_children
                .into_iter()
                .map(|(parent, child)| ForkPreferredChild::new(parent, child))
                .collect(),
            checkpoints: self.checkpoints.into_values().collect(),
            next_sequence: self.next_sequence,
        }
    }

    /// Returns the graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the current structural revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the current applied node, or the graph root.
    #[must_use]
    pub const fn current_node_id(&self) -> Option<&HistoryEntryId> {
        self.current_node_id.as_ref()
    }

    /// Returns the current first-class branch reference.
    #[must_use]
    pub const fn current_branch_id(&self) -> &ForkBranchId {
        &self.current_branch_id
    }

    /// Returns one immutable node.
    #[must_use]
    pub fn node(&self, entry_id: &HistoryEntryId) -> Option<&ForkHistoryNode<P>> {
        self.nodes.get(entry_id)
    }

    /// Returns all nodes in stable identity order.
    pub fn nodes(&self) -> impl Iterator<Item = &ForkHistoryNode<P>> {
        self.nodes.values()
    }

    /// Returns one first-class branch reference.
    #[must_use]
    pub fn branch(&self, branch_id: &ForkBranchId) -> Option<&ForkBranch> {
        self.branches.get(branch_id)
    }

    /// Returns all branches in stable identity order.
    pub fn branches(&self) -> impl Iterator<Item = &ForkBranch> {
        self.branches.values()
    }

    /// Returns canonical direct children in insertion order.
    #[must_use]
    pub fn child_ids(&self, parent_entry_id: Option<&HistoryEntryId>) -> &[HistoryEntryId] {
        self.children
            .get(&parent_entry_id.cloned())
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Returns the deterministic preferred direct child, when present.
    #[must_use]
    pub fn preferred_child_id(
        &self,
        parent_entry_id: Option<&HistoryEntryId>,
    ) -> Option<&HistoryEntryId> {
        self.preferred_children.get(&parent_entry_id.cloned())
    }

    /// Returns retained node count.
    #[must_use]
    pub fn retained_entry_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns retained consumer-measured payload weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }

    /// Records one already-applied mutation without deleting alternate futures.
    pub fn record_applied(
        &mut self,
        record: ForkRecord<P>,
    ) -> Result<ForkRecordReceipt, ForkHistoryError> {
        if record.expected_revision != self.revision {
            return Err(ForkHistoryError::StaleRevision {
                expected: record.expected_revision,
                actual: self.revision,
            });
        }
        if self.nodes.contains_key(&record.entry_id) {
            return Err(ForkHistoryError::DuplicateEntry(record.entry_id));
        }
        if self.nodes.len() >= MAXIMUM_FORK_NODES {
            return Err(ForkHistoryError::NodeLimitReached {
                maximum: MAXIMUM_FORK_NODES,
            });
        }

        let previous_branch_head = self
            .branches
            .get(&self.current_branch_id)
            .expect("validated graph always has its current branch")
            .head_entry_id()
            .cloned();
        let diverged = previous_branch_head != self.current_node_id;
        let new_branch = match (diverged, record.divergent_branch) {
            (true, Some(seed)) => {
                let (branch_id, metadata) = seed.into_parts();
                if self.branches.contains_key(&branch_id) {
                    return Err(ForkHistoryError::DuplicateBranch(branch_id));
                }
                if self.branches.len() >= MAXIMUM_FORK_BRANCHES {
                    return Err(ForkHistoryError::BranchLimitReached {
                        maximum: MAXIMUM_FORK_BRANCHES,
                    });
                }
                Some((branch_id, metadata))
            }
            (true, None) => return Err(ForkHistoryError::DivergentBranchRequired),
            (false, Some(_)) => return Err(ForkHistoryError::UnexpectedDivergentBranch),
            (false, None) => None,
        };

        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkHistoryError::RevisionOverflow)?;
        let next_sequence = self
            .next_sequence
            .checked_next()
            .map_err(|_| ForkHistoryError::SequenceOverflow)?;
        let retained_encoded_weight = self
            .retained_encoded_weight
            .checked_add(record.encoded_weight)
            .filter(|weight| *weight <= MAXIMUM_HISTORY_ENCODED_WEIGHT)
            .ok_or(ForkHistoryError::EncodedWeightLimitExceeded {
                maximum: MAXIMUM_HISTORY_ENCODED_WEIGHT,
                requested: self
                    .retained_encoded_weight
                    .saturating_add(record.encoded_weight),
            })?;

        let branch_id = if let Some((branch_id, metadata)) = new_branch {
            self.branches.insert(
                branch_id.clone(),
                ForkBranch::new(branch_id.clone(), None, metadata),
            );
            self.current_branch_id = branch_id.clone();
            branch_id
        } else {
            self.current_branch_id.clone()
        };
        let parent_entry_id = self.current_node_id.clone();
        let entry_id = record.entry_id.clone();
        let node = ForkHistoryNode::new(
            record.entry_id,
            parent_entry_id.clone(),
            record.metadata,
            self.next_sequence,
            committed_revision,
            record.encoded_weight,
            record.payload,
        );
        self.nodes.insert(entry_id.clone(), node);
        self.children
            .entry(parent_entry_id.clone())
            .or_default()
            .push(entry_id.clone());
        let replaced_preferred_child = self
            .preferred_children
            .insert(parent_entry_id.clone(), entry_id.clone())
            .filter(|previous| previous != &entry_id);
        self.current_node_id = Some(entry_id.clone());
        self.branches
            .get_mut(&branch_id)
            .expect("record branch was registered before mutation")
            .set_head(entry_id.clone());
        self.revision = committed_revision;
        self.next_sequence = next_sequence;
        self.retained_encoded_weight = retained_encoded_weight;

        Ok(ForkRecordReceipt {
            previous_revision: record.expected_revision,
            committed_revision,
            entry_id,
            branch_id,
            parent_entry_id,
            previous_branch_head,
            diverged,
            replaced_preferred_child,
        })
    }

    /// Replaces mutable metadata for one stable branch reference.
    pub fn set_branch_metadata(
        &mut self,
        expected_revision: HistoryRevision,
        branch_id: &ForkBranchId,
        metadata: ForkBranchMetadata,
    ) -> Result<ForkBranchUpdateReceipt, ForkHistoryError> {
        if expected_revision != self.revision {
            return Err(ForkHistoryError::StaleRevision {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        let Some(previous_metadata) = self
            .branches
            .get(branch_id)
            .map(|branch| branch.metadata().clone())
        else {
            return Err(ForkHistoryError::UnknownBranch(branch_id.clone()));
        };
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkHistoryError::RevisionOverflow)?;
        self.branches
            .get_mut(branch_id)
            .expect("branch existence was checked before mutation")
            .set_metadata(metadata.clone());
        self.revision = committed_revision;
        Ok(ForkBranchUpdateReceipt {
            previous_revision: expected_revision,
            committed_revision,
            branch_id: branch_id.clone(),
            previous_metadata,
            committed_metadata: metadata,
        })
    }
}
