//! Internal ledger validation and pruning helpers.

use std::collections::HashSet;

use longhorn_core::{
    NotificationId, NotificationLedgerRevision, NotificationProducerToken,
    NotificationReplacementKey, NotificationSourceId,
};

use crate::{
    NotificationAuthorityCursor, NotificationLedgerError, NotificationLedgerLimits,
    NotificationReadState, NotificationRecord, NotificationRemoval, NotificationRemovalReason,
    NotificationRetentionClass,
};

use super::NotificationLedger;

impl NotificationLedger {
    pub(crate) fn validate_authority(
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

    pub(crate) fn validate_revision(
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

    pub(crate) fn validate_draft_uniqueness(
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

    pub(crate) fn replacement_index(
        &self,
        source_id: &NotificationSourceId,
        replacement_key: &NotificationReplacementKey,
    ) -> Option<usize> {
        self.records.iter().position(|record| {
            record.draft().source_id() == source_id
                && record.draft().replacement_key() == Some(replacement_key)
        })
    }

    pub(crate) fn record_by_producer_token(
        &self,
        producer_token: &NotificationProducerToken,
    ) -> Option<&NotificationRecord> {
        self.records
            .iter()
            .find(|record| record.draft().producer_token() == Some(producer_token))
    }

    pub(crate) fn record_index(
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

    pub(crate) fn next_revision(
        &self,
    ) -> Result<NotificationLedgerRevision, NotificationLedgerError> {
        self.revision
            .checked_next()
            .map_err(|_| NotificationLedgerError::RevisionOverflow)
    }

    pub(crate) fn remember_producer_token(
        &mut self,
        token: NotificationProducerToken,
        notification_id: NotificationId,
    ) {
        if let Some(existing) = self.remembered_producer_token_ids.get_mut(&token) {
            *existing = notification_id;
            return;
        }
        if self.remembered_producer_tokens.len() >= crate::MAXIMUM_REMEMBERED_PRODUCER_TOKENS
            && let Some(evicted) = self.remembered_producer_tokens.pop_front()
        {
            self.remembered_producer_token_ids.remove(&evicted);
        }
        self.remembered_producer_tokens.push_back(token.clone());
        self.remembered_producer_token_ids
            .insert(token, notification_id);
    }
}

pub(crate) fn unseen_count(records: &[NotificationRecord]) -> usize {
    records
        .iter()
        .filter(|record| record.read_state() == NotificationReadState::Unseen)
        .count()
}

pub(crate) fn encoded_weight(
    records: &[NotificationRecord],
) -> Result<u64, NotificationLedgerError> {
    records.iter().try_fold(0_u64, |total, record| {
        total
            .checked_add(record.encoded_metadata_weight())
            .ok_or(NotificationLedgerError::EncodedWeightOverflow)
    })
}

pub(crate) fn prune_to_limits(
    records: &mut Vec<NotificationRecord>,
    limits: NotificationLedgerLimits,
    excluded_id: Option<&NotificationId>,
) -> Result<Vec<NotificationRemoval>, NotificationLedgerError> {
    let mut retained_count = records.len();
    let mut retained_weight = encoded_weight(records)?;
    if retained_count <= limits.maximum_notifications()
        && retained_weight <= limits.maximum_encoded_weight()
    {
        return Ok(Vec::new());
    }

    let mut prune_flags = vec![false; records.len()];
    let mut prune_count = 0usize;
    for (index, record) in records.iter().enumerate() {
        if retained_count <= limits.maximum_notifications()
            && retained_weight <= limits.maximum_encoded_weight()
        {
            break;
        }
        if Some(record.notification_id()) != excluded_id
            && record.draft().retention_class() == NotificationRetentionClass::Standard
        {
            prune_flags[index] = true;
            prune_count += 1;
            retained_count -= 1;
            retained_weight -= record.encoded_metadata_weight();
        }
    }

    if retained_count > limits.maximum_notifications()
        || retained_weight > limits.maximum_encoded_weight()
    {
        return Err(NotificationLedgerError::RetentionUnsatisfied {
            maximum_count: limits.maximum_notifications(),
            retained_count,
            maximum_encoded_weight: limits.maximum_encoded_weight(),
            retained_encoded_weight: retained_weight,
        });
    }

    let mut removals = Vec::with_capacity(prune_count);
    let mut kept = Vec::with_capacity(records.len() - prune_count);
    for (record, prune) in records.drain(..).zip(prune_flags) {
        if prune {
            removals.push(NotificationRemoval::new(
                record,
                NotificationRemovalReason::Pruned,
            ));
        } else {
            kept.push(record);
        }
    }
    *records = kept;
    Ok(removals)
}

pub(crate) fn increment_pruned_count(
    current: u64,
    removals: usize,
) -> Result<u64, NotificationLedgerError> {
    let removals =
        u64::try_from(removals).map_err(|_| NotificationLedgerError::PrunedCountOverflow)?;
    current
        .checked_add(removals)
        .ok_or(NotificationLedgerError::PrunedCountOverflow)
}

pub(crate) fn validate_clear_targets(
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
