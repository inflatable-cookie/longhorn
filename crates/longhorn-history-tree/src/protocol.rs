use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision,
};
use longhorn_history::{HistoryAuthorityEpoch, HistoryEntryPosition};
use serde::{Deserialize, Serialize};

use crate::{ForkBranchId, ForkBranchPage, ForkNavigationReceipt, ForkPathPage, ForkSummary};

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

/// One payload-free first-class branch record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkBranchRecord {
    /// Stable branch identity.
    pub branch_id: ForkBranchId,
    /// Branch head, or root.
    pub head_entry_id: Option<HistoryEntryId>,
    /// Shared ancestor relative to the current branch, or root.
    pub divergence_entry_id: Option<HistoryEntryId>,
    /// Optional branch name.
    pub name: Option<String>,
    /// Optional branch annotation.
    pub annotation: Option<String>,
    /// Retention pin state.
    pub pinned: bool,
    /// Whether this is the selected branch.
    pub current: bool,
}

/// Revision-bound request for one bounded branch page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkBranchPageCommand {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Authority lifetime observed by caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by caller.
    pub history_id: HistoryId,
    /// Exact graph revision required.
    pub expected_revision: HistoryRevision,
    /// Stable-id ordered offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Maximum requested records.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub limit: u64,
}

/// One bounded authoritative first-class branch page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkBranchPageSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable graph identity.
    pub history_id: HistoryId,
    /// Exact projected revision.
    pub revision: HistoryRevision,
    /// Stable-id ordered offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Total first-class branch count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_branches: u64,
    /// Bounded branch metadata.
    pub branches: Vec<ForkBranchRecord>,
    /// Whether earlier stable ids precede this page.
    pub truncated_before: bool,
    /// Whether later stable ids follow this page.
    pub truncated_after: bool,
}

impl ForkBranchPageSnapshot {
    /// Projects one checked pure branch page.
    pub fn from_page(
        authority_epoch: HistoryAuthorityEpoch,
        page: &ForkBranchPage,
    ) -> Result<Self, ForkProtocolProjectionError> {
        Ok(Self {
            protocol_version: ForkHistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id: page.history_id().clone(),
            revision: page.revision(),
            offset: count(page.offset())?,
            total_branches: count(page.total_branches())?,
            branches: page
                .branches()
                .iter()
                .map(|branch| ForkBranchRecord {
                    branch_id: branch.branch_id().clone(),
                    head_entry_id: branch.head_entry_id().cloned(),
                    divergence_entry_id: branch.divergence_entry_id().cloned(),
                    name: branch.name().map(str::to_owned),
                    annotation: branch.annotation().map(str::to_owned),
                    pinned: branch.pinned(),
                    current: branch.current(),
                })
                .collect(),
            truncated_before: page.truncated_before(),
            truncated_after: page.truncated_after(),
        })
    }
}

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

/// Coarse non-durable committed graph transition kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ForkChangedKind {
    /// A product mutation recorded a node.
    Record,
    /// Checked graph navigation committed.
    Navigation,
    /// Branch metadata changed.
    BranchMetadata,
    /// Retention pruned graph authority.
    Retention,
    /// Checkpoint metadata changed.
    Checkpoint,
    /// Persisted graph authority loaded.
    Imported,
    /// Graph authority reset.
    Reset,
}

/// Non-durable live invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkChangedEvent {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable graph identity.
    pub history_id: HistoryId,
    /// Previous graph revision, absent for load recovery.
    pub previous_revision: Option<HistoryRevision>,
    /// Authoritative resulting revision.
    pub committed_revision: HistoryRevision,
    /// Coarse invalidation category.
    pub kind: ForkChangedKind,
}

/// A platform-sized projection count exceeded the fixed protocol type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkProtocolProjectionError;

impl std::fmt::Display for ForkProtocolProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("fork projection count exceeds protocol bound")
    }
}

impl std::error::Error for ForkProtocolProjectionError {}

fn count(value: usize) -> Result<u64, ForkProtocolProjectionError> {
    u64::try_from(value).map_err(|_| ForkProtocolProjectionError)
}

const fn project_position(position: HistoryEntryPosition) -> ForkProjectionPosition {
    match position {
        HistoryEntryPosition::Past => ForkProjectionPosition::Past,
        HistoryEntryPosition::Current => ForkProjectionPosition::Current,
        HistoryEntryPosition::Future => ForkProjectionPosition::Future,
    }
}
