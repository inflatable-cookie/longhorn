use std::collections::BTreeMap;

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::{HistoryEntryMetadata, HistoryEntrySequence};

use crate::{
    ForkBranch, ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkCheckpoint, ForkCheckpointId,
    ForkHistoryNode,
};

/// One already-applied product mutation offered to the private graph.
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
    /// Constructs a private graph record request.
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

/// Successful private graph record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkRecordReceipt {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    entry_id: HistoryEntryId,
    branch_id: ForkBranchId,
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

/// Private immutable-node fork graph.
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
}

impl<P> ForkHistory<P> {
    /// Constructs an empty graph with one injected main branch.
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

    /// Returns one first-class branch reference.
    #[must_use]
    pub fn branch(&self, branch_id: &ForkBranchId) -> Option<&ForkBranch> {
        self.branches.get(branch_id)
    }

    /// Returns all branches in stable identity order.
    pub fn branches(&self) -> impl Iterator<Item = &ForkBranch> {
        self.branches.values()
    }

    /// Returns retained node count.
    #[must_use]
    pub fn retained_entry_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns retained consumer-measured payload weight.
    #[must_use]
    pub fn retained_encoded_weight(&self) -> u64 {
        self.nodes
            .values()
            .map(ForkHistoryNode::encoded_weight)
            .sum()
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
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkHistoryError::RevisionOverflow)?;
        let next_sequence = self
            .next_sequence
            .checked_next()
            .map_err(|_| ForkHistoryError::SequenceOverflow)?;

        let current_head = self
            .branches
            .get(&self.current_branch_id)
            .expect("current branch is always registered")
            .head_entry_id()
            .cloned();
        let diverged = current_head != self.current_node_id;
        let branch_id = if diverged {
            let seed = record
                .divergent_branch
                .ok_or(ForkHistoryError::DivergentBranchRequired)?;
            let (branch_id, metadata) = seed.into_parts();
            if self.branches.contains_key(&branch_id) {
                return Err(ForkHistoryError::DuplicateBranch(branch_id));
            }
            self.branches.insert(
                branch_id.clone(),
                ForkBranch::new(branch_id.clone(), None, metadata),
            );
            self.current_branch_id = branch_id.clone();
            branch_id
        } else {
            if record.divergent_branch.is_some() {
                return Err(ForkHistoryError::UnexpectedDivergentBranch);
            }
            self.current_branch_id.clone()
        };

        let parent = self.current_node_id.clone();
        let entry_id = record.entry_id.clone();
        let node = ForkHistoryNode::new(
            record.entry_id,
            parent.clone(),
            record.metadata,
            self.next_sequence,
            committed_revision,
            record.encoded_weight,
            record.payload,
        );
        self.nodes.insert(entry_id.clone(), node);
        self.children
            .entry(parent.clone())
            .or_default()
            .push(entry_id.clone());
        let replaced_preferred_child = self
            .preferred_children
            .insert(parent, entry_id.clone())
            .filter(|previous| previous != &entry_id);
        self.current_node_id = Some(entry_id.clone());
        self.branches
            .get_mut(&branch_id)
            .expect("record branch was registered")
            .set_head(Some(entry_id.clone()));
        self.revision = committed_revision;
        self.next_sequence = next_sequence;

        Ok(ForkRecordReceipt {
            previous_revision: record.expected_revision,
            committed_revision,
            entry_id,
            branch_id,
            diverged,
            replaced_preferred_child,
        })
    }

    /// Replaces mutable metadata for one branch reference.
    pub fn set_branch_metadata(
        &mut self,
        expected_revision: HistoryRevision,
        branch_id: &ForkBranchId,
        metadata: ForkBranchMetadata,
    ) -> Result<HistoryRevision, ForkHistoryError> {
        if expected_revision != self.revision {
            return Err(ForkHistoryError::StaleRevision {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkHistoryError::RevisionOverflow)?;
        let branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| ForkHistoryError::UnknownBranch(branch_id.clone()))?;
        branch.set_metadata(metadata);
        self.revision = committed_revision;
        Ok(committed_revision)
    }

    pub(crate) fn branch_contains(
        &self,
        branch_id: &ForkBranchId,
        target: Option<&HistoryEntryId>,
    ) -> bool {
        let Some(branch) = self.branches.get(branch_id) else {
            return false;
        };
        let target = target.cloned();
        let mut cursor = branch.head_entry_id().cloned();
        loop {
            if cursor == target {
                return true;
            }
            let Some(entry_id) = cursor else {
                return false;
            };
            cursor = self
                .nodes
                .get(&entry_id)
                .and_then(|node| node.parent_entry_id().cloned());
        }
    }
}

/// Rejected private graph transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkHistoryError {
    /// Request revision was stale.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Entry identity already exists.
    DuplicateEntry(HistoryEntryId),
    /// Branch identity already exists.
    DuplicateBranch(ForkBranchId),
    /// Divergent record omitted its new stable branch identity.
    DivergentBranchRequired,
    /// Non-divergent record supplied an unnecessary branch identity.
    UnexpectedDivergentBranch,
    /// Branch identity does not exist.
    UnknownBranch(ForkBranchId),
    /// Revision could not advance.
    RevisionOverflow,
    /// Entry sequence could not advance.
    SequenceOverflow,
}
