use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::{HistoryAuthorityEpoch, HistoryRecordedAt};
use serde::{Deserialize, Serialize};

use crate::{ForkBranchId, ForkContinuationPage};

use super::{ForkHistoryProtocolVersion, ForkProtocolProjectionError, count};

/// One entry continuing from an anchor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkContinuationRecord {
    /// Stable identity of the continuing entry.
    pub entry_id: HistoryEntryId,
    /// Consumer-owned label.
    pub label: String,
    /// Optional host-supplied recorded-at stamp, in epoch milliseconds.
    pub recorded_at: Option<HistoryRecordedAt>,
    /// Whether a redo from the anchor takes this continuation.
    pub preferred: bool,
    /// Entries in the run starting here, this one included.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub entry_count: u64,
    /// Branch a consumer lands on by taking this continuation.
    pub branch_id: ForkBranchId,
    /// That branch's optional name.
    pub branch_name: Option<String>,
}

/// Revision-bound request for the continuations at one anchor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkContinuationPageCommand {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Authority lifetime observed by the caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by the caller.
    pub history_id: HistoryId,
    /// Exact graph revision required.
    pub expected_revision: HistoryRevision,
    /// Entry the continuations continue from, or root.
    pub anchor_entry_id: Option<HistoryEntryId>,
    /// Graph-order offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Maximum requested records.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub limit: u64,
}

/// One bounded authoritative page of the continuations at an anchor.
///
/// Every child of the anchor appears, including the one the caller already
/// renders inline. The authority is never told which that is, so this page and
/// `ForkEntryRecord::continuation_count` cannot disagree.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkContinuationPageSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable graph identity.
    pub history_id: HistoryId,
    /// Exact projected revision.
    pub revision: HistoryRevision,
    /// Entry these continue from, or root.
    pub anchor_entry_id: Option<HistoryEntryId>,
    /// Graph-order offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Total continuations at the anchor.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_continuations: u64,
    /// Bounded continuation records.
    pub continuations: Vec<ForkContinuationRecord>,
    /// Whether earlier continuations precede this page.
    pub truncated_before: bool,
    /// Whether later continuations follow this page.
    pub truncated_after: bool,
}

impl ForkContinuationPageSnapshot {
    /// Projects one checked pure continuation page.
    pub fn from_page(
        authority_epoch: HistoryAuthorityEpoch,
        page: &ForkContinuationPage,
    ) -> Result<Self, ForkProtocolProjectionError> {
        Ok(Self {
            protocol_version: ForkHistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id: page.history_id().clone(),
            revision: page.revision(),
            anchor_entry_id: page.anchor_entry_id().cloned(),
            offset: count(page.offset())?,
            total_continuations: count(page.total_continuations())?,
            continuations: page
                .continuations()
                .iter()
                .map(|continuation| {
                    Ok(ForkContinuationRecord {
                        entry_id: continuation.entry_id().clone(),
                        label: continuation.label().as_str().to_owned(),
                        recorded_at: continuation.recorded_at(),
                        preferred: continuation.preferred(),
                        entry_count: count(continuation.entry_count())?,
                        branch_id: continuation.branch_id().clone(),
                        branch_name: continuation.branch_name().map(str::to_owned),
                    })
                })
                .collect::<Result<Vec<_>, ForkProtocolProjectionError>>()?,
            truncated_before: page.truncated_before(),
            truncated_after: page.truncated_after(),
        })
    }
}
