//! Fork-history projection methods.

use std::collections::HashMap;

use longhorn_core::HistoryEntryId;
use longhorn_history::{HistoryEntryPosition, HistoryEntrySequence};

use crate::{ForkBranchId, ForkHistory};

use super::{
    ForkBranchPage, ForkBranchProjection, ForkContinuation, ForkContinuationPage,
    ForkEntryProjection, ForkPathPage, ForkProjectionError, ForkProjectionPageRequest, ForkSummary,
    check_offset,
};

impl<P> ForkHistory<P> {
    /// Projects the linear default without materializing branch or path lists.
    pub fn project_summary(&self) -> Result<ForkSummary, ForkProjectionError> {
        let applied = self
            .lineage(self.current_node_id.as_ref())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        let default = self.default_lineage()?;
        let next_redo = default.get(applied.len());
        Ok(ForkSummary {
            history_id: self.history_id.clone(),
            revision: self.revision,
            current_branch_id: self.current_branch_id.clone(),
            current_entry_id: self.current_node_id.clone(),
            undo_depth: applied.len(),
            redo_depth: default.len().saturating_sub(applied.len()),
            next_undo_label: applied
                .last()
                .and_then(|entry_id| self.nodes.get(entry_id))
                .map(|node| node.metadata().label().clone()),
            next_redo_label: next_redo
                .and_then(|entry_id| self.nodes.get(entry_id))
                .map(|node| node.metadata().label().clone()),
            retained_entry_count: self.nodes.len(),
            retained_encoded_weight: self.retained_encoded_weight,
            branch_count: self.branches.len(),
            alternate_path_count: self
                .nodes
                .values()
                .filter(|node| self.child_ids(Some(node.entry_id())).is_empty())
                .count(),
        })
    }

    /// Projects one bounded page from the preferred linear-default path.
    pub fn project_default_path_page(
        &self,
        request: ForkProjectionPageRequest,
    ) -> Result<ForkPathPage, ForkProjectionError> {
        let lineage = self.default_lineage()?;
        self.project_lineage_page(None, lineage, request)
    }

    /// Projects one explicit branch path only after the caller selects it.
    pub fn project_branch_path_page(
        &self,
        branch_id: &ForkBranchId,
        request: ForkProjectionPageRequest,
    ) -> Result<ForkPathPage, ForkProjectionError> {
        let branch = self
            .branches
            .get(branch_id)
            .ok_or_else(|| ForkProjectionError::UnknownBranch(branch_id.clone()))?;
        let lineage = self
            .lineage(branch.head_entry_id())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        self.project_lineage_page(Some(branch_id.clone()), lineage, request)
    }

    /// Projects one bounded stable-id ordered branch page.
    pub fn project_branch_page(
        &self,
        request: ForkProjectionPageRequest,
    ) -> Result<ForkBranchPage, ForkProjectionError> {
        check_offset(request.offset, self.branches.len())?;

        // Build two compact O(N + B) indices without materializing full branch
        // lineages:
        // 1. `heads_at`: up to the two smallest branch ids whose head sits on a
        //    given node (so a branch can always find another branch at the same
        //    node in O(1)).
        // 2. `min_branch_in_subtree`: the smallest branch id (in `BTreeMap`
        //    order) whose head lies in each node's subtree. Because we walk
        //    `self.branches` in ascending id order and stop at the first
        //    already-visited node, each node in the graph is visited at most
        //    once.
        let mut heads_at: HashMap<&HistoryEntryId, (&ForkBranchId, Option<&ForkBranchId>)> =
            HashMap::with_capacity(self.branches.len());
        let mut min_branch_in_subtree: HashMap<&HistoryEntryId, &ForkBranchId> = HashMap::new();
        for (branch_id, branch) in &self.branches {
            let Some(head) = branch.head_entry_id() else {
                continue;
            };
            heads_at
                .entry(head)
                .and_modify(|(_, second)| {
                    if second.is_none() {
                        *second = Some(branch_id);
                    }
                })
                .or_insert((branch_id, None));

            let mut current = Some(head);
            let mut steps = 0usize;
            while let Some(node_id) = current {
                if min_branch_in_subtree.contains_key(node_id) {
                    break;
                }
                if steps > self.nodes.len() {
                    return Err(ForkProjectionError::InvalidTopology);
                }
                let node = self
                    .nodes
                    .get(node_id)
                    .ok_or(ForkProjectionError::InvalidTopology)?;
                min_branch_in_subtree.insert(node_id, branch_id);
                current = node.parent_entry_id();
                steps += 1;
            }
        }

        type EarliestBranchChild<'a> = (HistoryEntrySequence, &'a HistoryEntryId, &'a ForkBranchId);
        // For any parent node with multiple branch-bearing children, cache the
        // earliest-sequence branch-bearing child (and second-earliest, in case
        // the earliest is our own child) so each page branch resolves a fork
        // point in O(1) even on high-fanout anchors.
        let mut earliest_child_cache: HashMap<
            &HistoryEntryId,
            (
                Option<EarliestBranchChild<'_>>,
                Option<EarliestBranchChild<'_>>,
            ),
        > = HashMap::new();

        let branches = self
            .branches
            .values()
            .skip(request.offset)
            .take(request.limit)
            .map(|branch| {
                // Walk bottom-up from the branch head toward the root. The
                // first eligible ancestor branch found is guaranteed to have
                // the deepest shared prefix; at that node, a branch whose head
                // is the node itself (no divergent entry) beats any sibling
                // continuation, and among sibling continuations the lowest
                // first-divergent sequence wins.
                let mut divergence: Option<(&HistoryEntryId, &ForkBranchId)> = None;
                let mut current = branch.head_entry_id();
                let mut own_child: Option<(&HistoryEntryId, HistoryEntrySequence)> = None;
                let mut steps = 0usize;

                while let Some(node_id) = current {
                    if steps > self.nodes.len() {
                        return Err(ForkProjectionError::InvalidTopology);
                    }
                    let node = self
                        .nodes
                        .get(node_id)
                        .ok_or(ForkProjectionError::InvalidTopology)?;

                    // Case 1: another branch's head is `node_id` itself
                    // (strict prefix of or identical to our lineage).
                    if let Some((first, second)) = heads_at.get(node_id) {
                        let other = if *first != branch.branch_id() {
                            Some(*first)
                        } else {
                            *second
                        };
                        if let Some(ancestor_id) = other {
                            divergence = Some((node_id, ancestor_id));
                            break;
                        }
                    }

                    // Case 2: a sibling continuation with a strictly earlier
                    // first-divergent sequence has a branch head in its subtree.
                    if let Some((own_child_id, own_seq)) = own_child {
                        let (first, second) =
                            *earliest_child_cache.entry(node_id).or_insert_with(|| {
                                let mut best_one: Option<(
                                    HistoryEntrySequence,
                                    &HistoryEntryId,
                                    &ForkBranchId,
                                )> = None;
                                let mut best_two: Option<(
                                    HistoryEntrySequence,
                                    &HistoryEntryId,
                                    &ForkBranchId,
                                )> = None;
                                for child_id in self.child_ids(Some(node_id)) {
                                    let Some(&branch_in_sub) = min_branch_in_subtree.get(child_id)
                                    else {
                                        continue;
                                    };
                                    let Some(child_node) = self.nodes.get(child_id) else {
                                        continue;
                                    };
                                    let candidate =
                                        (child_node.sequence(), child_id, branch_in_sub);
                                    if best_one.is_none_or(|b| candidate.0 < b.0) {
                                        best_two = best_one;
                                        best_one = Some(candidate);
                                    } else if best_two.is_none_or(|b| candidate.0 < b.0) {
                                        best_two = Some(candidate);
                                    }
                                }
                                (best_one, best_two)
                            });
                        let sibling = first
                            .filter(|(_, child_id, _)| *child_id != own_child_id)
                            .or_else(|| {
                                second.filter(|(_, child_id, _)| *child_id != own_child_id)
                            });
                        if let Some((cand_seq, _, ancestor_id)) = sibling
                            && cand_seq < own_seq
                        {
                            divergence = Some((node_id, ancestor_id));
                            break;
                        }
                    }

                    own_child = Some((node_id, node.sequence()));
                    current = node.parent_entry_id();
                    steps += 1;
                }

                Ok(ForkBranchProjection {
                    branch_id: branch.branch_id().clone(),
                    head_entry_id: branch.head_entry_id().cloned(),
                    divergence_entry_id: divergence.map(|(entry_id, _)| entry_id.clone()),
                    divergence_branch_id: divergence.map(|(_, branch_id)| branch_id.clone()),
                    name: branch.metadata().name().map(str::to_owned),
                    annotation: branch.metadata().annotation().map(str::to_owned),
                    pinned: branch.metadata().pinned(),
                    current: branch.branch_id() == &self.current_branch_id,
                })
            })
            .collect::<Result<Vec<_>, ForkProjectionError>>()?;
        let page_end = request.offset + branches.len();
        Ok(ForkBranchPage {
            history_id: self.history_id.clone(),
            revision: self.revision,
            offset: request.offset,
            total_branches: self.branches.len(),
            branches,
            truncated_before: request.offset != 0,
            truncated_after: page_end < self.branches.len(),
        })
    }

    /// Projects one bounded page of the continuations at an anchor.
    ///
    /// `anchor` is the entry they continue from, or `None` for the root. Every
    /// child appears, including the one the caller already renders inline --
    /// the projection is not told which that is, so this page can never
    /// disagree with `continuation_count`.
    pub fn project_continuations(
        &self,
        anchor: Option<&HistoryEntryId>,
        request: ForkProjectionPageRequest,
    ) -> Result<ForkContinuationPage, ForkProjectionError> {
        // An empty page here would read as "this entry has no forks", which is
        // a different fact from "there is no such entry".
        if let Some(entry_id) = anchor
            && !self.nodes.contains_key(entry_id)
        {
            return Err(ForkProjectionError::UnknownEntry(entry_id.clone()));
        }
        let children = self.child_ids(anchor);
        check_offset(request.offset, children.len())?;
        let preferred = self.preferred_child_id(anchor);
        let continuations = children
            .iter()
            .skip(request.offset)
            .take(request.limit)
            .map(|entry_id| {
                let node = self
                    .nodes
                    .get(entry_id)
                    .ok_or(ForkProjectionError::InvalidTopology)?;
                let run = self.continuation_run(entry_id);
                let leaf = run.last().unwrap_or(entry_id);
                let branch = self
                    .branches
                    .values()
                    .find(|branch| branch.head_entry_id() == Some(leaf))
                    .ok_or(ForkProjectionError::InvalidTopology)?;
                Ok(ForkContinuation {
                    entry_id: entry_id.clone(),
                    label: node.metadata().label().clone(),
                    recorded_at: node.metadata().recorded_at(),
                    preferred: preferred == Some(entry_id),
                    entry_count: run.len(),
                    branch_id: branch.branch_id().clone(),
                    branch_name: branch.metadata().name().map(str::to_owned),
                })
            })
            .collect::<Result<Vec<_>, ForkProjectionError>>()?;
        let page_end = request.offset + continuations.len();
        Ok(ForkContinuationPage {
            history_id: self.history_id.clone(),
            revision: self.revision,
            anchor_entry_id: anchor.cloned(),
            offset: request.offset,
            total_continuations: children.len(),
            continuations,
            truncated_before: request.offset != 0,
            truncated_after: page_end < children.len(),
        })
    }

    /// Projects the flat run beginning at one entry.
    ///
    /// Structurally identical to a default path page -- same type, same
    /// positions, same counts, same truncation. That identity is deliberate:
    /// a renderer showing the run under an opened fork is running the same
    /// code it runs at the top level.
    pub fn project_continuation_run_page(
        &self,
        from_entry_id: &HistoryEntryId,
        request: ForkProjectionPageRequest,
    ) -> Result<ForkPathPage, ForkProjectionError> {
        if !self.nodes.contains_key(from_entry_id) {
            return Err(ForkProjectionError::UnknownEntry(from_entry_id.clone()));
        }
        self.project_lineage_page(None, self.continuation_run(from_entry_id), request)
    }

    /// The run starting at one entry: itself, then preferred children to the
    /// leaf. Terminates because a preferred child is recorded for every node
    /// that has any child, so the walk only stops at a childless node.
    pub(crate) fn continuation_run(&self, from_entry_id: &HistoryEntryId) -> Vec<HistoryEntryId> {
        let mut run = vec![from_entry_id.clone()];
        let mut parent = Some(from_entry_id.clone());
        while let Some(child) = self.preferred_child_id(parent.as_ref()) {
            let child = child.clone();
            run.push(child.clone());
            parent = Some(child);
        }
        run
    }

    pub(crate) fn default_lineage(&self) -> Result<Vec<HistoryEntryId>, ForkProjectionError> {
        let mut lineage = self
            .lineage(self.current_node_id.as_ref())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        let mut parent = self.current_node_id.clone();
        while let Some(child) = self.preferred_child_id(parent.as_ref()) {
            let child = child.clone();
            lineage.push(child.clone());
            parent = Some(child);
        }
        Ok(lineage)
    }

    pub(crate) fn project_lineage_page(
        &self,
        branch_id: Option<ForkBranchId>,
        lineage: Vec<HistoryEntryId>,
        request: ForkProjectionPageRequest,
    ) -> Result<ForkPathPage, ForkProjectionError> {
        check_offset(request.offset, lineage.len())?;
        let preceding_anchor = lineage
            .first()
            .and_then(|entry_id| self.nodes.get(entry_id))
            .and_then(|node| node.parent_entry_id().cloned());
        let applied = self
            .lineage(self.current_node_id.as_ref())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        let entries = lineage
            .iter()
            .rev()
            .skip(request.offset)
            .take(request.limit)
            .map(|entry_id| {
                let node = self
                    .nodes
                    .get(entry_id)
                    .ok_or(ForkProjectionError::InvalidTopology)?;
                let position = if self.current_node_id.as_ref() == Some(entry_id) {
                    HistoryEntryPosition::Current
                } else if applied.contains(entry_id) {
                    HistoryEntryPosition::Past
                } else {
                    HistoryEntryPosition::Future
                };
                Ok(ForkEntryProjection {
                    entry_id: entry_id.clone(),
                    label: node.metadata().label().clone(),
                    kind_id: node.metadata().kind_id().cloned(),
                    recorded_at: node.metadata().recorded_at(),
                    group_id: node.metadata().group_id().cloned(),
                    sequence: node.sequence(),
                    committed_revision: node.committed_revision(),
                    encoded_weight: node.encoded_weight(),
                    position,
                    continuation_count: self.child_ids(Some(entry_id)).len(),
                })
            })
            .collect::<Result<Vec<_>, ForkProjectionError>>()?;
        let page_end = request.offset + entries.len();
        Ok(ForkPathPage {
            history_id: self.history_id.clone(),
            revision: self.revision,
            branch_id,
            head_entry_id: lineage.last().cloned(),
            offset: request.offset,
            total_entries: lineage.len(),
            entries,
            // The position immediately above this run, not the history root.
            // For a lineage that starts at a root those are the same thing;
            // for a continuation run they are not, and reporting the root's
            // children on a run page is a fact about a different position.
            preceding_continuation_count: self.child_ids(preceding_anchor.as_ref()).len(),
            preceding_entry_id: preceding_anchor,
            truncated_before: request.offset != 0,
            truncated_after: page_end < lineage.len(),
        })
    }
}
