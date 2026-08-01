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

/// Current exact metadata-only renderer protocol version.
pub const HISTORY_PROTOCOL_VERSION: u32 = 1;

/// Exact metadata-only history protocol line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct HistoryProtocolVersion(u32);

impl HistoryProtocolVersion {
    /// Current exact protocol line.
    pub const CURRENT: Self = Self(HISTORY_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Nonzero identity for one live history authority lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct HistoryAuthorityEpoch(u64);

impl HistoryAuthorityEpoch {
    /// Constructs a nonzero live authority epoch.
    pub const fn new(value: u64) -> Result<Self, HistoryAuthorityEpochError> {
        if value == 0 {
            Err(HistoryAuthorityEpochError)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the serialized epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for HistoryAuthorityEpoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// A history authority epoch was zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryAuthorityEpochError;

impl fmt::Display for HistoryAuthorityEpochError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("history authority epoch must be nonzero")
    }
}

impl Error for HistoryAuthorityEpochError {}

/// Public history topology mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryProtocolMode {
    /// One applied path and one retained redo path.
    Linear,
}

/// Authoritative topology position of one projected entry.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryProjectionPosition {
    /// Applied before the current entry.
    Past,
    /// Current applied entry.
    Current,
    /// Retained redo entry.
    Future,
}

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

/// Stable payload-free navigation intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum HistoryNavigationTargetProjection {
    /// Move one entry toward the retained baseline.
    Undo,
    /// Move one entry toward the newest retained state.
    Redo,
    /// Make one stable retained entry current.
    Checkout {
        /// Stable entry identity, never a presentation index.
        entry_id: HistoryEntryId,
    },
}

/// One revision-bound renderer navigation command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryNavigationCommand {
    /// Exact metadata protocol line.
    pub protocol_version: HistoryProtocolVersion,
    /// Authority lifetime observed by the caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by the caller.
    pub history_id: HistoryId,
    /// Caller-injected plan and request correlation identity.
    pub plan_id: HistoryPlanId,
    /// Exact structural revision required for admission.
    pub expected_revision: HistoryRevision,
    /// Stable navigation intent.
    pub target: HistoryNavigationTargetProjection,
}

/// Direction of one committed linear navigation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryNavigationDirectionProjection {
    /// Moved toward the retained baseline.
    Undo,
    /// Moved toward the newest retained state.
    Redo,
    /// Explicit checkout retained the same current entry.
    Stationary,
}

/// Payload-free authoritative navigation position.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryNavigationPositionProjection {
    /// Retained applied depth.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub applied_depth: u64,
    /// Retained future depth.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub future_depth: u64,
    /// Current applied entry.
    pub current_entry_id: Option<HistoryEntryId>,
    /// Consumer-owned next undo label.
    pub next_undo_label: Option<String>,
    /// Next retained redo entry.
    pub next_redo_entry_id: Option<HistoryEntryId>,
    /// Consumer-owned next redo label.
    pub next_redo_label: Option<String>,
}

/// Exact payload-free committed navigation receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryNavigationReceiptProjection {
    /// Stable history identity.
    pub history_id: HistoryId,
    /// Committed plan and request correlation identity.
    pub plan_id: HistoryPlanId,
    /// Admitted source revision.
    pub previous_revision: HistoryRevision,
    /// Authoritative successor revision.
    pub committed_revision: HistoryRevision,
    /// Committed movement direction.
    pub direction: HistoryNavigationDirectionProjection,
    /// Entries moved in product-apply order.
    pub moved_entry_ids: Vec<HistoryEntryId>,
    /// Admitted source position.
    pub source_position: HistoryNavigationPositionProjection,
    /// Authoritative resulting position.
    pub authoritative_position: HistoryNavigationPositionProjection,
}

impl HistoryNavigationReceiptProjection {
    /// Projects one committed kernel receipt without its typed payload.
    pub fn from_receipt(
        receipt: &HistoryNavigationReceipt,
    ) -> Result<Self, HistoryProtocolProjectionError> {
        Ok(Self {
            history_id: receipt.history_id().clone(),
            plan_id: receipt.plan_id().clone(),
            previous_revision: receipt.previous_revision(),
            committed_revision: receipt.committed_revision(),
            direction: project_direction(receipt.direction()),
            moved_entry_ids: receipt.moved_entry_ids().to_vec(),
            source_position: project_position(receipt.source_position())?,
            authoritative_position: project_position(receipt.authoritative_position())?,
        })
    }
}

/// Stable client-visible navigation rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryNavigationRejectionCode {
    /// The exact metadata protocol is unsupported.
    IncompatibleProtocol,
    /// The caller targeted a replaced authority lifetime.
    StaleAuthority,
    /// The caller targeted another history identity.
    ForeignHistory,
    /// The caller targeted an older structural revision.
    StaleRevision,
    /// No applied entry can be undone.
    NothingToUndo,
    /// No retained future entry can be redone.
    NothingToRedo,
    /// Checkout named no retained entry.
    UnknownEntry,
    /// Current product authorization rejected the operation.
    Unauthorized,
    /// Consumer product apply failed and exact rollback succeeded.
    ApplyFailed,
    /// Consumer product apply and rollback both failed.
    RollbackFailed,
    /// Current authority rejected invalid structural intent.
    InvalidRequest,
}

/// Client-visible navigation rejection with safe diagnostic detail.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryNavigationRejectionProjection {
    /// Stable rejection category.
    pub code: HistoryNavigationRejectionCode,
    /// Diagnostic safe at the renderer boundary.
    pub detail: String,
    /// Whether a fresh snapshot may make a later request admissible.
    pub refresh_required: bool,
}

/// Authoritative navigation result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum HistoryNavigationResult {
    /// Product apply and structural commit both succeeded.
    Committed {
        /// Fresh authoritative snapshot.
        snapshot: HistorySnapshot,
        /// Exact payload-free commit receipt.
        receipt: Box<HistoryNavigationReceiptProjection>,
    },
    /// Current authority rejected the command without a history commit.
    Rejected {
        /// Fresh authoritative snapshot.
        snapshot: HistorySnapshot,
        /// Stable rejection.
        rejection: HistoryNavigationRejectionProjection,
    },
}

/// Coarse payload-free committed transition kind used for live invalidation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryChangedKind {
    /// A successful product mutation recorded or coalesced.
    Record,
    /// A checked navigation plan committed.
    Navigation,
    /// Retention limits changed.
    LimitsChanged,
    /// Persisted structural history was accepted.
    Imported,
    /// Persisted history was explicitly discarded.
    DiscardedPersistence,
    /// Retained structural history was reset.
    Reset,
}

/// Non-durable live invalidation hint for one committed transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryChangedEvent {
    /// Exact metadata protocol line.
    pub protocol_version: HistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable history identity.
    pub history_id: HistoryId,
    /// Prior in-memory revision, absent for load recovery.
    pub previous_revision: Option<HistoryRevision>,
    /// Authoritative resulting revision.
    pub committed_revision: HistoryRevision,
    /// Coarse transition category.
    pub kind: HistoryChangedKind,
}

impl HistoryChangedEvent {
    /// Projects one committed kernel transition into a live invalidation hint.
    #[must_use]
    pub fn from_transition(
        authority_epoch: HistoryAuthorityEpoch,
        transition: &HistoryCommittedTransition,
    ) -> Self {
        Self {
            protocol_version: HistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id: transition.history_id().clone(),
            previous_revision: transition.previous_revision(),
            committed_revision: transition.committed_revision(),
            kind: match transition.kind() {
                HistoryCommittedTransitionKind::Record { .. } => HistoryChangedKind::Record,
                HistoryCommittedTransitionKind::Navigation { .. } => HistoryChangedKind::Navigation,
                HistoryCommittedTransitionKind::LimitsChanged { .. } => {
                    HistoryChangedKind::LimitsChanged
                }
                HistoryCommittedTransitionKind::Imported { .. } => HistoryChangedKind::Imported,
                HistoryCommittedTransitionKind::DiscardedPersistence { .. } => {
                    HistoryChangedKind::DiscardedPersistence
                }
                HistoryCommittedTransitionKind::Reset { .. } => HistoryChangedKind::Reset,
            },
        }
    }
}

/// A bounded kernel projection could not fit the fixed renderer protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryProtocolProjectionError {
    /// One platform-sized count exceeded the protocol's unsigned 64-bit bound.
    CountOverflow,
}

impl fmt::Display for HistoryProtocolProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountOverflow => {
                formatter.write_str("history projection count exceeds the protocol bound")
            }
        }
    }
}

impl Error for HistoryProtocolProjectionError {}

fn project_count(value: usize) -> Result<u64, HistoryProtocolProjectionError> {
    u64::try_from(value).map_err(|_| HistoryProtocolProjectionError::CountOverflow)
}

fn project_direction(
    direction: HistoryNavigationDirection,
) -> HistoryNavigationDirectionProjection {
    match direction {
        HistoryNavigationDirection::Undo => HistoryNavigationDirectionProjection::Undo,
        HistoryNavigationDirection::Redo => HistoryNavigationDirectionProjection::Redo,
        HistoryNavigationDirection::Stationary => HistoryNavigationDirectionProjection::Stationary,
    }
}

fn project_position(
    position: &HistoryNavigationPosition,
) -> Result<HistoryNavigationPositionProjection, HistoryProtocolProjectionError> {
    Ok(HistoryNavigationPositionProjection {
        applied_depth: project_count(position.applied_depth())?,
        future_depth: project_count(position.future_depth())?,
        current_entry_id: position.current_entry_id().cloned(),
        next_undo_label: position
            .next_undo_label()
            .map(|label| label.as_str().to_owned()),
        next_redo_entry_id: position.next_redo_entry_id().cloned(),
        next_redo_label: position
            .next_redo_label()
            .map(|label| label.as_str().to_owned()),
    })
}
