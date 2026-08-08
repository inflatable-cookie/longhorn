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
