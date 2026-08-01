use std::{error::Error, fmt};

use longhorn_core::NotificationId;
use longhorn_operation::{OperationRecord, OperationTransitionReceipt};

use crate::{
    NotificationAdd, NotificationDraft, NotificationLedger, NotificationLedgerError,
    NotificationPublishOnce, NotificationPublishOutcome,
};

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
    /// Returns zero or one notification publication for a terminal transition.
    fn publication(
        &self,
        operation: &OperationRecord,
        receipt: &OperationTransitionReceipt,
    ) -> Option<OperationNotificationPublication>;
}

/// Invalid committed operation observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationNotificationObservationError {
    /// The transition receipt is not terminal.
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

/// Observes an already-committed terminal transition without mutating either authority.
pub fn observe_terminal_operation<P: OperationNotificationPolicy>(
    policy: &P,
    operation: &OperationRecord,
    receipt: &OperationTransitionReceipt,
) -> Result<Option<OperationNotificationPublication>, OperationNotificationObservationError> {
    if !receipt.committed_state().is_terminal() {
        return Err(OperationNotificationObservationError::NonTerminalTransition);
    }
    if operation.operation_id() != receipt.operation_id() {
        return Err(OperationNotificationObservationError::OperationMismatch);
    }
    if operation.state() != receipt.committed_state()
        || operation.revision() != receipt.committed_operation_revision()
    {
        return Err(OperationNotificationObservationError::UncommittedRecord);
    }
    Ok(policy.publication(operation, receipt))
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
