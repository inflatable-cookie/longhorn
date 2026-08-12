//! Fork-history projection view types.

use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};
use longhorn_history::{
    HistoryEntryPosition, HistoryEntrySequence, HistoryLabel, HistoryRecordedAt,
};

use crate::ForkBranchId;

use super::ForkProjectionError;

/// Hard ceiling for one fork-tree metadata page.
pub const MAXIMUM_FORK_PROJECTION_PAGE_SIZE: usize = 256;

/// One hard-bounded newest-first projection page request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkProjectionPageRequest {
    pub(crate) offset: usize,
    pub(crate) limit: usize,
}

impl ForkProjectionPageRequest {
    /// Validates one page request against the fork-tree metadata ceiling.
    pub const fn new(offset: usize, limit: usize) -> Result<Self, ForkProjectionError> {
        if limit == 0 {
            return Err(ForkProjectionError::ZeroPageSize);
        }
        if limit > MAXIMUM_FORK_PROJECTION_PAGE_SIZE {
            return Err(ForkProjectionError::PageTooLarge {
                maximum: MAXIMUM_FORK_PROJECTION_PAGE_SIZE,
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
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) current_branch_id: ForkBranchId,
    pub(crate) current_entry_id: Option<HistoryEntryId>,
    pub(crate) undo_depth: usize,
    pub(crate) redo_depth: usize,
    pub(crate) next_undo_label: Option<HistoryLabel>,
    pub(crate) next_redo_label: Option<HistoryLabel>,
    pub(crate) retained_entry_count: usize,
    pub(crate) retained_encoded_weight: u64,
    pub(crate) branch_count: usize,
    pub(crate) alternate_path_count: usize,
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
    pub(crate) entry_id: HistoryEntryId,
    pub(crate) label: HistoryLabel,
    pub(crate) kind_id: Option<HistoryKindId>,
    pub(crate) recorded_at: Option<HistoryRecordedAt>,
    pub(crate) group_id: Option<HistoryGroupId>,
    pub(crate) sequence: HistoryEntrySequence,
    pub(crate) committed_revision: HistoryRevision,
    pub(crate) encoded_weight: u64,
    pub(crate) position: HistoryEntryPosition,
    pub(crate) continuation_count: usize,
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

    /// Returns the optional host-supplied recorded-at stamp.
    #[must_use]
    pub const fn recorded_at(&self) -> Option<HistoryRecordedAt> {
        self.recorded_at
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

    /// Returns how many entries continue from this one.
    ///
    /// A graph fact, not a page fact: it counts every child, including the one
    /// this page continues to. A run always continues to exactly one child, so
    /// a renderer's fork count at this entry is one less than this. A run's
    /// last entry always returns zero, because a preferred-child walk only
    /// terminates at a node with no children at all.
    #[must_use]
    pub const fn continuation_count(&self) -> usize {
        self.continuation_count
    }
}

/// One bounded path page with no stable path identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkPathPage {
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) branch_id: Option<ForkBranchId>,
    pub(crate) head_entry_id: Option<HistoryEntryId>,
    pub(crate) offset: usize,
    pub(crate) total_entries: usize,
    pub(crate) entries: Vec<ForkEntryProjection>,
    pub(crate) preceding_continuation_count: usize,
    pub(crate) preceding_entry_id: Option<HistoryEntryId>,
    pub(crate) truncated_before: bool,
    pub(crate) truncated_after: bool,
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

    /// Returns how many entries continue from the position immediately above
    /// this run's first entry.
    ///
    /// For a default or branch path that position is the history root, and
    /// more than one root is reachable: undo to root, then record. For a
    /// continuation run it is the entry the run was anchored at, so the count
    /// includes this run and its siblings.
    ///
    /// One rule either way, and never the history root's children reported on
    /// a page that does not start there.
    #[must_use]
    pub const fn preceding_continuation_count(&self) -> usize {
        self.preceding_continuation_count
    }

    /// Returns the entry immediately above this run's first, or `None` at the
    /// history root.
    ///
    /// `None` is the origin -- the state the operator started from, which no
    /// entry names. `Some` is the fork point a continuation run hangs off,
    /// which is already a row in the list above.
    #[must_use]
    pub const fn preceding_entry_id(&self) -> Option<&HistoryEntryId> {
        self.preceding_entry_id.as_ref()
    }
}

/// Optional payload-free metadata for one first-class branch reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranchProjection {
    pub(crate) branch_id: ForkBranchId,
    pub(crate) head_entry_id: Option<HistoryEntryId>,
    pub(crate) divergence_entry_id: Option<HistoryEntryId>,
    pub(crate) divergence_branch_id: Option<ForkBranchId>,
    pub(crate) name: Option<String>,
    pub(crate) annotation: Option<String>,
    pub(crate) pinned: bool,
    pub(crate) current: bool,
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

    /// Returns the last entry shared with this branch's nearest ancestor
    /// branch -- the entry it forked at.
    ///
    /// Relative to the parent run, not to the current branch. Computing it
    /// against the current branch collapses a fork-off-a-fork onto an entry of
    /// the current path, which reports two structurally different forks at the
    /// same node.
    ///
    /// `None` when the branch shares nothing with any other, which is the
    /// root-attached case and not an error.
    #[must_use]
    pub const fn divergence_entry_id(&self) -> Option<&HistoryEntryId> {
        self.divergence_entry_id.as_ref()
    }

    /// Returns the branch this one forked off, paired with the entry above.
    #[must_use]
    pub const fn divergence_branch_id(&self) -> Option<&ForkBranchId> {
        self.divergence_branch_id.as_ref()
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
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) offset: usize,
    pub(crate) total_branches: usize,
    pub(crate) branches: Vec<ForkBranchProjection>,
    pub(crate) truncated_before: bool,
    pub(crate) truncated_after: bool,
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

/// One entry that continues from an anchor.
///
/// Continuations are the operator's forks. Every child of the anchor appears,
/// including the one the caller is already rendering inline: the projection is
/// never told which that is, so this page and `continuation_count` cannot
/// disagree, and the renderer drops the entry it already holds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkContinuation {
    pub(crate) entry_id: HistoryEntryId,
    pub(crate) label: HistoryLabel,
    pub(crate) recorded_at: Option<HistoryRecordedAt>,
    pub(crate) preferred: bool,
    pub(crate) entry_count: usize,
    pub(crate) branch_id: ForkBranchId,
    pub(crate) branch_name: Option<String>,
}

impl ForkContinuation {
    /// Returns the stable identity of the continuing entry.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns the continuing entry's presentation text.
    #[must_use]
    pub const fn label(&self) -> &HistoryLabel {
        &self.label
    }

    /// Returns the continuing entry's optional host-supplied stamp.
    #[must_use]
    pub const fn recorded_at(&self) -> Option<HistoryRecordedAt> {
        self.recorded_at
    }

    /// Returns whether a redo from the anchor takes this continuation.
    #[must_use]
    pub const fn preferred(&self) -> bool {
        self.preferred
    }

    /// Returns how many entries the run starting here holds, this one
    /// included, following preferred children to its leaf.
    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Returns the branch a consumer lands on by taking this continuation.
    ///
    /// Derived by following preferred children to a leaf and naming the branch
    /// whose head that is. Total, because recording always sets the head of
    /// the branch it commits to, so every leaf is some branch's head.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns that branch's optional name.
    #[must_use]
    pub fn branch_name(&self) -> Option<&str> {
        self.branch_name.as_deref()
    }
}

/// One bounded page of the continuations at a single anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkContinuationPage {
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) anchor_entry_id: Option<HistoryEntryId>,
    pub(crate) offset: usize,
    pub(crate) total_continuations: usize,
    pub(crate) continuations: Vec<ForkContinuation>,
    pub(crate) truncated_before: bool,
    pub(crate) truncated_after: bool,
}

impl ForkContinuationPage {
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

    /// Returns the entry these continue from, or `None` for the root.
    #[must_use]
    pub const fn anchor_entry_id(&self) -> Option<&HistoryEntryId> {
        self.anchor_entry_id.as_ref()
    }

    /// Returns graph-order offset.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns how many continuations the anchor has in total.
    #[must_use]
    pub const fn total_continuations(&self) -> usize {
        self.total_continuations
    }

    /// Returns the bounded continuations.
    #[must_use]
    pub fn continuations(&self) -> &[ForkContinuation] {
        &self.continuations
    }

    /// Returns whether earlier continuations precede this page.
    #[must_use]
    pub const fn truncated_before(&self) -> bool {
        self.truncated_before
    }

    /// Returns whether later continuations follow this page.
    #[must_use]
    pub const fn truncated_after(&self) -> bool {
        self.truncated_after
    }
}
