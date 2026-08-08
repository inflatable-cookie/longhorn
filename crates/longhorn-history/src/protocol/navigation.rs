//! Navigation commands and result projections.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};
use serde::{Deserialize, Serialize};

use crate::{HistoryNavigationDirection, HistoryNavigationPosition, HistoryNavigationReceipt};

use super::{
    HistoryAuthorityEpoch, HistoryProtocolProjectionError, HistoryProtocolVersion, HistorySnapshot,
    project_count,
};

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

pub(crate) fn project_direction(
    direction: HistoryNavigationDirection,
) -> HistoryNavigationDirectionProjection {
    match direction {
        HistoryNavigationDirection::Undo => HistoryNavigationDirectionProjection::Undo,
        HistoryNavigationDirection::Redo => HistoryNavigationDirectionProjection::Redo,
        HistoryNavigationDirection::Stationary => HistoryNavigationDirectionProjection::Stationary,
    }
}

pub(crate) fn project_position(
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
