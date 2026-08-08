//! Fork-history projection methods.

use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};
use longhorn_history::{HistoryEntryPosition, HistoryEntrySequence, HistoryLabel};

use crate::{ForkBranchId, ForkHistory};

use super::{
    ForkBranchPage, ForkBranchProjection, ForkEntryProjection, ForkPathPage, ForkProjectionError,
    ForkProjectionPageRequest, ForkSummary, MAXIMUM_FORK_PROJECTION_PAGE_SIZE, check_offset,
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
        let current_branch = self
            .branches
            .get(&self.current_branch_id)
            .ok_or(ForkProjectionError::InvalidTopology)?;
        let current_lineage = self
            .lineage(current_branch.head_entry_id())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        let branches = self
            .branches
            .values()
            .skip(request.offset)
            .take(request.limit)
            .map(|branch| {
                let lineage = self
                    .lineage(branch.head_entry_id())
                    .map_err(|_| ForkProjectionError::InvalidTopology)?;
                let shared = current_lineage
                    .iter()
                    .zip(&lineage)
                    .take_while(|(left, right)| left == right)
                    .count();
                Ok(ForkBranchProjection {
                    branch_id: branch.branch_id().clone(),
                    head_entry_id: branch.head_entry_id().cloned(),
                    divergence_entry_id: shared
                        .checked_sub(1)
                        .and_then(|index| lineage.get(index))
                        .cloned(),
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

    pub(crate) fn default_lineage(&self) -> Result<Vec<HistoryEntryId>, ForkProjectionError> {
        let mut lineage = self
            .lineage(self.current_node_id.as_ref())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        let mut parent = self.current_node_id.clone();
        while let Some(child) = self.preferred_children.get(&parent) {
            lineage.push(child.clone());
            parent = Some(child.clone());
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
                    group_id: node.metadata().group_id().cloned(),
                    sequence: node.sequence(),
                    committed_revision: node.committed_revision(),
                    encoded_weight: node.encoded_weight(),
                    position,
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
            truncated_before: request.offset != 0,
            truncated_after: page_end < lineage.len(),
        })
    }
}
