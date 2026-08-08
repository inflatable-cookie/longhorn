//! Ledger mutation operations.

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

use super::{
    NotificationLedger, encoded_weight, increment_pruned_count, prune_to_limits, unseen_count,
    validate_clear_targets,
};

impl NotificationLedger {
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
}
