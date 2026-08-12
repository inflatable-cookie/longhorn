//! Fork-history projection methods.

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
        // Every branch's lineage once, so the pairwise nearest-ancestor search
        // below walks cached vectors rather than the node map. Branch counts
        // are capped by `MAXIMUM_FORK_BRANCHES`, so pairwise is affordable and
        // an index would be state to keep correct for no measured gain.
        let lineages = self
            .branches
            .values()
            .map(|branch| {
                let lineage = self
                    .lineage(branch.head_entry_id())
                    .map_err(|_| ForkProjectionError::InvalidTopology)?;
                Ok((branch.branch_id().clone(), lineage))
            })
            .collect::<Result<Vec<_>, ForkProjectionError>>()?;
        let branches = self
            .branches
            .values()
            .skip(request.offset)
            .take(request.limit)
            .map(|branch| {
                let lineage = self
                    .lineage(branch.head_entry_id())
                    .map_err(|_| ForkProjectionError::InvalidTopology)?;
                // The nearest ancestor branch, relative to the parent run and
                // never to the current branch: computing it against the
                // current branch puts a fork-off-a-fork's divergence on the
                // current path, reporting two structurally different forks at
                // one node.
                //
                // Longest shared prefix alone is not enough, because it is
                // symmetric -- a branch and the branch that forked off it
                // share exactly the same prefix and each looks like the
                // other's nearest. The tiebreak is which one occupied the fork
                // point first: when you fork at an entry, the continuation
                // that was already there was recorded earlier, so the parent
                // is the candidate whose first divergent entry has the lower
                // sequence. A candidate whose lineage is a prefix of ours has
                // no divergent entry at all and is an ancestor outright.
                let first_divergent_sequence = |entries: &[HistoryEntryId], shared: usize| {
                    entries
                        .get(shared)
                        .and_then(|entry_id| self.nodes.get(entry_id))
                        .map(|node| node.sequence())
                };
                let mut best: Option<(usize, Option<HistoryEntrySequence>, &ForkBranchId)> = None;
                for (candidate_id, candidate_lineage) in &lineages {
                    if candidate_id == branch.branch_id() {
                        continue;
                    }
                    let shared = candidate_lineage
                        .iter()
                        .zip(&lineage)
                        .take_while(|(left, right)| left == right)
                        .count();
                    if shared == 0 {
                        continue;
                    }
                    let candidate_first = first_divergent_sequence(candidate_lineage, shared);
                    let own_first = first_divergent_sequence(&lineage, shared);
                    let earlier = match (candidate_first, own_first) {
                        // Candidate is a strict prefix of ours: an ancestor.
                        (None, _) => true,
                        // We are a strict prefix of the candidate, so the
                        // candidate forked off us, not the other way round.
                        (Some(_), None) => false,
                        (Some(candidate), Some(own)) => candidate < own,
                    };
                    if !earlier {
                        continue;
                    }
                    // Deepest first; among equals, whoever got there earliest.
                    // Deterministic, so the answer does not depend on map
                    // iteration order.
                    let better = match best {
                        None => true,
                        Some((depth, _, _)) if shared != depth => shared > depth,
                        Some((_, incumbent, _)) => match (candidate_first, incumbent) {
                            (None, Some(_)) => true,
                            (Some(candidate), Some(incumbent)) => candidate < incumbent,
                            _ => false,
                        },
                    };
                    if better {
                        best = Some((shared, candidate_first, candidate_id));
                    }
                }
                Ok(ForkBranchProjection {
                    branch_id: branch.branch_id().clone(),
                    head_entry_id: branch.head_entry_id().cloned(),
                    divergence_entry_id: best
                        .and_then(|(shared, _, _)| shared.checked_sub(1))
                        .and_then(|index| lineage.get(index))
                        .cloned(),
                    divergence_branch_id: best.map(|(_, _, id)| id.clone()),
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
