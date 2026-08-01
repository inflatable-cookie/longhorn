use longhorn_core::{NotificationId, NotificationLedgerRevision};

use crate::NotificationRecord;

/// Why a retained notification left the ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationRemovalReason {
    /// One record was explicitly dismissed.
    Dismissed,
    /// Records were removed by an explicit clear request.
    Cleared,
    /// Standard records were removed to enforce finite retention.
    Pruned,
}

/// Exact retained record removal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationRemoval {
    record: NotificationRecord,
    reason: NotificationRemovalReason,
}

impl NotificationRemoval {
    pub(crate) const fn new(record: NotificationRecord, reason: NotificationRemovalReason) -> Self {
        Self { record, reason }
    }

    /// Returns removed record identity.
    #[must_use]
    pub const fn notification_id(&self) -> &NotificationId {
        self.record.notification_id()
    }

    /// Returns the complete removed record.
    #[must_use]
    pub const fn record(&self) -> &NotificationRecord {
        &self.record
    }

    /// Returns the distinct removal reason.
    #[must_use]
    pub const fn reason(&self) -> NotificationRemovalReason {
        self.reason
    }
}

/// Receipt for an add, replace, or mark-seen mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationMutationReceipt {
    pub(crate) record: NotificationRecord,
    pub(crate) previous_ledger_revision: NotificationLedgerRevision,
    pub(crate) committed_ledger_revision: NotificationLedgerRevision,
    pub(crate) pruned: Vec<NotificationRemoval>,
}

impl NotificationMutationReceipt {
    /// Returns the committed retained record.
    #[must_use]
    pub const fn record(&self) -> &NotificationRecord {
        &self.record
    }
    /// Returns revision before mutation.
    #[must_use]
    pub const fn previous_ledger_revision(&self) -> NotificationLedgerRevision {
        self.previous_ledger_revision
    }
    /// Returns committed revision.
    #[must_use]
    pub const fn committed_ledger_revision(&self) -> NotificationLedgerRevision {
        self.committed_ledger_revision
    }
    /// Returns every automatic retention removal caused by the mutation.
    #[must_use]
    pub fn pruned(&self) -> &[NotificationRemoval] {
        &self.pruned
    }
}

/// Receipt for dismiss, clear, or explicit prune transitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationRemovalReceipt {
    pub(crate) previous_ledger_revision: NotificationLedgerRevision,
    pub(crate) committed_ledger_revision: NotificationLedgerRevision,
    pub(crate) removals: Vec<NotificationRemoval>,
}

impl NotificationRemovalReceipt {
    /// Returns revision before mutation.
    #[must_use]
    pub const fn previous_ledger_revision(&self) -> NotificationLedgerRevision {
        self.previous_ledger_revision
    }
    /// Returns committed revision.
    #[must_use]
    pub const fn committed_ledger_revision(&self) -> NotificationLedgerRevision {
        self.committed_ledger_revision
    }
    /// Returns every exact removal.
    #[must_use]
    pub fn removals(&self) -> &[NotificationRemoval] {
        &self.removals
    }
}
