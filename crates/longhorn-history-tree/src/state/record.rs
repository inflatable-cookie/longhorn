//! Mutation receipts for fork history.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};

use crate::{ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkHistoryNode};
use longhorn_history::HistoryEntryMetadata;

/// One already-applied product mutation offered to the graph authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkRecord<P> {
    pub(crate) expected_revision: HistoryRevision,
    pub(crate) entry_id: HistoryEntryId,
    pub(crate) metadata: HistoryEntryMetadata,
    pub(crate) encoded_weight: u64,
    pub(crate) payload: P,
    pub(crate) divergent_branch: Option<ForkBranchSeed>,
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
    pub(crate) previous_revision: HistoryRevision,
    pub(crate) committed_revision: HistoryRevision,
    pub(crate) entry_id: HistoryEntryId,
    pub(crate) branch_id: ForkBranchId,
    pub(crate) parent_entry_id: Option<HistoryEntryId>,
    pub(crate) previous_branch_head: Option<HistoryEntryId>,
    pub(crate) diverged: bool,
    pub(crate) replaced_preferred_child: Option<HistoryEntryId>,
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
    pub(crate) previous_revision: HistoryRevision,
    pub(crate) committed_revision: HistoryRevision,
    pub(crate) branch_id: ForkBranchId,
    pub(crate) previous_metadata: ForkBranchMetadata,
    pub(crate) committed_metadata: ForkBranchMetadata,
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

