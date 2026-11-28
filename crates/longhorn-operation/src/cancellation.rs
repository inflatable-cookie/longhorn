use longhorn_core::{OperationCatalogueRevision, OperationId, OperationRevision};

use crate::{OperationAuthorityCursor, OperationRecord, OperationRemoval, OperationState};

/// Whether a consumer executor accepts cancellation requests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationCancellationSupport {
    /// The consumer can act on cancellation requests.
    Supported,
    /// Cancellation is not supported for this operation.
    Unsupported,
}

/// Whether consumer admission confirms a queued operation never started executing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum QueuedCancellationConfirmation {
    /// Execution start is not confirmed absent; accepted queued work enters
    /// [`OperationState::Cancelling`] and waits for executor confirmation.
    #[default]
    Unconfirmed,
    /// Consumer admission confirms execution never started, permitting
    /// immediate terminal [`OperationState::Cancelled`] when still queued.
    ConfirmedUnstarted,
}

/// Revision-bound cancellation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCancellationRequest {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) operation_id: OperationId,
    pub(crate) expected_operation_revision: OperationRevision,
    pub(crate) queued_confirmation: QueuedCancellationConfirmation,
}

impl OperationCancellationRequest {
    /// Constructs a cancellation request without confirming unstarted queued admission.
    #[must_use]
    pub const fn new(
        authority: OperationAuthorityCursor,
        operation_id: OperationId,
        expected_operation_revision: OperationRevision,
    ) -> Self {
        Self {
            authority,
            operation_id,
            expected_operation_revision,
            queued_confirmation: QueuedCancellationConfirmation::Unconfirmed,
        }
    }

    /// Sets whether consumer admission confirmed that queued work never started.
    #[must_use]
    pub const fn with_queued_confirmation(
        mut self,
        confirmation: QueuedCancellationConfirmation,
    ) -> Self {
        self.queued_confirmation = confirmation;
        self
    }

    /// Returns the queued-execution confirmation supplied with this request.
    #[must_use]
    pub const fn queued_confirmation(&self) -> QueuedCancellationConfirmation {
        self.queued_confirmation
    }
}

/// Checked cancellation admission result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationCancellationOutcome {
    /// Cancellation was newly accepted.
    Accepted,
    /// The operation is already awaiting its executor terminal fact.
    AlreadyRequested,
    /// This operation does not support cancellation.
    Unsupported,
    /// The operation already has a sticky terminal outcome.
    Terminal,
}

/// Receipt for cancellation admission. Acceptance is not executor completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCancellationReceipt {
    pub(crate) operation: OperationRecord,
    pub(crate) outcome: OperationCancellationOutcome,
    pub(crate) previous_state: OperationState,
    pub(crate) committed_state: OperationState,
    pub(crate) previous_operation_revision: OperationRevision,
    pub(crate) committed_operation_revision: OperationRevision,
    pub(crate) previous_catalogue_revision: OperationCatalogueRevision,
    pub(crate) committed_catalogue_revision: OperationCatalogueRevision,
    pub(crate) evicted: Vec<OperationRemoval>,
}

impl OperationCancellationReceipt {
    /// Returns the authoritative post-admission operation record, preserved
    /// even if terminal retention evicted it during the same commit.
    #[must_use]
    pub const fn operation(&self) -> &OperationRecord {
        &self.operation
    }

    /// Returns the target operation.
    #[must_use]
    pub const fn operation_id(&self) -> &OperationId {
        self.operation.operation_id()
    }

    /// Returns cancellation admission outcome.
    #[must_use]
    pub const fn outcome(&self) -> OperationCancellationOutcome {
        self.outcome
    }

    /// Returns state before the request.
    #[must_use]
    pub const fn previous_state(&self) -> OperationState {
        self.previous_state
    }

    /// Returns state after the request.
    #[must_use]
    pub const fn committed_state(&self) -> OperationState {
        self.committed_state
    }

    /// Returns the operation revision before the request.
    #[must_use]
    pub const fn previous_operation_revision(&self) -> OperationRevision {
        self.previous_operation_revision
    }

    /// Returns the operation revision after the request.
    #[must_use]
    pub const fn committed_operation_revision(&self) -> OperationRevision {
        self.committed_operation_revision
    }

    /// Returns the catalogue revision before the request.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.previous_catalogue_revision
    }

    /// Returns the catalogue revision after the request.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.committed_catalogue_revision
    }

    /// Returns terminal records evicted by an accepted queued cancellation.
    #[must_use]
    pub fn evicted(&self) -> &[OperationRemoval] {
        &self.evicted
    }
}
