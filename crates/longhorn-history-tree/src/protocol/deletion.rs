use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::HistoryAuthorityEpoch;
use serde::{Deserialize, Serialize};

use crate::{ForkBranchId, ForkCheckpointId, ForkPruningReceipt};

use super::{ForkHistoryProtocolVersion, ForkProtocolProjectionError, count};

/// Revision-bound request to delete one continuation and everything below it.
///
/// Irreversible. Nothing restores what this removes, which is why the
/// operation exists: a delete that could be undone would have to keep what it
/// deleted.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkDeleteContinuationCommand {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Authority lifetime observed by the caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by the caller.
    pub history_id: HistoryId,
    /// Exact graph revision required.
    pub expected_revision: HistoryRevision,
    /// First entry of the run to delete -- the same handle a checkout takes.
    pub entry_id: HistoryEntryId,
}

/// One removed node, named so a consumer can report what went.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkRemovedEntryRecord {
    /// Stable identity of the removed entry.
    pub entry_id: HistoryEntryId,
    /// Its original insertion sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// The consumer-measured weight it no longer occupies.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub encoded_weight: u64,
}

/// What a deletion or a prune removed.
///
/// Deliberately precise rather than a count. This is the only destructive
/// operation in either history domain, so a consumer that has to tell an
/// operator what happened can name every part of it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkRemovalReceiptProjection {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable graph identity. Carried for the same reason the navigation
    /// receipt carries it: the invalidation event built from this receipt has
    /// to name the history it invalidates.
    pub history_id: HistoryId,
    /// Revision the removal started from.
    pub previous_revision: HistoryRevision,
    /// Revision it committed at.
    pub committed_revision: HistoryRevision,
    /// Every removed entry, deepest first.
    pub removed_entries: Vec<ForkRemovedEntryRecord>,
    /// Branches whose heads were inside the removal.
    pub removed_branches: Vec<ForkBranchId>,
    /// Checkpoints anchored inside the removal.
    pub removed_checkpoints: Vec<ForkCheckpointId>,
    /// Entries the graph still holds.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_entry_count: u64,
    /// Weight the graph still holds.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_encoded_weight: u64,
    /// Entries a budget still governs -- everything unprotected. Beside the
    /// retained totals, not instead of them: the two answer different
    /// questions.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub unprotected_entry_count: u64,
    /// Encoded weight a budget still governs.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub unprotected_encoded_weight: u64,
}

impl ForkRemovalReceiptProjection {
    /// Projects one checked pure removal receipt.
    pub fn from_receipt(
        authority_epoch: HistoryAuthorityEpoch,
        history_id: HistoryId,
        receipt: &ForkPruningReceipt,
    ) -> Result<Self, ForkProtocolProjectionError> {
        Ok(Self {
            protocol_version: ForkHistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id,
            previous_revision: receipt.previous_revision(),
            committed_revision: receipt.committed_revision(),
            removed_entries: receipt
                .pruned_nodes()
                .iter()
                .map(|node| ForkRemovedEntryRecord {
                    entry_id: node.entry_id().clone(),
                    sequence: node.sequence().get(),
                    encoded_weight: node.encoded_weight(),
                })
                .collect(),
            removed_branches: receipt.removed_branches().to_vec(),
            removed_checkpoints: receipt.removed_checkpoints().to_vec(),
            retained_entry_count: count(receipt.retained_entry_count())?,
            retained_encoded_weight: receipt.retained_encoded_weight(),
            unprotected_entry_count: count(receipt.unprotected_entry_count())?,
            unprotected_encoded_weight: receipt.unprotected_encoded_weight(),
        })
    }
}

/// Revision-bound request to prune the graph to a budget.
///
/// The budget bounds the **unprotected** share -- everything not on the
/// current branch and not on a pinned branch. Size it for the transient
/// history to keep, not for the graph.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkPruneCommand {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Authority lifetime observed by the caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by the caller.
    pub history_id: HistoryId,
    /// Exact graph revision required.
    pub expected_revision: HistoryRevision,
    /// Maximum unprotected entries to keep.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub maximum_entries: u64,
    /// Maximum unprotected encoded weight to keep.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub maximum_encoded_weight: u64,
}

/// The result of a prune.
///
/// Tagged rather than an empty receipt, because "already inside the budget"
/// and "removed nothing that mattered" are different facts and a consumer
/// reporting to an operator should not have to infer which it got.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum ForkPruneResult {
    /// The unprotected share already fitted the budget. Nothing was removed
    /// and the revision did not move.
    Unchanged,
    /// One pruning transition committed.
    Pruned {
        /// What it removed, and what is left.
        receipt: ForkRemovalReceiptProjection,
    },
}
