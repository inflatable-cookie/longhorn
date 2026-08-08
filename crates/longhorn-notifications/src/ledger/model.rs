//! Validated mutable authority for one finite retained notification ledger.

use longhorn_core::{NotificationAuthorityId, NotificationId, NotificationLedgerRevision};

use crate::{
    NotificationAuthorityCursor, NotificationAuthorityEpoch, NotificationLedgerError,
    NotificationLedgerLimits, NotificationLedgerProjection, NotificationPage, NotificationRecord,
    NotificationSequence,
};

use super::{encoded_weight, unseen_count};
/// Validated mutable authority for one finite retained notification ledger.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationLedger {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) revision: NotificationLedgerRevision,
    pub(crate) next_sequence: NotificationSequence,
    pub(crate) limits: NotificationLedgerLimits,
    pub(crate) records: Vec<NotificationRecord>,
    pub(crate) pruned_count: u64,
}

impl NotificationLedger {
    /// Constructs an empty ledger with explicit identity, epoch, and limits.
    #[must_use]
    pub const fn new(
        authority_id: NotificationAuthorityId,
        authority_epoch: NotificationAuthorityEpoch,
        limits: NotificationLedgerLimits,
    ) -> Self {
        Self {
            authority: NotificationAuthorityCursor::new(authority_id, authority_epoch),
            revision: NotificationLedgerRevision::INITIAL,
            next_sequence: NotificationSequence::FIRST,
            limits,
            records: Vec::new(),
            pruned_count: 0,
        }
    }

    /// Returns exact live authority identity and epoch.
    #[must_use]
    pub const fn authority(&self) -> &NotificationAuthorityCursor {
        &self.authority
    }

    /// Returns current authoritative ledger revision.
    #[must_use]
    pub const fn revision(&self) -> NotificationLedgerRevision {
        self.revision
    }

    /// Returns current finite retention limits.
    #[must_use]
    pub const fn limits(&self) -> NotificationLedgerLimits {
        self.limits
    }

    /// Returns records in oldest-first insertion order.
    pub fn records(&self) -> impl ExactSizeIterator<Item = &NotificationRecord> {
        self.records.iter()
    }

    /// Returns one retained record.
    #[must_use]
    pub fn record(&self, notification_id: &NotificationId) -> Option<&NotificationRecord> {
        self.records
            .iter()
            .find(|record| record.notification_id() == notification_id)
    }

    /// Returns a bounded authoritative ledger summary.
    pub fn projection(&self) -> Result<NotificationLedgerProjection, NotificationLedgerError> {
        Ok(NotificationLedgerProjection {
            authority: self.authority.clone(),
            ledger_revision: self.revision,
            retained_count: self.records.len(),
            unseen_count: unseen_count(&self.records),
            retained_encoded_weight: encoded_weight(&self.records)?,
            pruned_count: self.pruned_count,
            limits: self.limits,
        })
    }

    /// Returns a bounded newest-first page and ledger-wide truncation evidence.
    pub fn page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<NotificationPage, NotificationLedgerError> {
        crate::limits::validate_page_size(limit)?;
        let records = self
            .records
            .iter()
            .rev()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        Ok(NotificationPage {
            authority: self.authority.clone(),
            ledger_revision: self.revision,
            offset,
            total_count: self.records.len(),
            unseen_count: unseen_count(&self.records),
            records,
        })
    }
}
