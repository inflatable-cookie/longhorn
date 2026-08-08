use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::{HistoryAuthorityEpoch, HistoryEntryPosition};
use serde::{Deserialize, Serialize};

use crate::{ForkBranchId, ForkSummary};

use super::{ForkProtocolProjectionError, count};

/// Current exact metadata-only fork-history protocol version.
pub const FORK_HISTORY_PROTOCOL_VERSION: u32 = 1;

/// Exact metadata-only fork-history protocol line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct ForkHistoryProtocolVersion(u32);

impl ForkHistoryProtocolVersion {
    /// Current exact protocol line.
    pub const CURRENT: Self = Self(FORK_HISTORY_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Position of one payload-free entry relative to current applied authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ForkProjectionPosition {
    /// Applied before the current node.
    Past,
    /// Current applied node.
    Current,
    /// Retained but not currently applied.
    Future,
}

/// Payload-free linear-default graph summary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkSummaryProjection {
    /// Stable graph authority identity.
    pub history_id: HistoryId,
    /// Exact graph revision.
    pub revision: HistoryRevision,
    /// Selected first-class branch.
    pub current_branch_id: ForkBranchId,
    /// Current applied node, or root.
    pub current_entry_id: Option<HistoryEntryId>,
    /// Applied depth.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub undo_depth: u64,
    /// Preferred future depth.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub redo_depth: u64,
    /// Consumer-owned next undo label.
    pub next_undo_label: Option<String>,
    /// Consumer-owned next preferred-redo label.
    pub next_redo_label: Option<String>,
    /// Total retained nodes.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_entry_count: u64,
    /// Total consumer-measured retained payload weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_encoded_weight: u64,
    /// First-class branch count without eager branch metadata.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub branch_count: u64,
    /// Derived leaf-path count without eager path data.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub alternate_path_count: u64,
}

impl ForkSummaryProjection {
    /// Projects one pure summary without payload or alternate collections.
    pub fn from_summary(summary: &ForkSummary) -> Result<Self, ForkProtocolProjectionError> {
        Ok(Self {
            history_id: summary.history_id().clone(),
            revision: summary.revision(),
            current_branch_id: summary.current_branch_id().clone(),
            current_entry_id: summary.current_entry_id().cloned(),
            undo_depth: count(summary.undo_depth())?,
            redo_depth: count(summary.redo_depth())?,
            next_undo_label: summary
                .next_undo_label()
                .map(|label| label.as_str().to_owned()),
            next_redo_label: summary
                .next_redo_label()
                .map(|label| label.as_str().to_owned()),
            retained_entry_count: count(summary.retained_entry_count())?,
            retained_encoded_weight: summary.retained_encoded_weight(),
            branch_count: count(summary.branch_count())?,
            alternate_path_count: count(summary.alternate_path_count())?,
        })
    }
}

/// One live authoritative fork-history snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime shared with history host semantics.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Linear-default payload-free state.
    pub summary: ForkSummaryProjection,
}

impl ForkSnapshot {
    /// Projects one pure graph summary under a live authority epoch.
    pub fn from_summary(
        authority_epoch: HistoryAuthorityEpoch,
        summary: &ForkSummary,
    ) -> Result<Self, ForkProtocolProjectionError> {
        Ok(Self {
            protocol_version: ForkHistoryProtocolVersion::CURRENT,
            authority_epoch,
            summary: ForkSummaryProjection::from_summary(summary)?,
        })
    }
}

pub(crate) const fn project_position(position: HistoryEntryPosition) -> ForkProjectionPosition {
    match position {
        HistoryEntryPosition::Past => ForkProjectionPosition::Past,
        HistoryEntryPosition::Current => ForkProjectionPosition::Current,
        HistoryEntryPosition::Future => ForkProjectionPosition::Future,
    }
}
