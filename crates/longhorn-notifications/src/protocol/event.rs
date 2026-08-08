//! Notification change events.

use longhorn_core::{NotificationId, NotificationLedgerRevision, NotificationRequestId};
use serde::{Deserialize, Serialize};

use super::*;

/// Authoritative event summary kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationChangedKind {
    /// Fresh record addition.
    Added,
    /// Explicit replacement.
    Replaced,
    /// Explicit mark-seen transition.
    Seen,
    /// Explicit dismissal.
    Dismissed,
    /// Explicit clear transition.
    Cleared,
    /// Retention limits changed.
    RetentionChanged,
}

/// Non-durable request-correlated authority invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationChangedEvent {
    /// Exact protocol line.
    pub protocol_version: NotificationProtocolVersion,
    /// Request correlation identity.
    pub request_id: NotificationRequestId,
    /// Live authority cursor.
    pub authority: NotificationAuthorityProjection,
    /// Ledger revision before commit.
    pub previous_ledger_revision: NotificationLedgerRevision,
    /// Ledger revision after commit.
    pub committed_ledger_revision: NotificationLedgerRevision,
    /// Every directly changed or removed record id.
    pub affected_notification_ids: Vec<NotificationId>,
    /// Change source.
    pub kind: NotificationChangedKind,
}

impl NotificationChangedEvent {
    /// Projects an event only for a revision-advancing commit.
    #[must_use]
    pub fn from_mutation(result: &NotificationMutationResult) -> Option<Self> {
        let NotificationMutationResult::Committed {
            request_id,
            snapshot,
            receipt,
        } = result
        else {
            return None;
        };
        if receipt.previous_ledger_revision() == receipt.committed_ledger_revision() {
            return None;
        }
        let kind = match receipt.as_ref() {
            NotificationMutationReceiptProjection::Added { .. } => NotificationChangedKind::Added,
            NotificationMutationReceiptProjection::Replaced { .. } => {
                NotificationChangedKind::Replaced
            }
            NotificationMutationReceiptProjection::Seen { .. } => NotificationChangedKind::Seen,
            NotificationMutationReceiptProjection::Removed { removals, .. } => {
                if removals
                    .iter()
                    .all(|removal| removal.reason == NotificationRemovalReasonProjection::Dismissed)
                {
                    NotificationChangedKind::Dismissed
                } else {
                    NotificationChangedKind::Cleared
                }
            }
            NotificationMutationReceiptProjection::RetentionChanged { .. } => {
                NotificationChangedKind::RetentionChanged
            }
        };
        Some(Self {
            protocol_version: NotificationProtocolVersion::CURRENT,
            request_id: request_id.clone(),
            authority: snapshot.authority.clone(),
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
            affected_notification_ids: receipt.affected_notification_ids(),
            kind,
        })
    }
}
