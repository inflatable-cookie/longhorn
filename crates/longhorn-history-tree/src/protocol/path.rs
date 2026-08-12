use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};
use longhorn_history::{HistoryAuthorityEpoch, HistoryRecordedAt};
use serde::{Deserialize, Serialize};

use crate::{ForkBranchId, ForkPathPage};

use super::{
    ForkHistoryProtocolVersion, ForkProjectionPosition, ForkProtocolProjectionError, count,
    project_position,
};

/// One payload-free entry on a requested path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkEntryRecord {
    /// Stable entry identity.
    pub entry_id: HistoryEntryId,
    /// Consumer-owned label.
    pub label: String,
    /// Optional consumer-owned kind.
    pub kind_id: Option<HistoryKindId>,
    /// Optional host-supplied recorded-at stamp, in epoch milliseconds.
    pub recorded_at: Option<HistoryRecordedAt>,
    /// Optional consumer-owned group.
    pub group_id: Option<HistoryGroupId>,
    /// Monotonic insertion sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// Revision that committed this immutable node.
    pub committed_revision: HistoryRevision,
    /// Consumer-measured payload weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub encoded_weight: u64,
    /// Current applied position.
    pub position: ForkProjectionPosition,
}

/// Explicit path selection. Alternate paths never load by default.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum ForkPathTargetProjection {
    /// Preferred linear-default path.
    Default,
    /// Path ending at one stable branch head.
    Branch {
        /// Explicit first-class branch selection.
        branch_id: ForkBranchId,
    },
}

/// Revision-bound request for one bounded path page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkPathPageCommand {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Authority lifetime observed by the caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by the caller.
    pub history_id: HistoryId,
    /// Exact graph revision required.
    pub expected_revision: HistoryRevision,
    /// Default or explicit alternate path.
    pub target: ForkPathTargetProjection,
    /// Newest-first entry offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Maximum requested records.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub limit: u64,
}

/// One bounded authoritative payload-free path page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkPathPageSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable graph identity.
    pub history_id: HistoryId,
    /// Exact projected revision.
    pub revision: HistoryRevision,
    /// Explicit branch selection, absent for the default path.
    pub branch_id: Option<ForkBranchId>,
    /// Path head, or root.
    pub head_entry_id: Option<HistoryEntryId>,
    /// Newest-first offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Full path length.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_entries: u64,
    /// Bounded payload-free records.
    pub entries: Vec<ForkEntryRecord>,
    /// Whether newer records precede this page.
    pub truncated_before: bool,
    /// Whether older records follow this page.
    pub truncated_after: bool,
}

impl ForkPathPageSnapshot {
    /// Projects one checked pure path page.
    pub fn from_page(
        authority_epoch: HistoryAuthorityEpoch,
        page: &ForkPathPage,
    ) -> Result<Self, ForkProtocolProjectionError> {
        Ok(Self {
            protocol_version: ForkHistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id: page.history_id().clone(),
            revision: page.revision(),
            branch_id: page.branch_id().cloned(),
            head_entry_id: page.head_entry_id().cloned(),
            offset: count(page.offset())?,
            total_entries: count(page.total_entries())?,
            entries: page
                .entries()
                .iter()
                .map(|entry| ForkEntryRecord {
                    entry_id: entry.entry_id().clone(),
                    label: entry.label().as_str().to_owned(),
                    kind_id: entry.kind_id().cloned(),
                    group_id: entry.group_id().cloned(),
                    recorded_at: entry.recorded_at(),
                    sequence: entry.sequence().get(),
                    committed_revision: entry.committed_revision(),
                    encoded_weight: entry.encoded_weight(),
                    position: project_position(entry.position()),
                })
                .collect(),
            truncated_before: page.truncated_before(),
            truncated_after: page.truncated_after(),
        })
    }
}
