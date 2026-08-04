use std::collections::HashSet;

use longhorn_core::{
    NotificationAuthorityId, NotificationId, NotificationLedgerRevision, NotificationProducerToken,
    NotificationReplacementKey, NotificationSourceId,
};

use crate::{
    NotificationAdd, NotificationAuthorityCursor, NotificationAuthorityEpoch, NotificationClear,
    NotificationClearTarget, NotificationLedgerError, NotificationLedgerLimits,
    NotificationLedgerProjection, NotificationMutationReceipt, NotificationPage,
    NotificationPublishOnce, NotificationPublishOutcome, NotificationReadState, NotificationRecord,
    NotificationRemoval, NotificationRemovalReason, NotificationRemovalReceipt,
    NotificationReplace, NotificationRetentionChange, NotificationRetentionClass, NotificationSeen,
    NotificationSequence,
};

/// Validated mutable authority for one finite retained notification ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationLedger {
    authority: NotificationAuthorityCursor,
    revision: NotificationLedgerRevision,
    next_sequence: NotificationSequence,
    limits: NotificationLedgerLimits,
    records: Vec<NotificationRecord>,
    pruned_count: u64,
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

    /// Adds a new record. Text never participates in deduplication.
    pub fn add(
        &mut self,
        request: NotificationAdd,
    ) -> Result<NotificationMutationReceipt, NotificationLedgerError> {
        self.validate_authority(&request.authority)?;
        self.validate_revision(request.expected_revision)?;
        if self.record(&request.notification_id).is_some() {
            return Err(NotificationLedgerError::DuplicateNotification {
                notification_id: request.notification_id,
            });
        }
        self.validate_draft_uniqueness(&request.draft, None)?;

        let committed_revision = self.next_revision()?;
        let next_sequence = self
            .next_sequence
            .checked_next()
            .map_err(|_| NotificationLedgerError::SequenceOverflow)?;
        let admitted_id = request.notification_id.clone();
        let record = NotificationRecord::added(
            request.notification_id,
            request.draft,
            self.next_sequence,
            committed_revision,
        );
        let mut records = self.records.clone();
        records.push(record);
        let pruned = prune_to_limits(&mut records, self.limits, Some(&admitted_id))?;
        let pruned_count = increment_pruned_count(self.pruned_count, pruned.len())?;
        let committed_record = records
            .iter()
            .find(|record| record.notification_id() == &admitted_id)
            .expect("new record is protected from its admission prune")
            .clone();

        let receipt = NotificationMutationReceipt {
            record: committed_record,
            previous_ledger_revision: self.revision,
            committed_ledger_revision: committed_revision,
            pruned,
        };
        self.records = records;
        self.revision = committed_revision;
        self.next_sequence = next_sequence;
        self.pruned_count = pruned_count;
        Ok(receipt)
    }

    /// Publishes at most once by the draft's durable producer token.
    pub fn publish_once(
        &mut self,
        request: NotificationPublishOnce,
    ) -> Result<NotificationPublishOutcome, NotificationLedgerError> {
        self.validate_authority(&request.add.authority)?;
        let token = request
            .add
            .draft
            .producer_token()
            .ok_or(NotificationLedgerError::MissingProducerToken)?;
        if let Some(record) = self.record_by_producer_token(token) {
            return Ok(NotificationPublishOutcome::AlreadyPublished {
                record: record.clone(),
                ledger_revision: self.revision,
            });
        }
        self.add(request.add)
            .map(NotificationPublishOutcome::Published)
    }

    /// Replaces exactly one record by source and replacement key.
    pub fn replace(
        &mut self,
        request: NotificationReplace,
    ) -> Result<NotificationMutationReceipt, NotificationLedgerError> {
        self.validate_authority(&request.authority)?;
        self.validate_revision(request.expected_revision)?;
        let replacement_key = request
            .draft
            .replacement_key()
            .ok_or(NotificationLedgerError::MissingReplacementKey)?;
        let index = self
            .replacement_index(request.draft.source_id(), replacement_key)
            .ok_or_else(|| NotificationLedgerError::ReplacementTargetNotFound {
                source_id: request.draft.source_id().clone(),
                replacement_key: replacement_key.clone(),
            })?;
        let notification_id = self.records[index].notification_id().clone();
        self.validate_draft_uniqueness(&request.draft, Some(&notification_id))?;
        let committed_revision = self.next_revision()?;
        let mut records = self.records.clone();
        records[index].replace(request.draft, committed_revision, request.mark_unseen);
        let pruned = prune_to_limits(&mut records, self.limits, Some(&notification_id))?;
        let pruned_count = increment_pruned_count(self.pruned_count, pruned.len())?;
        let committed_record = records
            .iter()
            .find(|record| record.notification_id() == &notification_id)
            .expect("replaced record is protected from its replacement prune")
            .clone();
        let receipt = NotificationMutationReceipt {
            record: committed_record,
            previous_ledger_revision: self.revision,
            committed_ledger_revision: committed_revision,
            pruned,
        };
        self.records = records;
        self.revision = committed_revision;
        self.pruned_count = pruned_count;
        Ok(receipt)
    }

    /// Marks one unseen retained record seen without removing it.
    pub fn mark_seen(
        &mut self,
        request: NotificationSeen,
    ) -> Result<NotificationMutationReceipt, NotificationLedgerError> {
        self.validate_authority(&request.authority)?;
        self.validate_revision(request.expected_revision)?;
        let index = self.record_index(&request.notification_id)?;
        if self.records[index].read_state() == NotificationReadState::Seen {
            return Err(NotificationLedgerError::AlreadySeen {
                notification_id: request.notification_id,
            });
        }
        let committed_revision = self.next_revision()?;
        let previous_revision = self.revision;
        self.records[index].mark_seen(committed_revision);
        self.revision = committed_revision;
        Ok(NotificationMutationReceipt {
            record: self.records[index].clone(),
            previous_ledger_revision: previous_revision,
            committed_ledger_revision: committed_revision,
            pruned: Vec::new(),
        })
    }

    /// Explicitly dismisses one retained record.
    pub fn dismiss(
        &mut self,
        authority: NotificationAuthorityCursor,
        expected_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
    ) -> Result<NotificationRemovalReceipt, NotificationLedgerError> {
        self.validate_authority(&authority)?;
        self.validate_revision(expected_revision)?;
        let index = self.record_index(&notification_id)?;
        let committed_revision = self.next_revision()?;
        let record = self.records.remove(index);
        let receipt = NotificationRemovalReceipt {
            previous_ledger_revision: self.revision,
            committed_ledger_revision: committed_revision,
            removals: vec![NotificationRemoval::new(
                record,
                NotificationRemovalReason::Dismissed,
            )],
        };
        self.revision = committed_revision;
        Ok(receipt)
    }

    /// Clears all records or an exact bounded identity set.
    pub fn clear(
        &mut self,
        request: NotificationClear,
    ) -> Result<NotificationRemovalReceipt, NotificationLedgerError> {
        self.validate_authority(&request.authority)?;
        self.validate_revision(request.expected_revision)?;
        let ids = match request.target {
            NotificationClearTarget::All => self
                .records
                .iter()
                .map(|record| record.notification_id().clone())
                .collect(),
            NotificationClearTarget::Records(ids) => {
                validate_clear_targets(&self.records, &ids)?;
                ids
            }
        };
        if ids.is_empty() {
            return Ok(NotificationRemovalReceipt {
                previous_ledger_revision: self.revision,
                committed_ledger_revision: self.revision,
                removals: Vec::new(),
            });
        }
        let committed_revision = self.next_revision()?;
        let id_set: HashSet<_> = ids.into_iter().collect();
        let mut removals = Vec::with_capacity(id_set.len());
        self.records.retain(|record| {
            if id_set.contains(record.notification_id()) {
                removals.push(NotificationRemoval::new(
                    record.clone(),
                    NotificationRemovalReason::Cleared,
                ));
                false
            } else {
                true
            }
        });
        let receipt = NotificationRemovalReceipt {
            previous_ledger_revision: self.revision,
            committed_ledger_revision: committed_revision,
            removals,
        };
        self.revision = committed_revision;
        Ok(receipt)
    }

    /// Changes finite limits and prunes every reported standard record.
    pub fn change_retention(
        &mut self,
        request: NotificationRetentionChange,
    ) -> Result<NotificationRemovalReceipt, NotificationLedgerError> {
        self.validate_authority(&request.authority)?;
        self.validate_revision(request.expected_revision)?;
        let committed_revision = self.next_revision()?;
        let mut records = self.records.clone();
        let removals = prune_to_limits(&mut records, request.limits, None)?;
        let pruned_count = increment_pruned_count(self.pruned_count, removals.len())?;
        let receipt = NotificationRemovalReceipt {
            previous_ledger_revision: self.revision,
            committed_ledger_revision: committed_revision,
            removals,
        };
        self.records = records;
        self.revision = committed_revision;
        self.limits = request.limits;
        self.pruned_count = pruned_count;
        Ok(receipt)
    }

    fn validate_authority(
        &self,
        actual: &NotificationAuthorityCursor,
    ) -> Result<(), NotificationLedgerError> {
        if actual == &self.authority {
            Ok(())
        } else {
            Err(NotificationLedgerError::WrongAuthority {
                expected: self.authority.clone(),
                actual: actual.clone(),
            })
        }
    }

    fn validate_revision(
        &self,
        actual: NotificationLedgerRevision,
    ) -> Result<(), NotificationLedgerError> {
        if actual == self.revision {
            Ok(())
        } else {
            Err(NotificationLedgerError::StaleRevision {
                expected: self.revision,
                actual,
            })
        }
    }

    fn validate_draft_uniqueness(
        &self,
        draft: &crate::NotificationDraft,
        excluding: Option<&NotificationId>,
    ) -> Result<(), NotificationLedgerError> {
        if let Some(key) = draft.replacement_key()
            && self.records.iter().any(|record| {
                Some(record.notification_id()) != excluding
                    && record.draft().source_id() == draft.source_id()
                    && record.draft().replacement_key() == Some(key)
            })
        {
            return Err(NotificationLedgerError::DuplicateReplacementKey {
                source_id: draft.source_id().clone(),
                replacement_key: key.clone(),
            });
        }
        if let Some(token) = draft.producer_token()
            && self.records.iter().any(|record| {
                Some(record.notification_id()) != excluding
                    && record.draft().producer_token() == Some(token)
            })
        {
            return Err(NotificationLedgerError::DuplicateProducerToken {
                producer_token: token.clone(),
            });
        }
        Ok(())
    }

    fn replacement_index(
        &self,
        source_id: &NotificationSourceId,
        replacement_key: &NotificationReplacementKey,
    ) -> Option<usize> {
        self.records.iter().position(|record| {
            record.draft().source_id() == source_id
                && record.draft().replacement_key() == Some(replacement_key)
        })
    }

    fn record_by_producer_token(
        &self,
        producer_token: &NotificationProducerToken,
    ) -> Option<&NotificationRecord> {
        self.records
            .iter()
            .find(|record| record.draft().producer_token() == Some(producer_token))
    }

    fn record_index(
        &self,
        notification_id: &NotificationId,
    ) -> Result<usize, NotificationLedgerError> {
        self.records
            .iter()
            .position(|record| record.notification_id() == notification_id)
            .ok_or_else(|| NotificationLedgerError::NotificationNotFound {
                notification_id: notification_id.clone(),
            })
    }

    fn next_revision(&self) -> Result<NotificationLedgerRevision, NotificationLedgerError> {
        self.revision
            .checked_next()
            .map_err(|_| NotificationLedgerError::RevisionOverflow)
    }
}

fn unseen_count(records: &[NotificationRecord]) -> usize {
    records
        .iter()
        .filter(|record| record.read_state() == NotificationReadState::Unseen)
        .count()
}

fn encoded_weight(records: &[NotificationRecord]) -> Result<u64, NotificationLedgerError> {
    records.iter().try_fold(0_u64, |total, record| {
        total
            .checked_add(record.encoded_metadata_weight())
            .ok_or(NotificationLedgerError::EncodedWeightOverflow)
    })
}

fn prune_to_limits(
    records: &mut Vec<NotificationRecord>,
    limits: NotificationLedgerLimits,
    excluded_id: Option<&NotificationId>,
) -> Result<Vec<NotificationRemoval>, NotificationLedgerError> {
    let mut removals = Vec::new();
    loop {
        let weight = encoded_weight(records)?;
        if records.len() <= limits.maximum_notifications()
            && weight <= limits.maximum_encoded_weight()
        {
            return Ok(removals);
        }
        let candidate = records.iter().position(|record| {
            Some(record.notification_id()) != excluded_id
                && record.draft().retention_class() == NotificationRetentionClass::Standard
        });
        let Some(index) = candidate else {
            return Err(NotificationLedgerError::RetentionUnsatisfied {
                maximum_count: limits.maximum_notifications(),
                retained_count: records.len(),
                maximum_encoded_weight: limits.maximum_encoded_weight(),
                retained_encoded_weight: weight,
            });
        };
        removals.push(NotificationRemoval::new(
            records.remove(index),
            NotificationRemovalReason::Pruned,
        ));
    }
}

fn increment_pruned_count(current: u64, removals: usize) -> Result<u64, NotificationLedgerError> {
    let removals =
        u64::try_from(removals).map_err(|_| NotificationLedgerError::PrunedCountOverflow)?;
    current
        .checked_add(removals)
        .ok_or(NotificationLedgerError::PrunedCountOverflow)
}

fn validate_clear_targets(
    records: &[NotificationRecord],
    ids: &[NotificationId],
) -> Result<(), NotificationLedgerError> {
    if ids.len() > crate::MAXIMUM_RETAINED_NOTIFICATIONS {
        return Err(NotificationLedgerError::TooManyClearTargets {
            maximum: crate::MAXIMUM_RETAINED_NOTIFICATIONS,
            actual: ids.len(),
        });
    }
    let mut seen = HashSet::with_capacity(ids.len());
    for notification_id in ids {
        if !seen.insert(notification_id) {
            return Err(NotificationLedgerError::DuplicateClearTarget {
                notification_id: notification_id.clone(),
            });
        }
        if !records
            .iter()
            .any(|record| record.notification_id() == notification_id)
        {
            return Err(NotificationLedgerError::ClearTargetNotFound {
                notification_id: notification_id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use longhorn_core::{
        NotificationAuthorityId, NotificationId, NotificationLedgerRevision, NotificationSourceId,
    };

    use super::*;
    use crate::{NotificationDraft, NotificationSeverity, NotificationSummary, NotificationTitle};

    fn ledger(maximum: usize) -> NotificationLedger {
        NotificationLedger::new(
            NotificationAuthorityId::new("notifications:overflow").unwrap(),
            NotificationAuthorityEpoch::new(1).unwrap(),
            NotificationLedgerLimits::new(maximum, 1 << 20).unwrap(),
        )
    }

    fn request(ledger: &NotificationLedger, suffix: &str) -> NotificationAdd {
        NotificationAdd::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationId::new(format!("notification:{suffix}")).unwrap(),
            NotificationDraft::new(
                NotificationSourceId::new("source:test").unwrap(),
                NotificationSeverity::Info,
                NotificationTitle::new(suffix).unwrap(),
                NotificationSummary::new(suffix).unwrap(),
            ),
        )
    }

    #[test]
    fn revision_sequence_and_prune_count_overflow_reject_atomically() {
        let mut revision = ledger(2);
        revision.revision = NotificationLedgerRevision::new(u64::MAX);
        let before = revision.clone();
        assert_eq!(
            revision.add(request(&revision, "revision")),
            Err(NotificationLedgerError::RevisionOverflow)
        );
        assert_eq!(revision, before);

        let mut sequence = ledger(2);
        sequence.next_sequence = NotificationSequence::for_test(u64::MAX);
        let before = sequence.clone();
        assert_eq!(
            sequence.add(request(&sequence, "sequence")),
            Err(NotificationLedgerError::SequenceOverflow)
        );
        assert_eq!(sequence, before);

        let mut pruned = ledger(1);
        pruned.add(request(&pruned, "oldest")).unwrap();
        pruned.pruned_count = u64::MAX;
        let before = pruned.clone();
        assert_eq!(
            pruned.add(request(&pruned, "newest")),
            Err(NotificationLedgerError::PrunedCountOverflow)
        );
        assert_eq!(pruned, before);
    }
}
