use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};
use longhorn_history::{
    HistoryEntryPosition, HistoryEntrySequence, HistoryLabel, MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
};

use crate::{ForkBranchId, ForkHistory};

/// One hard-bounded newest-first projection page request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkProjectionPageRequest {
    offset: usize,
    limit: usize,
}

impl ForkProjectionPageRequest {
    /// Validates one page request against the shared history ceiling.
    pub const fn new(offset: usize, limit: usize) -> Result<Self, ForkProjectionError> {
        if limit == 0 {
            return Err(ForkProjectionError::ZeroPageSize);
        }
        if limit > MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE {
            return Err(ForkProjectionError::PageTooLarge {
                maximum: MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
                actual: limit,
            });
        }
        Ok(Self { offset, limit })
    }

    /// Returns the newest-first offset.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }

    /// Returns the maximum records requested.
    #[must_use]
    pub const fn limit(self) -> usize {
        self.limit
    }
}

/// Payload-free linear-default graph summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkSummary {
    history_id: HistoryId,
    revision: HistoryRevision,
    current_branch_id: ForkBranchId,
    current_entry_id: Option<HistoryEntryId>,
    undo_depth: usize,
    redo_depth: usize,
    next_undo_label: Option<HistoryLabel>,
    next_redo_label: Option<HistoryLabel>,
    retained_entry_count: usize,
    retained_encoded_weight: u64,
    branch_count: usize,
    alternate_path_count: usize,
}

impl ForkSummary {
    /// Returns the graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the exact graph revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the selected first-class branch.
    #[must_use]
    pub const fn current_branch_id(&self) -> &ForkBranchId {
        &self.current_branch_id
    }

    /// Returns the currently applied entry, or root.
    #[must_use]
    pub const fn current_entry_id(&self) -> Option<&HistoryEntryId> {
        self.current_entry_id.as_ref()
    }

    /// Returns applied depth on the current lineage.
    #[must_use]
    pub const fn undo_depth(&self) -> usize {
        self.undo_depth
    }

    /// Returns preferred future depth on the linear-default path.
    #[must_use]
    pub const fn redo_depth(&self) -> usize {
        self.redo_depth
    }

    /// Returns the next consumer-owned undo label.
    #[must_use]
    pub const fn next_undo_label(&self) -> Option<&HistoryLabel> {
        self.next_undo_label.as_ref()
    }

    /// Returns the next consumer-owned preferred-redo label.
    #[must_use]
    pub const fn next_redo_label(&self) -> Option<&HistoryLabel> {
        self.next_redo_label.as_ref()
    }

    /// Returns retained node count.
    #[must_use]
    pub const fn retained_entry_count(&self) -> usize {
        self.retained_entry_count
    }

    /// Returns retained consumer-measured payload weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }

    /// Returns first-class branch-reference count without projecting them.
    #[must_use]
    pub const fn branch_count(&self) -> usize {
        self.branch_count
    }

    /// Returns derived leaf-path count without projecting any lineage.
    #[must_use]
    pub const fn alternate_path_count(&self) -> usize {
        self.alternate_path_count
    }
}

/// One payload-free entry on a requested graph path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkEntryProjection {
    entry_id: HistoryEntryId,
    label: HistoryLabel,
    kind_id: Option<HistoryKindId>,
    group_id: Option<HistoryGroupId>,
    sequence: HistoryEntrySequence,
    committed_revision: HistoryRevision,
    encoded_weight: u64,
    position: HistoryEntryPosition,
}

impl ForkEntryProjection {
    /// Returns stable entry identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns consumer-owned presentation text.
    #[must_use]
    pub const fn label(&self) -> &HistoryLabel {
        &self.label
    }

    /// Returns optional consumer-owned kind identity.
    #[must_use]
    pub const fn kind_id(&self) -> Option<&HistoryKindId> {
        self.kind_id.as_ref()
    }

    /// Returns optional consumer-owned group identity.
    #[must_use]
    pub const fn group_id(&self) -> Option<&HistoryGroupId> {
        self.group_id.as_ref()
    }

    /// Returns monotonic insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> HistoryEntrySequence {
        self.sequence
    }

    /// Returns the revision that committed the node.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns consumer-measured payload weight, never payload bytes.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }

    /// Returns current authoritative position relative to the applied lineage.
    #[must_use]
    pub const fn position(&self) -> HistoryEntryPosition {
        self.position
    }
}

/// One bounded path page with no stable path identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkPathPage {
    history_id: HistoryId,
    revision: HistoryRevision,
    branch_id: Option<ForkBranchId>,
    head_entry_id: Option<HistoryEntryId>,
    offset: usize,
    total_entries: usize,
    entries: Vec<ForkEntryProjection>,
    truncated_before: bool,
    truncated_after: bool,
}

impl ForkPathPage {
    /// Returns graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns exact projected revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the stable branch used for an explicit path, or `None` for the
    /// preferred linear-default path.
    #[must_use]
    pub const fn branch_id(&self) -> Option<&ForkBranchId> {
        self.branch_id.as_ref()
    }

    /// Returns the projected path head, or root.
    #[must_use]
    pub const fn head_entry_id(&self) -> Option<&HistoryEntryId> {
        self.head_entry_id.as_ref()
    }

    /// Returns newest-first offset.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns full path length without duplicating its lineage.
    #[must_use]
    pub const fn total_entries(&self) -> usize {
        self.total_entries
    }

    /// Returns bounded payload-free records.
    #[must_use]
    pub fn entries(&self) -> &[ForkEntryProjection] {
        &self.entries
    }

    /// Returns whether newer path records precede this page.
    #[must_use]
    pub const fn truncated_before(&self) -> bool {
        self.truncated_before
    }

    /// Returns whether older path records follow this page.
    #[must_use]
    pub const fn truncated_after(&self) -> bool {
        self.truncated_after
    }
}

/// Optional payload-free metadata for one first-class branch reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranchProjection {
    branch_id: ForkBranchId,
    head_entry_id: Option<HistoryEntryId>,
    divergence_entry_id: Option<HistoryEntryId>,
    name: Option<String>,
    annotation: Option<String>,
    pinned: bool,
    current: bool,
}

impl ForkBranchProjection {
    /// Returns stable branch identity.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns branch head or root.
    #[must_use]
    pub const fn head_entry_id(&self) -> Option<&HistoryEntryId> {
        self.head_entry_id.as_ref()
    }

    /// Returns the shared divergence ancestor relative to the current branch.
    #[must_use]
    pub const fn divergence_entry_id(&self) -> Option<&HistoryEntryId> {
        self.divergence_entry_id.as_ref()
    }

    /// Returns optional branch name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns optional branch annotation.
    #[must_use]
    pub fn annotation(&self) -> Option<&str> {
        self.annotation.as_deref()
    }

    /// Returns whether retention protects the branch explicitly.
    #[must_use]
    pub const fn pinned(&self) -> bool {
        self.pinned
    }

    /// Returns whether this is the selected branch.
    #[must_use]
    pub const fn current(&self) -> bool {
        self.current
    }
}

/// One bounded stable branch-reference page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranchPage {
    history_id: HistoryId,
    revision: HistoryRevision,
    offset: usize,
    total_branches: usize,
    branches: Vec<ForkBranchProjection>,
    truncated_before: bool,
    truncated_after: bool,
}

impl ForkBranchPage {
    /// Returns graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns exact projected revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns stable-id ordered offset.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns total branch-reference count.
    #[must_use]
    pub const fn total_branches(&self) -> usize {
        self.total_branches
    }

    /// Returns bounded branch metadata.
    #[must_use]
    pub fn branches(&self) -> &[ForkBranchProjection] {
        &self.branches
    }

    /// Returns whether earlier stable ids precede this page.
    #[must_use]
    pub const fn truncated_before(&self) -> bool {
        self.truncated_before
    }

    /// Returns whether later stable ids follow this page.
    #[must_use]
    pub const fn truncated_after(&self) -> bool {
        self.truncated_after
    }
}

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

    fn default_lineage(&self) -> Result<Vec<HistoryEntryId>, ForkProjectionError> {
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

    fn project_lineage_page(
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

fn check_offset(offset: usize, maximum: usize) -> Result<(), ForkProjectionError> {
    if offset > maximum {
        return Err(ForkProjectionError::OffsetOutOfRange {
            maximum,
            actual: offset,
        });
    }
    Ok(())
}

/// Rejected bounded graph projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkProjectionError {
    /// Requested page size was zero.
    ZeroPageSize,
    /// Requested page exceeded the shared hard ceiling.
    PageTooLarge {
        /// Shared hard maximum.
        maximum: usize,
        /// Supplied size.
        actual: usize,
    },
    /// Requested offset exceeded the selected collection.
    OffsetOutOfRange {
        /// Maximum accepted offset.
        maximum: usize,
        /// Supplied offset.
        actual: usize,
    },
    /// Explicit path named no branch reference.
    UnknownBranch(ForkBranchId),
    /// Validated topology could not be projected consistently.
    InvalidTopology,
}

impl fmt::Display for ForkProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "fork projection failed: {self:?}")
    }
}

impl Error for ForkProjectionError {}
