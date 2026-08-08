//! Protocol execution over NotificationLedger.

use longhorn_core::NotificationRequestId;

use crate::{
    NotificationAdd, NotificationClear, NotificationClearTarget, NotificationLedger,
    NotificationLedgerError, NotificationMutationReceipt, NotificationRemovalReceipt,
    NotificationReplace, NotificationRetentionChange, NotificationSeen,
};

use super::*;

impl NotificationLedger {
    /// Executes one strict bounded snapshot query.
    pub fn execute_protocol_snapshot(
        &self,
        query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationProtocolError> {
        if query.protocol_version != NotificationProtocolVersion::CURRENT {
            return Err(NotificationProtocolError::incompatible());
        }
        Ok(NotificationSnapshotResponse {
            request_id: query.request_id,
            snapshot: NotificationSnapshot::from_ledger(self, query.offset, query.limit)?,
        })
    }

    /// Executes one strict mutation and returns fresh first-page authority.
    pub fn execute_protocol_mutation(
        &mut self,
        command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationProtocolError> {
        let request_id = command.request_id().clone();
        if command.protocol_version() != NotificationProtocolVersion::CURRENT {
            return self.rejected_mutation(request_id, incompatible_rejection());
        }
        let result = execute_mutation(self, command);
        match result {
            Ok(receipt) => Ok(NotificationMutationResult::Committed {
                request_id,
                snapshot: NotificationSnapshot::from_ledger(
                    self,
                    0,
                    NOTIFICATION_DEFAULT_PAGE_SIZE,
                )?,
                receipt: Box::new(receipt),
            }),
            Err(rejection) => self.rejected_mutation(request_id, rejection),
        }
    }

    pub(crate) fn rejected_mutation(
        &self,
        request_id: NotificationRequestId,
        rejection: NotificationRejection,
    ) -> Result<NotificationMutationResult, NotificationProtocolError> {
        Ok(NotificationMutationResult::Rejected {
            request_id,
            snapshot: NotificationSnapshot::from_ledger(self, 0, NOTIFICATION_DEFAULT_PAGE_SIZE)?,
            rejection,
        })
    }
}

fn execute_mutation(
    ledger: &mut NotificationLedger,
    command: NotificationMutationCommand,
) -> Result<NotificationMutationReceiptProjection, NotificationRejection> {
    match command {
        NotificationMutationCommand::Add {
            authority,
            expected_ledger_revision,
            notification_id,
            draft,
            ..
        } => {
            let receipt = ledger
                .add(NotificationAdd::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    notification_id,
                    draft.into_draft().map_err(NotificationRejection::from)?,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(project_record_receipt(receipt, true, false))
        }
        NotificationMutationCommand::Replace {
            authority,
            expected_ledger_revision,
            draft,
            mark_unseen,
            ..
        } => {
            let receipt = ledger
                .replace(NotificationReplace::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    draft.into_draft().map_err(NotificationRejection::from)?,
                    mark_unseen,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(project_record_receipt(receipt, false, false))
        }
        NotificationMutationCommand::MarkSeen {
            authority,
            expected_ledger_revision,
            notification_id,
            ..
        } => {
            let receipt = ledger
                .mark_seen(NotificationSeen::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    notification_id,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(project_record_receipt(receipt, false, true))
        }
        NotificationMutationCommand::Dismiss {
            authority,
            expected_ledger_revision,
            notification_id,
            ..
        } => ledger
            .dismiss(
                authority
                    .into_cursor()
                    .map_err(NotificationRejection::from)?,
                expected_ledger_revision,
                notification_id,
            )
            .map(project_removal_receipt)
            .map_err(NotificationRejection::from),
        NotificationMutationCommand::Clear {
            authority,
            expected_ledger_revision,
            target,
            ..
        } => {
            let target = match target {
                NotificationClearTargetProjection::All => NotificationClearTarget::All,
                NotificationClearTargetProjection::Records { notification_ids } => {
                    NotificationClearTarget::Records(notification_ids)
                }
            };
            ledger
                .clear(NotificationClear::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    target,
                ))
                .map(project_removal_receipt)
                .map_err(NotificationRejection::from)
        }
        NotificationMutationCommand::ChangeRetention {
            authority,
            expected_ledger_revision,
            limits,
            ..
        } => {
            let previous_limits = NotificationLedgerLimitsProjection::from_limits(ledger.limits())
                .expect("validated notification limits fit u64");
            let receipt = ledger
                .change_retention(NotificationRetentionChange::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    limits.into_limits().map_err(NotificationRejection::from)?,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(NotificationMutationReceiptProjection::RetentionChanged {
                previous_limits,
                committed_limits: NotificationLedgerLimitsProjection::from_limits(ledger.limits())
                    .expect("validated notification limits fit u64"),
                previous_ledger_revision: receipt.previous_ledger_revision(),
                committed_ledger_revision: receipt.committed_ledger_revision(),
                removals: receipt.removals().iter().map(Into::into).collect(),
            })
        }
    }
}

fn project_record_receipt(
    receipt: NotificationMutationReceipt,
    added: bool,
    seen: bool,
) -> NotificationMutationReceiptProjection {
    let record = NotificationRecordProjection::from_record(receipt.record());
    if seen {
        NotificationMutationReceiptProjection::Seen {
            record,
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
        }
    } else if added {
        NotificationMutationReceiptProjection::Added {
            record,
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
            pruned: receipt.pruned().iter().map(Into::into).collect(),
        }
    } else {
        NotificationMutationReceiptProjection::Replaced {
            record,
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
            pruned: receipt.pruned().iter().map(Into::into).collect(),
        }
    }
}

fn project_removal_receipt(
    receipt: NotificationRemovalReceipt,
) -> NotificationMutationReceiptProjection {
    NotificationMutationReceiptProjection::Removed {
        previous_ledger_revision: receipt.previous_ledger_revision(),
        committed_ledger_revision: receipt.committed_ledger_revision(),
        removals: receipt.removals().iter().map(Into::into).collect(),
    }
}

impl From<NotificationLedgerError> for NotificationRejection {
    fn from(error: NotificationLedgerError) -> Self {
        let code = match error {
            NotificationLedgerError::WrongAuthority { .. } => {
                NotificationRejectionCode::AuthorityMismatch
            }
            NotificationLedgerError::StaleRevision { .. } => {
                NotificationRejectionCode::LedgerRevisionMismatch
            }
            NotificationLedgerError::DuplicateNotification { .. } => {
                NotificationRejectionCode::DuplicateNotification
            }
            NotificationLedgerError::NotificationNotFound { .. } => {
                NotificationRejectionCode::UnknownNotification
            }
            NotificationLedgerError::DuplicateReplacementKey { .. } => {
                NotificationRejectionCode::DuplicateReplacementKey
            }
            NotificationLedgerError::MissingReplacementKey => {
                NotificationRejectionCode::MissingReplacementKey
            }
            NotificationLedgerError::ReplacementTargetNotFound { .. } => {
                NotificationRejectionCode::ReplacementTargetNotFound
            }
            NotificationLedgerError::DuplicateProducerToken { .. } => {
                NotificationRejectionCode::DuplicateProducerToken
            }
            NotificationLedgerError::MissingProducerToken => {
                NotificationRejectionCode::MissingProducerToken
            }
            NotificationLedgerError::AlreadySeen { .. } => NotificationRejectionCode::AlreadySeen,
            NotificationLedgerError::DuplicateClearTarget { .. } => {
                NotificationRejectionCode::DuplicateClearTarget
            }
            NotificationLedgerError::ClearTargetNotFound { .. } => {
                NotificationRejectionCode::ClearTargetNotFound
            }
            NotificationLedgerError::RetentionUnsatisfied { .. } => {
                NotificationRejectionCode::RetentionUnsatisfied
            }
            NotificationLedgerError::TooManyActions { .. }
            | NotificationLedgerError::TooManyClearTargets { .. }
            | NotificationLedgerError::InvalidPageSize(_) => {
                NotificationRejectionCode::InvalidCommand
            }
            NotificationLedgerError::EncodedWeightOverflow
            | NotificationLedgerError::RevisionOverflow
            | NotificationLedgerError::SequenceOverflow
            | NotificationLedgerError::PrunedCountOverflow => {
                NotificationRejectionCode::CapacityOverflow
            }
        };
        let refresh_required = matches!(
            code,
            NotificationRejectionCode::AuthorityMismatch
                | NotificationRejectionCode::LedgerRevisionMismatch
                | NotificationRejectionCode::UnknownNotification
                | NotificationRejectionCode::ReplacementTargetNotFound
                | NotificationRejectionCode::AlreadySeen
                | NotificationRejectionCode::ClearTargetNotFound
        );
        Self {
            code,
            detail: error.to_string(),
            refresh_required,
        }
    }
}

impl From<NotificationProtocolInputError> for NotificationRejection {
    fn from(error: NotificationProtocolInputError) -> Self {
        Self {
            code: NotificationRejectionCode::InvalidCommand,
            detail: error.to_string(),
            refresh_required: false,
        }
    }
}
