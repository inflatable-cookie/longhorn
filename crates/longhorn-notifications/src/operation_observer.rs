use std::{error::Error, fmt};

use longhorn_core::{NotificationId, OperationCatalogueRevision, OperationId, OperationRevision};
use longhorn_operation::{
    OperationCancellationReceipt, OperationRecord, OperationState, OperationTeardownOutcome,
    OperationTeardownReceipt, OperationTransitionReceipt,
};

use crate::{
    NotificationAdd, NotificationDraft, NotificationLedger, NotificationLedgerError,
    NotificationPublishOnce, NotificationPublishOutcome,
};

/// Committed terminal outcome evidence from any operation authority path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationTerminalEvidence<'a> {
    /// Terminal state reached through a direct lifecycle transition.
    Transition(&'a OperationTransitionReceipt),
    /// Terminal state reached through a confirmed queued cancellation.
    Cancellation(&'a OperationCancellationReceipt),
    /// Terminal state reached during controlled authority teardown.
    Teardown(&'a OperationTeardownReceipt),
}

impl<'a> OperationTerminalEvidence<'a> {
    /// Returns the committed catalogue revision of the underlying receipt.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        match self {
            Self::Transition(receipt) => receipt.committed_catalogue_revision(),
            Self::Cancellation(receipt) => receipt.committed_catalogue_revision(),
            Self::Teardown(receipt) => receipt.committed_catalogue_revision(),
        }
    }

    fn resolve_for(
        &self,
        operation_id: &OperationId,
    ) -> Result<(OperationState, OperationRevision), OperationNotificationObservationError> {
        match self {
            Self::Transition(receipt) => {
                if !receipt.committed_state().is_terminal() {
                    return Err(OperationNotificationObservationError::NonTerminalTransition);
                }
                if receipt.operation_id() != operation_id {
                    return Err(OperationNotificationObservationError::OperationMismatch);
                }
                Ok((
                    receipt.committed_state(),
                    receipt.committed_operation_revision(),
                ))
            }
            Self::Cancellation(receipt) => {
                if !receipt.committed_state().is_terminal() {
                    return Err(OperationNotificationObservationError::NonTerminalTransition);
                }
                if receipt.operation_id() != operation_id {
                    return Err(OperationNotificationObservationError::OperationMismatch);
                }
                Ok((
                    receipt.committed_state(),
                    receipt.committed_operation_revision(),
                ))
            }
            Self::Teardown(receipt) => {
                let outcome = receipt
                    .outcomes()
                    .iter()
                    .find(|outcome| outcome.operation_id() == operation_id)
                    .ok_or(OperationNotificationObservationError::OperationMismatch)?;
                match outcome {
                    OperationTeardownOutcome::Completed {
                        state,
                        committed_operation_revision,
                        ..
                    } => {
                        if !state.is_terminal() {
                            return Err(
                                OperationNotificationObservationError::NonTerminalTransition,
                            );
                        }
                        Ok((*state, *committed_operation_revision))
                    }
                    OperationTeardownOutcome::Transferred { .. } => {
                        Err(OperationNotificationObservationError::NonTerminalTransition)
                    }
                }
            }
        }
    }
}

impl<'a> From<&'a OperationTransitionReceipt> for OperationTerminalEvidence<'a> {
    fn from(receipt: &'a OperationTransitionReceipt) -> Self {
        Self::Transition(receipt)
    }
}

impl<'a> From<&'a OperationCancellationReceipt> for OperationTerminalEvidence<'a> {
    fn from(receipt: &'a OperationCancellationReceipt) -> Self {
        Self::Cancellation(receipt)
    }
}

impl<'a> From<&'a OperationTeardownReceipt> for OperationTerminalEvidence<'a> {
    fn from(receipt: &'a OperationTeardownReceipt) -> Self {
        Self::Teardown(receipt)
    }
}

/// Consumer policy output for one terminal operation notification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationNotificationPublication {
    notification_id: NotificationId,
    draft: NotificationDraft,
}

impl OperationNotificationPublication {
    /// Constructs a policy-selected publication.
    #[must_use]
    pub const fn new(notification_id: NotificationId, draft: NotificationDraft) -> Self {
        Self {
            notification_id,
            draft,
        }
    }

    /// Returns the stable notification identity.
    #[must_use]
    pub const fn notification_id(&self) -> &NotificationId {
        &self.notification_id
    }

    /// Returns bounded notification metadata.
    #[must_use]
    pub const fn draft(&self) -> &NotificationDraft {
        &self.draft
    }
}

/// Consumer-owned mapping from committed terminal operation truth.
pub trait OperationNotificationPolicy {
    /// Returns zero or one notification publication for a committed terminal outcome.
    fn publication(
        &self,
        operation: &OperationRecord,
        evidence: OperationTerminalEvidence<'_>,
    ) -> Option<OperationNotificationPublication>;
}

/// Invalid committed operation observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationNotificationObservationError {
    /// The observed outcome is not terminal.
    NonTerminalTransition,
    /// The supplied record and receipt name different operations.
    OperationMismatch,
    /// The supplied record does not reflect the committed receipt state.
    UncommittedRecord,
}

impl fmt::Display for OperationNotificationObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonTerminalTransition => {
                formatter.write_str("operation transition is not terminal")
            }
            Self::OperationMismatch => {
                formatter.write_str("operation record and receipt identities differ")
            }
            Self::UncommittedRecord => {
                formatter.write_str("operation record does not reflect committed receipt")
            }
        }
    }
}

impl Error for OperationNotificationObservationError {}

/// Observes an already-committed terminal outcome from transition, queued
/// cancellation, or teardown without mutating either authority.
pub fn observe_terminal_operation<'a, P, E>(
    policy: &P,
    operation: &OperationRecord,
    evidence: E,
) -> Result<Option<OperationNotificationPublication>, OperationNotificationObservationError>
where
    P: OperationNotificationPolicy,
    E: Into<OperationTerminalEvidence<'a>>,
{
    let evidence = evidence.into();
    let (committed_state, committed_revision) = evidence.resolve_for(operation.operation_id())?;
    if operation.state() != committed_state || operation.revision() != committed_revision {
        return Err(OperationNotificationObservationError::UncommittedRecord);
    }
    Ok(policy.publication(operation, evidence))
}

/// Publishes one observation idempotently against current ledger state.
///
/// Operation state is absent from the mutable input, so publication failure
/// cannot alter the committed terminal outcome.
pub fn publish_operation_notification(
    ledger: &mut NotificationLedger,
    publication: OperationNotificationPublication,
) -> Result<NotificationPublishOutcome, NotificationLedgerError> {
    let request = NotificationAdd::new(
        ledger.authority().clone(),
        ledger.revision(),
        publication.notification_id,
        publication.draft,
    );
    ledger.publish_once(NotificationPublishOnce::new(request))
}
