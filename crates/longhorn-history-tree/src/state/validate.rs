//! Structural validation helpers for fork-history import.

use std::collections::BTreeMap;

use longhorn_core::HistoryEntryId;
use longhorn_history::HistoryEntrySequence;
use longhorn_core::HistoryRevision;

use crate::{
    ForkBranch, ForkBranchId, ForkHistoryNode, ForkHistoryStateError, MAXIMUM_FORK_CHECKPOINTS,
};

use super::{ForkHistoryState, MAXIMUM_FORK_BRANCHES, MAXIMUM_FORK_NODES};

pub(crate) fn validate_counts<P>(state: &ForkHistoryState<P>) -> Result<(), ForkHistoryStateError> {
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

pub(crate) fn validate_nodes<P>(
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

pub(crate) fn build_children<P>(
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

pub(crate) fn branch_contains<P>(
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
