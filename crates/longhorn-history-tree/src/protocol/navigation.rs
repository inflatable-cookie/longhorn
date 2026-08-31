use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};
use longhorn_history::HistoryAuthorityEpoch;
use serde::{Deserialize, Serialize};

use crate::{ForkBranchId, ForkNavigationReceipt};

use super::{ForkHistoryProtocolVersion, ForkSnapshot};

/// Stable payload-free graph navigation intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum ForkNavigationTargetProjection {
    /// Move to the parent of the current node.
    Undo,
    /// Move to the deterministic preferred child.
    Redo,
    /// Move to one entry on one stable branch.
    Checkout {
        /// Stable target branch.
        branch_id: ForkBranchId,
        /// Stable target entry.
        entry_id: HistoryEntryId,
    },
    /// Move to a branch's root, holding no entry.
    CheckoutBranchRoot {
        /// Stable target branch.
        branch_id: ForkBranchId,
    },
    /// Make one entry its parent's preferred continuation, applying none of
    /// it. The chosen run becomes the default path; the previous default
    /// becomes a continuation at the same entry.
    CheckoutContinuation {
        /// Entry to prefer.
        entry_id: HistoryEntryId,
    },
}

/// One exact revision-bound graph navigation command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkNavigationCommand {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Authority lifetime observed by caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by caller.
    pub history_id: HistoryId,
    /// Caller-injected plan and correlation identity.
    pub plan_id: HistoryPlanId,
    /// Exact graph revision required.
    pub expected_revision: HistoryRevision,
    /// Stable graph navigation intent.
    pub target: ForkNavigationTargetProjection,
}

/// Exact payload-free committed graph-navigation receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkNavigationReceiptProjection {
    /// Stable graph identity.
    pub history_id: HistoryId,
    /// Committed plan identity.
    pub plan_id: HistoryPlanId,
    /// Admitted source revision.
    pub previous_revision: HistoryRevision,
    /// Authoritative successor revision.
    pub committed_revision: HistoryRevision,
    /// Source node, or root.
    pub source_entry_id: Option<HistoryEntryId>,
    /// Target node, or root.
    pub target_entry_id: Option<HistoryEntryId>,
    /// Selected stable target branch.
    pub target_branch_id: ForkBranchId,
    /// Nodes moved in product-apply order.
    pub moved_entry_ids: Vec<HistoryEntryId>,
}

impl ForkNavigationReceiptProjection {
    /// Removes typed product payloads from one committed receipt.
    #[must_use]
    pub fn from_receipt(receipt: &ForkNavigationReceipt) -> Self {
        Self {
            history_id: receipt.history_id().clone(),
            plan_id: receipt.plan_id().clone(),
            previous_revision: receipt.previous_revision(),
            committed_revision: receipt.committed_revision(),
            source_entry_id: receipt.source_node_id().cloned(),
            target_entry_id: receipt.target_node_id().cloned(),
            target_branch_id: receipt.target_branch_id().clone(),
            moved_entry_ids: receipt.moved_entry_ids().to_vec(),
        }
    }
}

/// Stable client-visible graph navigation rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ForkNavigationRejectionCode {
    /// Exact metadata protocol is unsupported.
    IncompatibleProtocol,
    /// Caller targeted a replaced authority lifetime.
    StaleAuthority,
    /// Caller targeted another history identity.
    ForeignHistory,
    /// Caller targeted another graph revision.
    StaleRevision,
    /// No applied node can be undone.
    NothingToUndo,
    /// No preferred future can be redone.
    NothingToRedo,
    /// Requested checkout is already the current target.
    AlreadyAtTarget,
    /// Branch or entry target does not exist.
    UnknownTarget,
    /// Current product authorization rejected the operation.
    Unauthorized,
    /// Consumer apply failed and exact rollback succeeded.
    ApplyFailed,
    /// Consumer apply and rollback both failed.
    RollbackFailed,
    /// Current authority rejected invalid structural intent.
    InvalidRequest,
}

/// Client-visible graph navigation rejection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkNavigationRejectionProjection {
    /// Stable rejection category.
    pub code: ForkNavigationRejectionCode,
    /// Renderer-safe diagnostic.
    pub detail: String,
    /// Whether fresh authority may make later work admissible.
    pub refresh_required: bool,
}

/// Authoritative graph-navigation result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum ForkNavigationResult {
    /// Product apply and graph commit succeeded.
    Committed {
        /// Fresh authoritative snapshot.
        snapshot: ForkSnapshot,
        /// Exact payload-free receipt.
        receipt: ForkNavigationReceiptProjection,
    },
    /// Current authority rejected without a graph commit.
    Rejected {
        /// Fresh authoritative snapshot.
        snapshot: ForkSnapshot,
        /// Stable rejection.
        rejection: ForkNavigationRejectionProjection,
    },
}
