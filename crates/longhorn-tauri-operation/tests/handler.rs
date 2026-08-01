//! Injected operation authority and executor assembly tests.

use std::sync::{Arc, Mutex};

use longhorn_core::OperationId;
use longhorn_operation::{
    OperationCancellationCommand, OperationCancellationResult, OperationExecutorDispatchProjection,
    OperationMutationCommand, OperationMutationResult, OperationSnapshotQuery,
    OperationSnapshotResponse,
};
use longhorn_tauri_operation::{
    OperationExecutorError, OperationExecutorPort, OperationHandlerAssembly,
    OperationHostAuthority, OperationHostError, OperationHostService,
    operation_cancellation_changed_event, operation_mutation_changed_event,
};
use serde_json::Value;

#[derive(Clone)]
struct FixtureAuthority {
    fixture: Value,
    callers: Arc<Mutex<Vec<String>>>,
}

impl OperationHostAuthority for FixtureAuthority {
    fn snapshot(
        &mut self,
        caller: &str,
        query: OperationSnapshotQuery,
    ) -> Result<OperationSnapshotResponse, OperationHostError> {
        self.callers.lock().unwrap().push(caller.into());
        let mut response: OperationSnapshotResponse =
            serde_json::from_value(self.fixture["snapshotResponse"].clone()).unwrap();
        response.request_id = query.request_id;
        Ok(response)
    }

    fn mutate(
        &mut self,
        caller: &str,
        command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationHostError> {
        self.callers.lock().unwrap().push(caller.into());
        let request_id = command.request_id();
        self.fixture["mutationResults"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|value| {
                let result: OperationMutationResult =
                    serde_json::from_value(value.clone()).unwrap();
                match &result {
                    OperationMutationResult::Committed {
                        request_id: candidate,
                        ..
                    }
                    | OperationMutationResult::Rejected {
                        request_id: candidate,
                        ..
                    } if candidate == request_id => Some(result),
                    _ => None,
                }
            })
            .ok_or_else(|| OperationHostError::authority("missing fixture result", false))
    }

    fn cancel(
        &mut self,
        caller: &str,
        _command: OperationCancellationCommand,
    ) -> Result<OperationCancellationResult, OperationHostError> {
        self.callers.lock().unwrap().push(caller.into());
        serde_json::from_value(self.fixture["cancellationResult"].clone())
            .map_err(|error| OperationHostError::authority(error.to_string(), false))
    }
}

struct RecordingExecutor {
    requests: Arc<Mutex<Vec<OperationId>>>,
    fail: bool,
}

impl OperationExecutorPort for RecordingExecutor {
    fn request_cancellation(
        &mut self,
        operation_id: &OperationId,
    ) -> Result<(), OperationExecutorError> {
        self.requests.lock().unwrap().push(operation_id.clone());
        if self.fail {
            Err(OperationExecutorError::new(
                "offline",
                "executor offline",
                true,
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn assembly_preserves_caller_and_dispatches_after_authority_commit() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/operation/protocol-v1.json")).unwrap();
    let callers = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let assembly = OperationHandlerAssembly::new(
        FixtureAuthority {
            fixture: fixture.clone(),
            callers: Arc::clone(&callers),
        },
        RecordingExecutor {
            requests: Arc::clone(&requests),
            fail: false,
        },
    );
    let command: OperationCancellationCommand =
        serde_json::from_value(fixture["cancellationCommand"].clone()).unwrap();
    let result = assembly.cancel("operations", command).unwrap();
    assert_eq!(callers.lock().unwrap().as_slice(), ["operations"]);
    assert_eq!(requests.lock().unwrap().len(), 1);
    let OperationCancellationResult::Committed {
        executor_dispatch, ..
    } = &result
    else {
        panic!("fixture cancellation must commit");
    };
    assert_eq!(
        *executor_dispatch,
        OperationExecutorDispatchProjection::Requested
    );
    assert!(operation_cancellation_changed_event(&result).is_some());
}

#[test]
fn executor_failure_is_visible_without_disguising_committed_authority() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/operation/protocol-v1.json")).unwrap();
    let assembly = OperationHandlerAssembly::new(
        FixtureAuthority {
            fixture: fixture.clone(),
            callers: Arc::new(Mutex::new(Vec::new())),
        },
        RecordingExecutor {
            requests: Arc::new(Mutex::new(Vec::new())),
            fail: true,
        },
    );
    let command = serde_json::from_value(fixture["cancellationCommand"].clone()).unwrap();
    let result = assembly.cancel("main", command).unwrap();
    let OperationCancellationResult::Committed {
        executor_dispatch: OperationExecutorDispatchProjection::Failed { code, .. },
        ..
    } = result
    else {
        panic!("authority commit must survive executor failure");
    };
    assert_eq!(code, "offline");
}

#[test]
fn mutation_events_only_project_committed_revision_changes() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/operation/protocol-v1.json")).unwrap();
    let committed: OperationMutationResult =
        serde_json::from_value(fixture["mutationResults"][0].clone()).unwrap();
    let rejected: OperationMutationResult =
        serde_json::from_value(fixture["mutationResults"][3].clone()).unwrap();
    assert!(operation_mutation_changed_event(&committed).is_some());
    assert!(operation_mutation_changed_event(&rejected).is_none());
}
