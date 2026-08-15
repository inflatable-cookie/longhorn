//! Mutation commands, receipts, and results.

use longhorn_core::{NotificationId, NotificationLedgerRevision, NotificationRequestId};
use serde::{Deserialize, Serialize};

use crate::{NotificationRemoval, NotificationRemovalReason};

use super::*;

/// Revision-bound notification mutation command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum NotificationMutationCommand {
    /// Adds one new notification.
    Add {
        /// Correlation identity echoed in the result.
        request_id: NotificationRequestId,
        /// Protocol version the caller negotiated.
        protocol_version: NotificationProtocolVersion,
        /// Authority composition the caller expects.
        authority: NotificationAuthorityProjection,
        /// Ledger revision the caller observed.
        expected_ledger_revision: NotificationLedgerRevision,
        /// Identity assigned to the new notification.
        notification_id: NotificationId,
        /// Content of the new notification.
        draft: NotificationDraftProjection,
    },
    /// Replaces one existing notification.
    Replace {
        /// Correlation identity echoed in the result.
        request_id: NotificationRequestId,
        /// Protocol version the caller negotiated.
        protocol_version: NotificationProtocolVersion,
        /// Authority composition the caller expects.
        authority: NotificationAuthorityProjection,
        /// Ledger revision the caller observed.
        expected_ledger_revision: NotificationLedgerRevision,
        /// Replacement content and target identity.
        draft: NotificationDraftProjection,
        /// Whether the replacement re-marks the record unseen.
        mark_unseen: bool,
    },
    /// Marks one notification as seen.
    MarkSeen {
        /// Correlation identity echoed in the result.
        request_id: NotificationRequestId,
        /// Protocol version the caller negotiated.
        protocol_version: NotificationProtocolVersion,
        /// Authority composition the caller expects.
        authority: NotificationAuthorityProjection,
        /// Ledger revision the caller observed.
        expected_ledger_revision: NotificationLedgerRevision,
        /// Notification to mark seen.
        notification_id: NotificationId,
    },
    /// Dismisses one notification.
    Dismiss {
        /// Correlation identity echoed in the result.
        request_id: NotificationRequestId,
        /// Protocol version the caller negotiated.
        protocol_version: NotificationProtocolVersion,
        /// Authority composition the caller expects.
        authority: NotificationAuthorityProjection,
        /// Ledger revision the caller observed.
        expected_ledger_revision: NotificationLedgerRevision,
        /// Notification to dismiss.
        notification_id: NotificationId,
    },
    /// Removes notifications matching an explicit target.
    Clear {
        /// Correlation identity echoed in the result.
        request_id: NotificationRequestId,
        /// Protocol version the caller negotiated.
        protocol_version: NotificationProtocolVersion,
        /// Authority composition the caller expects.
        authority: NotificationAuthorityProjection,
        /// Ledger revision the caller observed.
        expected_ledger_revision: NotificationLedgerRevision,
        /// Which records to remove.
        target: NotificationClearTargetProjection,
    },
    /// Changes ledger retention limits.
    ChangeRetention {
        /// Correlation identity echoed in the result.
        request_id: NotificationRequestId,
        /// Protocol version the caller negotiated.
        protocol_version: NotificationProtocolVersion,
        /// Authority composition the caller expects.
        authority: NotificationAuthorityProjection,
        /// Ledger revision the caller observed.
        expected_ledger_revision: NotificationLedgerRevision,
        /// Requested retention limits.
        limits: NotificationLedgerLimitsProjection,
    },
}

impl NotificationMutationCommand {
    /// Returns request correlation identity.
    #[must_use]
    pub const fn request_id(&self) -> &NotificationRequestId {
        match self {
            Self::Add { request_id, .. }
            | Self::Replace { request_id, .. }
            | Self::MarkSeen { request_id, .. }
            | Self::Dismiss { request_id, .. }
            | Self::Clear { request_id, .. }
            | Self::ChangeRetention { request_id, .. } => request_id,
        }
    }

    pub(crate) const fn protocol_version(&self) -> NotificationProtocolVersion {
        match self {
            Self::Add {
                protocol_version, ..
            }
            | Self::Replace {
                protocol_version, ..
            }
            | Self::MarkSeen {
                protocol_version, ..
            }
            | Self::Dismiss {
                protocol_version, ..
            }
            | Self::Clear {
                protocol_version, ..
            }
            | Self::ChangeRetention {
                protocol_version, ..
            } => *protocol_version,
        }
    }
}

/// Distinct removal reason on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationRemovalReasonProjection {
    /// Explicit single-record dismissal.
    Dismissed,
    /// Explicit clear transition.
    Cleared,
    /// Finite retention prune.
    Pruned,
}

/// Exact removed record on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationRemovalProjection {
    /// Complete removed record.
    pub record: NotificationRecordProjection,
    /// Distinct removal reason.
    pub reason: NotificationRemovalReasonProjection,
}

impl From<&NotificationRemoval> for NotificationRemovalProjection {
    fn from(value: &NotificationRemoval) -> Self {
        Self {
            record: NotificationRecordProjection::from_record(value.record()),
            reason: match value.reason() {
                NotificationRemovalReason::Dismissed => {
                    NotificationRemovalReasonProjection::Dismissed
                }
                NotificationRemovalReason::Cleared => NotificationRemovalReasonProjection::Cleared,
                NotificationRemovalReason::Pruned => NotificationRemovalReasonProjection::Pruned,
            },
        }
    }
}

/// Exact successful mutation receipt on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum NotificationMutationReceiptProjection {
    /// A notification was added.
    Added {
        /// Committed record.
        record: NotificationRecordProjection,
        /// Ledger revision before the mutation.
        previous_ledger_revision: NotificationLedgerRevision,
        /// Ledger revision after the mutation.
        committed_ledger_revision: NotificationLedgerRevision,
        /// Records retention pruned as a side effect.
        pruned: Vec<NotificationRemovalProjection>,
    },
    /// A notification was replaced.
    Replaced {
        /// Committed record.
        record: NotificationRecordProjection,
        /// Ledger revision before the mutation.
        previous_ledger_revision: NotificationLedgerRevision,
        /// Ledger revision after the mutation.
        committed_ledger_revision: NotificationLedgerRevision,
        /// Records retention pruned as a side effect.
        pruned: Vec<NotificationRemovalProjection>,
    },
    /// A notification was marked seen.
    Seen {
        /// Committed record.
        record: NotificationRecordProjection,
        /// Ledger revision before the mutation.
        previous_ledger_revision: NotificationLedgerRevision,
        /// Ledger revision after the mutation.
        committed_ledger_revision: NotificationLedgerRevision,
    },
    /// Notifications were removed by dismissal or clear.
    Removed {
        /// Ledger revision before the mutation.
        previous_ledger_revision: NotificationLedgerRevision,
        /// Ledger revision after the mutation.
        committed_ledger_revision: NotificationLedgerRevision,
        /// Removed records with reasons.
        removals: Vec<NotificationRemovalProjection>,
    },
    /// Retention limits changed.
    RetentionChanged {
        /// Retention limits before the mutation.
        previous_limits: NotificationLedgerLimitsProjection,
        /// Retention limits after the mutation.
        committed_limits: NotificationLedgerLimitsProjection,
        /// Ledger revision before the mutation.
        previous_ledger_revision: NotificationLedgerRevision,
        /// Ledger revision after the mutation.
        committed_ledger_revision: NotificationLedgerRevision,
        /// Records the new limits pruned.
        removals: Vec<NotificationRemovalProjection>,
    },
}

impl NotificationMutationReceiptProjection {
    /// Returns revision before the transition.
    #[must_use]
    pub const fn previous_ledger_revision(&self) -> NotificationLedgerRevision {
        match self {
            Self::Added {
                previous_ledger_revision,
                ..
            }
            | Self::Replaced {
                previous_ledger_revision,
                ..
            }
            | Self::Seen {
                previous_ledger_revision,
                ..
            }
            | Self::Removed {
                previous_ledger_revision,
                ..
            }
            | Self::RetentionChanged {
                previous_ledger_revision,
                ..
            } => *previous_ledger_revision,
        }
    }

    /// Returns revision after the transition.
    #[must_use]
    pub const fn committed_ledger_revision(&self) -> NotificationLedgerRevision {
        match self {
            Self::Added {
                committed_ledger_revision,
                ..
            }
            | Self::Replaced {
                committed_ledger_revision,
                ..
            }
            | Self::Seen {
                committed_ledger_revision,
                ..
            }
            | Self::Removed {
                committed_ledger_revision,
                ..
            }
            | Self::RetentionChanged {
                committed_ledger_revision,
                ..
            } => *committed_ledger_revision,
        }
    }

    /// Returns every directly changed or removed record identity.
    #[must_use]
    pub fn affected_notification_ids(&self) -> Vec<NotificationId> {
        match self {
            Self::Added { record, pruned, .. } | Self::Replaced { record, pruned, .. } => {
                let mut ids = vec![record.notification_id.clone()];
                ids.extend(
                    pruned
                        .iter()
                        .map(|removal| removal.record.notification_id.clone()),
                );
                ids
            }
            Self::Seen { record, .. } => vec![record.notification_id.clone()],
            Self::Removed { removals, .. } | Self::RetentionChanged { removals, .. } => removals
                .iter()
                .map(|removal| removal.record.notification_id.clone())
                .collect(),
        }
    }
}

/// Stable checked mutation rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationRejectionCode {
    /// Command protocol version is not supported.
    IncompatibleProtocol,
    /// Command failed structural validation.
    InvalidCommand,
    /// Authority composition does not match the ledger's.
    AuthorityMismatch,
    /// Expected ledger revision is stale.
    LedgerRevisionMismatch,
    /// Notification identity already exists.
    DuplicateNotification,
    /// Notification identity does not exist.
    UnknownNotification,
    /// Replacement key is already bound to another record.
    DuplicateReplacementKey,
    /// Replacement requires a replacement key the draft lacks.
    MissingReplacementKey,
    /// No record carries the draft's replacement key.
    ReplacementTargetNotFound,
    /// Producer token is already bound to another record.
    DuplicateProducerToken,
    /// Replacement requires a producer token the draft lacks.
    MissingProducerToken,
    /// Notification is already marked seen.
    AlreadySeen,
    /// Clear target lists one notification twice.
    DuplicateClearTarget,
    /// Clear target names an unknown notification.
    ClearTargetNotFound,
    /// Requested retention limits cannot hold the current records.
    RetentionUnsatisfied,
    /// Ledger capacity counter overflowed.
    CapacityOverflow,
}

/// Checked rejection with fresh-snapshot guidance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationRejection {
    /// Stable category.
    pub code: NotificationRejectionCode,
    /// Product-neutral diagnostic.
    pub detail: String,
    /// Whether caller should load fresh authority before retry.
    pub refresh_required: bool,
}

/// Successful or checked-rejected notification mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status",
    deny_unknown_fields
)]
pub enum NotificationMutationResult {
    /// Authority committed the mutation.
    Committed {
        /// Echoed request identity.
        request_id: NotificationRequestId,
        /// Fresh authoritative first page.
        snapshot: NotificationSnapshot,
        /// Exact mutation receipt.
        receipt: Box<NotificationMutationReceiptProjection>,
    },
    /// Authority rejected without mutation.
    Rejected {
        /// Echoed request identity.
        request_id: NotificationRequestId,
        /// Unchanged authoritative first page.
        snapshot: NotificationSnapshot,
        /// Checked rejection.
        rejection: NotificationRejection,
    },
}
