use longhorn_core::OperationId;
use longhorn_operation::{
    OperationCancellationCommand, OperationCancellationResult, OperationMutationCommand,
    OperationMutationResult, OperationSnapshotQuery, OperationSnapshotResponse,
};

use crate::OperationHostError;

/// Consumer-injected caller authorization and catalogue authority.
pub trait OperationHostAuthority: Send {
    /// Returns one caller-authorized payload-free snapshot.
    fn snapshot(
        &mut self,
        caller: &str,
        query: OperationSnapshotQuery,
    ) -> Result<OperationSnapshotResponse, OperationHostError>;

    /// Applies one caller-authorized management mutation.
    fn mutate(
        &mut self,
        caller: &str,
        command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationHostError>;

    /// Admits one caller-authorized cancellation request.
    fn cancel(
        &mut self,
        caller: &str,
        command: OperationCancellationCommand,
    ) -> Result<OperationCancellationResult, OperationHostError>;
}

/// Consumer-injected executor cancellation boundary.
pub trait OperationExecutorPort: Send {
    /// Requests cancellation after catalogue authority has committed admission.
    fn request_cancellation(
        &mut self,
        operation_id: &OperationId,
    ) -> Result<(), OperationExecutorError>;
}

/// Stable failure returned by the injected executor boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationExecutorError {
    /// Stable adapter-specific code.
    pub code: String,
    /// Product-neutral diagnostic.
    pub message: String,
    /// Whether an explicit later dispatch may succeed.
    pub retryable: bool,
}

impl OperationExecutorError {
    /// Constructs executor dispatch evidence.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }
}
