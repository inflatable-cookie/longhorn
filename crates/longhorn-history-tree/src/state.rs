use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::{
    HistoryEntryMetadata, HistoryEntrySequence, MAXIMUM_HISTORY_ENCODED_WEIGHT,
};

use crate::{
    ForkBranch, ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkCheckpoint, ForkCheckpointId,
    ForkHistoryError, ForkHistoryNode, ForkHistoryStateError, MAXIMUM_FORK_CHECKPOINTS,
};

/// Defensive hard ceiling for retained fork-history nodes.
pub const MAXIMUM_FORK_NODES: usize = 65_536;
/// Defensive hard ceiling for stable branch references.
pub const MAXIMUM_FORK_BRANCHES: usize = 4_096;

/// One preferred redo edge supplied to structural-state validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkPreferredChild {
    parent_entry_id: Option<HistoryEntryId>,
    child_entry_id: HistoryEntryId,
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
    history_id: HistoryId,
    revision: HistoryRevision,
    nodes: Vec<ForkHistoryNode<P>>,
    branches: Vec<ForkBranch>,
    current_branch_id: ForkBranchId,
    current_node_id: Option<HistoryEntryId>,
    preferred_children: Vec<ForkPreferredChild>,
    checkpoints: Vec<ForkCheckpoint>,
    next_sequence: HistoryEntrySequence,
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

/// One already-applied product mutation offered to the graph authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkRecord<P> {
    expected_revision: HistoryRevision,
    entry_id: HistoryEntryId,
    metadata: HistoryEntryMetadata,
    encoded_weight: u64,
    payload: P,
    divergent_branch: Option<ForkBranchSeed>,
}

impl<P> ForkRecord<P> {
    /// Constructs a record request.
    #[must_use]
    pub const fn new(
        expected_revision: HistoryRevision,
        entry_id: HistoryEntryId,
        metadata: HistoryEntryMetadata,
        encoded_weight: u64,
        payload: P,
        divergent_branch: Option<ForkBranchSeed>,
    ) -> Self {
        Self {
            expected_revision,
            entry_id,
            metadata,
            encoded_weight,
            payload,
            divergent_branch,
        }
    }
}

/// Exact receipt for one successful graph record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkRecordReceipt {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    entry_id: HistoryEntryId,
    branch_id: ForkBranchId,
    parent_entry_id: Option<HistoryEntryId>,
    previous_branch_head: Option<HistoryEntryId>,
    diverged: bool,
    replaced_preferred_child: Option<HistoryEntryId>,
}

impl ForkRecordReceipt {
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

    /// Returns the inserted node identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns the branch advanced or created by the record.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns the immutable parent assigned to the inserted node.
    #[must_use]
    pub const fn parent_entry_id(&self) -> Option<&HistoryEntryId> {
        self.parent_entry_id.as_ref()
    }

    /// Returns the prior head of the branch active before record.
    #[must_use]
    pub const fn previous_branch_head(&self) -> Option<&HistoryEntryId> {
        self.previous_branch_head.as_ref()
    }

    /// Returns whether the record created a new branch reference.
    #[must_use]
    pub const fn diverged(&self) -> bool {
        self.diverged
    }

    /// Returns the former deterministic redo child, when replaced.
    #[must_use]
    pub const fn replaced_preferred_child(&self) -> Option<&HistoryEntryId> {
        self.replaced_preferred_child.as_ref()
    }
}

/// Exact receipt for one branch-metadata replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranchUpdateReceipt {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    branch_id: ForkBranchId,
    previous_metadata: ForkBranchMetadata,
    committed_metadata: ForkBranchMetadata,
}

impl ForkBranchUpdateReceipt {
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

    /// Returns the updated branch identity.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns the replaced metadata.
    #[must_use]
    pub const fn previous_metadata(&self) -> &ForkBranchMetadata {
        &self.previous_metadata
    }

    /// Returns the committed metadata.
    #[must_use]
    pub const fn committed_metadata(&self) -> &ForkBranchMetadata {
        &self.committed_metadata
    }
}

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

fn validate_counts<P>(state: &ForkHistoryState<P>) -> Result<(), ForkHistoryStateError> {
    if state.nodes.len() > MAXIMUM_FORK_NODES {
        return Err(ForkHistoryStateError::TooManyNodes {
            maximum: MAXIMUM_FORK_NODES,
            actual: state.nodes.len(),
        });
    }
    if state.branches.is_empty() {
        return Err(ForkHistoryStateError::MissingBranch);
    }
    if state.branches.len() > MAXIMUM_FORK_BRANCHES {
        return Err(ForkHistoryStateError::TooManyBranches {
            maximum: MAXIMUM_FORK_BRANCHES,
            actual: state.branches.len(),
        });
    }
    if state.checkpoints.len() > MAXIMUM_FORK_CHECKPOINTS {
        return Err(ForkHistoryStateError::TooManyCheckpoints {
            maximum: MAXIMUM_FORK_CHECKPOINTS,
            actual: state.checkpoints.len(),
        });
    }
    Ok(())
}

fn validate_nodes<P>(
    nodes: &BTreeMap<HistoryEntryId, ForkHistoryNode<P>>,
    revision: HistoryRevision,
    next_sequence: HistoryEntrySequence,
) -> Result<(), ForkHistoryStateError> {
    let maximum_sequence = nodes
        .values()
        .map(|node| node.sequence())
        .max()
        .unwrap_or(HistoryEntrySequence::FIRST);
    if !nodes.is_empty() && next_sequence <= maximum_sequence {
        return Err(ForkHistoryStateError::InvalidNextSequence);
    }
    for node in nodes.values() {
        if node.committed_revision() == HistoryRevision::INITIAL
            || node.committed_revision() > revision
        {
            return Err(ForkHistoryStateError::InvalidCommittedRevision(
                node.entry_id().clone(),
            ));
        }
        if let Some(parent_id) = node.parent_entry_id() {
            let Some(parent) = nodes.get(parent_id) else {
                return Err(ForkHistoryStateError::InvalidParent(
                    node.entry_id().clone(),
                ));
            };
            if parent.sequence() >= node.sequence()
                || parent.committed_revision() >= node.committed_revision()
            {
                return Err(ForkHistoryStateError::InvalidParent(
                    node.entry_id().clone(),
                ));
            }
        }
    }
    Ok(())
}

fn build_children<P>(
    nodes: &BTreeMap<HistoryEntryId, ForkHistoryNode<P>>,
) -> BTreeMap<Option<HistoryEntryId>, Vec<HistoryEntryId>> {
    let mut children: BTreeMap<Option<HistoryEntryId>, Vec<HistoryEntryId>> = BTreeMap::new();
    for node in nodes.values() {
        children
            .entry(node.parent_entry_id().cloned())
            .or_default()
            .push(node.entry_id().clone());
    }
    for child_ids in children.values_mut() {
        child_ids.sort_by_key(|entry_id| {
            nodes
                .get(entry_id)
                .expect("child index is derived only from retained nodes")
                .sequence()
        });
    }
    children
}

fn branch_contains<P>(
    nodes: &BTreeMap<HistoryEntryId, ForkHistoryNode<P>>,
    branches: &BTreeMap<ForkBranchId, ForkBranch>,
    branch_id: &ForkBranchId,
    target: Option<&HistoryEntryId>,
) -> bool {
    let Some(branch) = branches.get(branch_id) else {
        return false;
    };
    let mut cursor = branch.head_entry_id().cloned();
    loop {
        if cursor.as_ref() == target {
            return true;
        }
        let Some(entry_id) = cursor else {
            return false;
        };
        cursor = nodes
            .get(&entry_id)
            .and_then(|node| node.parent_entry_id().cloned());
    }
}
