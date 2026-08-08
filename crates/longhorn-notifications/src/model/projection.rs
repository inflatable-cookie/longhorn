//! Ledger projection and paged views.

use longhorn_core::NotificationLedgerRevision;

use crate::NotificationLedgerLimits;

use super::{NotificationAuthorityCursor, NotificationRecord};
/// Lightweight authoritative ledger summary.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationLedgerProjection {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) ledger_revision: NotificationLedgerRevision,
    pub(crate) retained_count: usize,
    pub(crate) unseen_count: usize,
    pub(crate) retained_encoded_weight: u64,
    pub(crate) pruned_count: u64,
    pub(crate) limits: NotificationLedgerLimits,
}

impl NotificationLedgerProjection {
    /// Returns authority identity and epoch.
    #[must_use]
    pub const fn authority(&self) -> &NotificationAuthorityCursor {
        &self.authority
    }
    /// Returns authoritative revision.
    #[must_use]
    pub const fn ledger_revision(&self) -> NotificationLedgerRevision {
        self.ledger_revision
    }
    /// Returns retained record count.
    #[must_use]
    pub const fn retained_count(&self) -> usize {
        self.retained_count
    }
    /// Returns exact unseen record count.
    #[must_use]
    pub const fn unseen_count(&self) -> usize {
        self.unseen_count
    }
    /// Returns retained canonical encoded weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }
    /// Returns cumulative automatic prune count.
    #[must_use]
    pub const fn pruned_count(&self) -> u64 {
        self.pruned_count
    }
    /// Returns current finite limits.
    #[must_use]
    pub const fn limits(&self) -> NotificationLedgerLimits {
        self.limits
    }
}

/// Bounded newest-first record page with truncation evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationPage {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) ledger_revision: NotificationLedgerRevision,
    pub(crate) offset: usize,
    pub(crate) total_count: usize,
    pub(crate) unseen_count: usize,
    pub(crate) records: Vec<NotificationRecord>,
}

impl NotificationPage {
    /// Returns authority identity and epoch.
    #[must_use]
    pub const fn authority(&self) -> &NotificationAuthorityCursor {
        &self.authority
    }
    /// Returns authoritative revision.
    #[must_use]
    pub const fn ledger_revision(&self) -> NotificationLedgerRevision {
        self.ledger_revision
    }
    /// Returns requested newest-first offset.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }
    /// Returns total retained count.
    #[must_use]
    pub const fn total_count(&self) -> usize {
        self.total_count
    }
    /// Returns exact ledger-wide unseen count.
    #[must_use]
    pub const fn unseen_count(&self) -> usize {
        self.unseen_count
    }
    /// Returns newest-first records in this page.
    #[must_use]
    pub fn records(&self) -> &[NotificationRecord] {
        &self.records
    }
    /// Returns whether older retained records were omitted.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.offset.saturating_add(self.records.len()) < self.total_count
    }
}
