//! Mutation commands, receipts, and results.

use longhorn_core::{NotificationId, NotificationLedgerRevision, NotificationRequestId};
use serde::{Deserialize, Serialize};

use crate::{NotificationRemoval, NotificationRemovalReason};

use super::*;

/// Revision-bound notification mutation command.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum NotificationMutationCommand {
    Add {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
        draft: NotificationDraftProjection,
    },
    Replace {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        draft: NotificationDraftProjection,
        mark_unseen: bool,
    },
    MarkSeen {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
    },
    Dismiss {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
    },
    Clear {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        target: NotificationClearTargetProjection,
    },
    ChangeRetention {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
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
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum NotificationMutationReceiptProjection {
    Added {
        record: NotificationRecordProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
        pruned: Vec<NotificationRemovalProjection>,
    },
    Replaced {
        record: NotificationRecordProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
        pruned: Vec<NotificationRemovalProjection>,
    },
    Seen {
        record: NotificationRecordProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
    },
    Removed {
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
        removals: Vec<NotificationRemovalProjection>,
    },
    RetentionChanged {
        previous_limits: NotificationLedgerLimitsProjection,
        committed_limits: NotificationLedgerLimitsProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
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
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationRejectionCode {
    IncompatibleProtocol,
    InvalidCommand,
    AuthorityMismatch,
    LedgerRevisionMismatch,
    DuplicateNotification,
    UnknownNotification,
    DuplicateReplacementKey,
    MissingReplacementKey,
    ReplacementTargetNotFound,
    DuplicateProducerToken,
    MissingProducerToken,
    AlreadySeen,
    DuplicateClearTarget,
    ClearTargetNotFound,
    RetentionUnsatisfied,
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
