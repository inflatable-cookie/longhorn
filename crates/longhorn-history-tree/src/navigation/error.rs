//! Navigation rejection errors.

use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};

use crate::ForkBranchId;
/// Rejected graph navigation.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkNavigationError<E> {
    /// Plan belongs to another graph authority.
    WrongHistory {
        /// Expected graph identity.
        expected: HistoryId,
        /// Supplied graph identity.
        actual: HistoryId,
    },
    /// Request or plan revision is stale.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Current node is the root.
    NothingToUndo,
    /// Current node has no preferred child.
    NothingToRedo,
    /// Requested checkout is already current.
    AlreadyAtTarget,
    /// Entry identity does not exist.
    UnknownEntry(HistoryEntryId),
    /// Branch identity does not exist.
    UnknownBranch(ForkBranchId),
    /// Target entry is not on the selected branch.
    EntryOutsideBranch {
        /// Selected branch.
        branch_id: ForkBranchId,
        /// Requested target.
        entry_id: HistoryEntryId,
    },
    /// A preferred child has no first-class branch reference.
    UnreferencedTarget(HistoryEntryId),
    /// Planned route exceeded its hard bound.
    RouteTooLong {
        /// Maximum navigation steps.
        maximum: usize,
        /// Planned navigation steps.
        actual: usize,
    },
    /// Consumer inverse policy rejected a node.
    Policy {
        /// Rejected entry.
        entry_id: HistoryEntryId,
        /// Consumer policy failure.
        error: E,
    },
    /// Plan no longer matches graph state.
    InvalidPlan,
    /// Product apply failed and exact rollback was verified.
    RolledBack {
        /// Product apply failure.
        error: E,
    },
    /// Product apply and rollback both failed.
    RollbackFailed {
        /// Product apply failure.
        error: E,
        /// Rollback failure.
        rollback_error: E,
    },
    /// Revision could not advance.
    RevisionOverflow,
}

impl<E> fmt::Display for ForkNavigationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongHistory { .. } => "fork navigation plan belongs to another history",
            Self::StaleRevision { .. } => "fork navigation revision is stale",
            Self::NothingToUndo => "fork history has nothing to undo",
            Self::NothingToRedo => "fork history has nothing to redo",
            Self::AlreadyAtTarget => "fork history is already at the requested target",
            Self::UnknownEntry(_) => "fork navigation entry does not exist",
            Self::UnknownBranch(_) => "fork navigation branch does not exist",
            Self::EntryOutsideBranch { .. } => "fork navigation entry is outside its branch",
            Self::UnreferencedTarget(_) => "fork preferred child has no branch reference",
            Self::RouteTooLong { .. } => "fork navigation route exceeds its hard limit",
            Self::Policy { .. } => "fork navigation inverse policy failed",
            Self::InvalidPlan => "fork navigation plan is invalid",
            Self::RolledBack { .. } => "fork navigation apply failed and rolled back",
            Self::RollbackFailed { .. } => "fork navigation apply and rollback failed",
            Self::RevisionOverflow => "fork history revision cannot advance",
        })
    }
}

impl<E: Error + fmt::Debug> Error for ForkNavigationError<E> {}
