use std::sync::Mutex;

use longhorn_operation::{
    OperationCancellationCommand, OperationCancellationResult, OperationExecutorDispatchProjection,
    OperationMutationCommand, OperationMutationResult, OperationSnapshotQuery,
    OperationSnapshotResponse,
};

use crate::{OperationExecutorPort, OperationHostAuthority, OperationHostError};

/// Shared operation assembly used by Tauri and conformance tests.
pub struct OperationHandlerAssembly {
    authority: Mutex<Box<dyn OperationHostAuthority>>,
    executor: Mutex<Box<dyn OperationExecutorPort>>,
}

impl OperationHandlerAssembly {
    /// Binds explicitly injected authority and executor ports.
    #[must_use]
    pub fn new(
        authority: impl OperationHostAuthority + 'static,
        executor: impl OperationExecutorPort + 'static,
    ) -> Self {
        Self {
            authority: Mutex::new(Box::new(authority)),
            executor: Mutex::new(Box::new(executor)),
        }
    }

    /// Returns a caller-authorized snapshot.
    pub fn snapshot(
        &self,
        caller: &str,
        query: OperationSnapshotQuery,
    ) -> Result<OperationSnapshotResponse, OperationHostError> {
        self.authority
            .lock()
            .map_err(|_| OperationHostError::authority_state_unavailable())?
            .snapshot(caller, query)
    }

    /// Applies a caller-authorized management mutation.
    pub fn mutate(
        &self,
        caller: &str,
        command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationHostError> {
        self.authority
            .lock()
            .map_err(|_| OperationHostError::authority_state_unavailable())?
            .mutate(caller, command)
    }

    /// Admits cancellation and dispatches through the injected executor.
    pub fn cancel(
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
            Err(_) => {
                let error = OperationHostError::executor_state_unavailable();
                OperationExecutorDispatchProjection::Failed {
                    code: error.code.as_str().to_owned(),
                    message: error.message,
                    retryable: error.retryable,
                }
            }
        };
        Ok(result.with_executor_dispatch(dispatch))
    }
}
