//! Snapshot, summary, and page projections.

use std::{error::Error, fmt};

use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision,
};
use serde::{Deserialize, Serialize};

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryEntryPosition,
    HistoryNavigationDirection, HistoryNavigationPosition, HistoryNavigationReceipt, HistoryPage,
    HistorySummary,
};

use super::{HistoryAuthorityEpoch, HistoryProjectionPosition, HistoryProtocolMode, HistoryProtocolProjectionError, HistoryProtocolVersion, project_count};

/// Payload-free durable baseline evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryBaselineProjection {
    /// Entries removed from the oldest applied prefix.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub pruned_entry_count: u64,
    /// Consumer-measured payload weight removed with that prefix.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub pruned_encoded_weight: u64,
    /// Last entry absorbed into the baseline.
    pub last_pruned_entry_id: Option<HistoryEntryId>,
    /// Last insertion sequence absorbed into the baseline.
    #[cfg_attr(feature = "bindings", ts(type = "number | null"))]
    pub last_pruned_sequence: Option<u64>,
}

/// Authoritative payload-free history summary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistorySummaryProjection {
    /// Stable history authority identity.
    pub history_id: HistoryId,
    /// Current structural history revision.
    pub revision: HistoryRevision,
    /// Public topology mode.
    pub mode: HistoryProtocolMode,
    /// Retained applied depth.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub undo_depth: u64,
    /// Retained future depth.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub redo_depth: u64,
    /// Current applied entry.
    pub current_entry_id: Option<HistoryEntryId>,
    /// Consumer-owned next undo label.
    pub next_undo_label: Option<String>,
    /// Consumer-owned next redo label.
    pub next_redo_label: Option<String>,
    /// Total retained entries.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_entry_count: u64,
    /// Total consumer-measured retained payload weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_encoded_weight: u64,
    /// Evidence for history pruned before retained entries.
    pub retained_baseline: HistoryBaselineProjection,
}

/// One live authoritative metadata snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistorySnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: HistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Payload-free authoritative state.
    pub summary: HistorySummaryProjection,
}

impl HistorySnapshot {
    /// Projects one checked kernel summary into the renderer protocol.
    pub fn from_summary(
        authority_epoch: HistoryAuthorityEpoch,
        summary: &HistorySummary,
    ) -> Result<Self, HistoryProtocolProjectionError> {
        Ok(Self {
            protocol_version: HistoryProtocolVersion::CURRENT,
            authority_epoch,
            summary: HistorySummaryProjection {
                history_id: summary.history_id().clone(),
                revision: summary.revision(),
                mode: HistoryProtocolMode::Linear,
                undo_depth: project_count(summary.undo_depth())?,
                redo_depth: project_count(summary.redo_depth())?,
                current_entry_id: summary.current_entry_id().cloned(),
                next_undo_label: summary
                    .next_undo_label()
                    .map(|label| label.as_str().to_owned()),
                next_redo_label: summary
                    .next_redo_label()
                    .map(|label| label.as_str().to_owned()),
                retained_entry_count: project_count(summary.retained_entry_count())?,
                retained_encoded_weight: summary.retained_encoded_weight(),
                retained_baseline: HistoryBaselineProjection {
                    pruned_entry_count: summary.retained_baseline().pruned_entry_count(),
                    pruned_encoded_weight: summary.retained_baseline().pruned_encoded_weight(),
                    last_pruned_entry_id: summary
                        .retained_baseline()
                        .last_pruned_entry_id()
                        .cloned(),
                    last_pruned_sequence: summary
                        .retained_baseline()
                        .last_pruned_sequence()
                        .map(|sequence| sequence.get()),
                },
            },
        })
    }
}

/// One authoritative payload-free retained entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryEntryRecord {
    /// Stable entry identity.
    pub entry_id: HistoryEntryId,
    /// Consumer-owned display label.
    pub label: String,
    /// Optional consumer-owned kind.
    pub kind_id: Option<HistoryKindId>,
    /// Optional committed group identity.
    pub group_id: Option<HistoryGroupId>,
    /// Monotonic insertion sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// Last structural revision that changed this entry.
    pub committed_revision: HistoryRevision,
    /// Consumer-measured encoded payload weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub encoded_weight: u64,
    /// Current authoritative topology position.
    pub position: HistoryProjectionPosition,
}

/// Bounded revision-safe metadata page request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryPageCommand {
    /// Exact metadata protocol line.
    pub protocol_version: HistoryProtocolVersion,
    /// Authority lifetime observed by the caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by the caller.
    pub history_id: HistoryId,
    /// Structural revision observed by the caller.
    pub expected_revision: HistoryRevision,
    /// Newest-first entry offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Maximum requested entries.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub limit: u64,
}

/// One bounded authoritative payload-free metadata page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryPageSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: HistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable history identity.
    pub history_id: HistoryId,
    /// Exact projected structural revision.
    pub revision: HistoryRevision,
    /// Newest-first entry offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Total authoritative retained entries.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_entries: u64,
    /// Payload-free entry records.
    pub entries: Vec<HistoryEntryRecord>,
    /// Whether newer entries precede this page.
    pub truncated_before: bool,
    /// Whether older entries follow this page.
    pub truncated_after: bool,
    /// Evidence for history pruned before retained entries.
    pub retained_baseline: HistoryBaselineProjection,
}

impl HistoryPageSnapshot {
    /// Projects one checked kernel page into the renderer protocol.
    pub fn from_page(
        authority_epoch: HistoryAuthorityEpoch,
        page: &HistoryPage,
    ) -> Result<Self, HistoryProtocolProjectionError> {
        let entries = page
            .entries()
            .iter()
            .map(|entry| HistoryEntryRecord {
                entry_id: entry.entry_id().clone(),
                label: entry.label().as_str().to_owned(),
                kind_id: entry.kind_id().cloned(),
                group_id: entry.group_id().cloned(),
                sequence: entry.sequence().get(),
                committed_revision: entry.committed_revision(),
                encoded_weight: entry.encoded_weight(),
                position: match entry.position() {
                    HistoryEntryPosition::Past => HistoryProjectionPosition::Past,
                    HistoryEntryPosition::Current => HistoryProjectionPosition::Current,
                    HistoryEntryPosition::Future => HistoryProjectionPosition::Future,
                },
            })
            .collect();
        Ok(Self {
            protocol_version: HistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id: page.history_id().clone(),
            revision: page.revision(),
            offset: project_count(page.offset())?,
            total_entries: project_count(page.total_entries())?,
            entries,
            truncated_before: page.truncated_before(),
            truncated_after: page.truncated_after(),
            retained_baseline: HistoryBaselineProjection {
                pruned_entry_count: page.retained_baseline().pruned_entry_count(),
                pruned_encoded_weight: page.retained_baseline().pruned_encoded_weight(),
                last_pruned_entry_id: page.retained_baseline().last_pruned_entry_id().cloned(),
                last_pruned_sequence: page
                    .retained_baseline()
                    .last_pruned_sequence()
                    .map(|sequence| sequence.get()),
            },
        })
    }
}

