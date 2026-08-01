use std::sync::Mutex;

use longhorn_operation::{
    OperationCancellationCommand, OperationCancellationResult, OperationExecutorDispatchProjection,
    OperationMutationCommand, OperationMutationResult, OperationSnapshotQuery,
    OperationSnapshotResponse,
};

use crate::{
    OperationExecutorPort, OperationHostAuthority, OperationHostError, OperationHostService,
};

/// Shared operation assembly used by Tauri and conformance tests.
pub struct OperationHandlerAssembly<A, E> {
    authority: Mutex<A>,
    executor: Mutex<E>,
}

impl<A, E> OperationHandlerAssembly<A, E> {
    /// Binds explicitly injected authority and executor ports.
    #[must_use]
    pub const fn new(authority: A, executor: E) -> Self {
        Self {
            authority: Mutex::new(authority),
            executor: Mutex::new(executor),
        }
    }
}

impl<A, E> OperationHostService for OperationHandlerAssembly<A, E>
where
    A: OperationHostAuthority,
    E: OperationExecutorPort,
{
    fn snapshot(
        &self,
        caller: &str,
        query: OperationSnapshotQuery,
    ) -> Result<OperationSnapshotResponse, OperationHostError> {
        self.authority
            .lock()
            .map_err(|_| OperationHostError::authority_state_unavailable())?
            .snapshot(caller, query)
    }

    fn mutate(
        &self,
        caller: &str,
        command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationHostError> {
        self.authority
            .lock()
            .map_err(|_| OperationHostError::authority_state_unavailable())?
            .mutate(caller, command)
    }

    fn cancel(
        &self,
        caller: &str,
        command: OperationCancellationCommand,
    ) -> Result<OperationCancellationResult, OperationHostError> {
        let result = self
            .authority
            .lock()
            .map_err(|_| OperationHostError::authority_state_unavailable())?
            .cancel(caller, command)?;
        let Some(operation_id) = result.executor_dispatch_operation_id().cloned() else {
            return Ok(result);
        };
        let dispatch = match self.executor.lock() {
            Ok(mut executor) => match executor.request_cancellation(&operation_id) {
                Ok(()) => OperationExecutorDispatchProjection::Requested,
                Err(error) => OperationExecutorDispatchProjection::Failed {
                    code: error.code,
                    message: error.message,
                    retryable: error.retryable,
                },
            },
            Err(_) => OperationExecutorDispatchProjection::Failed {
                code: "executorStateUnavailable".into(),
                message: OperationHostError::executor_state_unavailable().message,
                retryable: true,
            },
        };
        Ok(result.with_executor_dispatch(dispatch))
    }
}
