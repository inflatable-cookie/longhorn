use longhorn_agent_tool_dispatch as longhorn;
use std::future::poll_fn;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;
use swallowtail_host_local::{
    LocalHostServices, LocalProcessHost, LocalProcessLimits, OperationBridgeCleanupCause,
};
use swallowtail_runtime::{
    AdmissionPhase, BoxFuture, CleanupOutcome, RegisteredToolCall, RegisteredToolCallId,
    RegisteredToolCallRequest, RegisteredToolCancellation, RegisteredToolDispatchContext,
    RegisteredToolDispatcher, RegisteredToolExecutionDisposition, RegisteredToolFailure,
    RegisteredToolFailureKind, RegisteredToolOutcome, RegisteredToolPayload, RegisteredToolResult,
    RuntimeFailure,
};
use swallowtail_testkit::{
    FIXTURE_CLEANUP_BUDGET, FIXTURE_NATIVE_TOOL, FakeClock, RegisteredToolHarness,
    conformance_deadline, conformance_host_id, conformance_turn, drive_fixture, fixture_media_type,
    fixture_payload, fixture_tool_id, poll_fixture_once,
};

struct BridgeCancellation(RegisteredToolCancellation);

impl longhorn::CancellationSource for BridgeCancellation {
    fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

#[derive(Default)]
struct CallbackState {
    invocations: AtomicUsize,
    cancellation_observations: AtomicUsize,
    retained_cancel_result: Mutex<Option<longhorn::ResultRecorder>>,
}

#[derive(Clone)]
struct CallbackPort(Arc<CallbackState>);

impl longhorn::RecordingCallback for CallbackPort {
    fn invoke(
        &self,
        call: longhorn::AdmittedCall,
        cancellation: longhorn::Cancellation,
        result: longhorn::ResultRecorder,
    ) -> longhorn::DispatchFuture<'_, Result<(), longhorn::CallbackFailure>> {
        self.0.invocations.fetch_add(1, Ordering::SeqCst);
        if call.call_id() != "call-cancel" {
            return Box::pin(async move {
                result
                    .record(longhorn::CallbackResult::new(
                        longhorn_schema("1", "sha256:output"),
                        b"bounded-result".to_vec(),
                    ))
                    .map_err(|_| longhorn::CallbackFailure::new())
            });
        }

        *self
            .0
            .retained_cancel_result
            .lock()
            .expect("retained result lock") = Some(result.clone());
        let state = Arc::clone(&self.0);
        Box::pin(async move {
            poll_fn(|context| {
                if !cancellation.is_cancelled() {
                    context.waker().wake_by_ref();
                    return Poll::Pending;
                }
                state
                    .cancellation_observations
                    .fetch_add(1, Ordering::SeqCst);
                let rejection = result.record(longhorn::CallbackResult::new(
                    longhorn_schema("1", "sha256:output"),
                    b"cancelled-result".to_vec(),
                ));
                assert_eq!(
                    rejection.expect_err("cancellation must win").kind(),
                    longhorn::DispatchErrorKind::Cancelled
                );
                Poll::Ready(())
            })
            .await;
            Ok(())
        })
    }
}

type LonghornDispatcher = longhorn::RecordingDispatcher<CallbackPort>;

struct SwallowtailAdapter {
    inner: Mutex<Option<Arc<LonghornDispatcher>>>,
    registration: longhorn::RegisteredCapability,
    callbacks: Arc<CallbackState>,
    dispatches: AtomicUsize,
}

impl SwallowtailAdapter {
    fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            registration: longhorn_registration(),
            callbacks: Arc::new(CallbackState::default()),
            dispatches: AtomicUsize::new(0),
        }
    }

    fn bind(&self, binding: longhorn::TrustedBinding) {
        let dispatcher = Arc::new(longhorn::RecordingDispatcher::new(
            self.registration.clone(),
            binding,
            CallbackPort(Arc::clone(&self.callbacks)),
        ));
        let replaced = self
            .inner
            .lock()
            .expect("dispatcher binding lock")
            .replace(dispatcher);
        assert!(replaced.is_none(), "fixture binds one operation only");
    }

    fn inner(&self) -> Arc<LonghornDispatcher> {
        Arc::clone(
            self.inner
                .lock()
                .expect("dispatcher binding lock")
                .as_ref()
                .expect("lease binding installed before dispatch"),
        )
    }
}

impl RegisteredToolDispatcher for SwallowtailAdapter {
    fn dispatch(
        &self,
        call: RegisteredToolCall,
        context: RegisteredToolDispatchContext,
    ) -> BoxFuture<'_, Result<RegisteredToolOutcome, RuntimeFailure>> {
        self.dispatches.fetch_add(1, Ordering::SeqCst);
        let dispatcher = self.inner();
        let invocation = normalize_call(&call, self.registration.clone());
        let cancellation = longhorn::Cancellation::new(Arc::new(BridgeCancellation(
            context.cancellation().clone(),
        )));
        Box::pin(async move {
            match dispatcher.dispatch(invocation, cancellation).await {
                Ok(outcome) => {
                    let payload = RegisteredToolPayload::new(
                        fixture_media_type(),
                        outcome.result().payload_for_delivery().to_vec(),
                        call.binding().effective_bounds().max_result_bytes(),
                    )
                    .map_err(RegisteredToolFailure::into_runtime_failure)?;
                    let digest = swallowtail_runtime::RegisteredToolSchemaDigest::new(
                        outcome.result().schema().digest(),
                    )
                    .map_err(RegisteredToolFailure::into_runtime_failure)?;
                    Ok(RegisteredToolOutcome::completed(
                        &call,
                        RegisteredToolResult::new(payload, digest),
                    ))
                }
                Err(error) => Ok(RegisteredToolOutcome::failed(
                    &call,
                    RegisteredToolFailure::new(swallowtail_failure(error.kind())),
                    execution_disposition(error.kind()),
                )),
            }
        })
    }
}

fn swallowtail_failure(kind: longhorn::DispatchErrorKind) -> RegisteredToolFailureKind {
    match kind {
        longhorn::DispatchErrorKind::UnsupportedSchemaVersion
        | longhorn::DispatchErrorKind::UnknownSchema => {
            RegisteredToolFailureKind::UnsupportedSchema
        }
        longhorn::DispatchErrorKind::StaleRegistration
        | longhorn::DispatchErrorKind::StaleBinding => RegisteredToolFailureKind::StaleCorrelation,
        longhorn::DispatchErrorKind::UnknownCapability => {
            RegisteredToolFailureKind::UnsupportedTool
        }
        longhorn::DispatchErrorKind::ForeignBinding => {
            RegisteredToolFailureKind::ForeignCorrelation
        }
        longhorn::DispatchErrorKind::RevokedBinding => RegisteredToolFailureKind::Revoked,
        longhorn::DispatchErrorKind::DuplicateCall
        | longhorn::DispatchErrorKind::DuplicateResult => {
            RegisteredToolFailureKind::DuplicateCorrelation
        }
        longhorn::DispatchErrorKind::PostTerminalResult => {
            RegisteredToolFailureKind::PostTerminalCorrelation
        }
        longhorn::DispatchErrorKind::LimitExceeded | longhorn::DispatchErrorKind::InvalidBounds => {
            RegisteredToolFailureKind::LimitExceeded
        }
        longhorn::DispatchErrorKind::Cancelled => RegisteredToolFailureKind::Cancelled,
        longhorn::DispatchErrorKind::DeadlineExceeded => {
            RegisteredToolFailureKind::DeadlineExceeded
        }
        longhorn::DispatchErrorKind::InvalidIdentity => RegisteredToolFailureKind::IdentityRejected,
        longhorn::DispatchErrorKind::ExecutionKindMismatch
        | longhorn::DispatchErrorKind::EffectMismatch
        | longhorn::DispatchErrorKind::MissingResult
        | longhorn::DispatchErrorKind::CallbackFailed => {
            RegisteredToolFailureKind::ServerExecutionFailed
        }
    }
}

fn execution_disposition(kind: longhorn::DispatchErrorKind) -> RegisteredToolExecutionDisposition {
    match kind {
        longhorn::DispatchErrorKind::StaleRegistration
        | longhorn::DispatchErrorKind::UnknownCapability
        | longhorn::DispatchErrorKind::UnsupportedSchemaVersion
        | longhorn::DispatchErrorKind::UnknownSchema
        | longhorn::DispatchErrorKind::ExecutionKindMismatch
        | longhorn::DispatchErrorKind::EffectMismatch
        | longhorn::DispatchErrorKind::ForeignBinding
        | longhorn::DispatchErrorKind::StaleBinding
        | longhorn::DispatchErrorKind::RevokedBinding
        | longhorn::DispatchErrorKind::InvalidIdentity
        | longhorn::DispatchErrorKind::InvalidBounds
        | longhorn::DispatchErrorKind::LimitExceeded
        | longhorn::DispatchErrorKind::DeadlineExceeded
        | longhorn::DispatchErrorKind::DuplicateCall => {
            RegisteredToolExecutionDisposition::NotExecuted
        }
        longhorn::DispatchErrorKind::DuplicateResult
        | longhorn::DispatchErrorKind::PostTerminalResult
        | longhorn::DispatchErrorKind::Cancelled
        | longhorn::DispatchErrorKind::MissingResult
        | longhorn::DispatchErrorKind::CallbackFailed => {
            RegisteredToolExecutionDisposition::Unknown
        }
    }
}

fn normalize_call(
    call: &RegisteredToolCall,
    registration: longhorn::RegisteredCapability,
) -> longhorn::DispatchInvocation {
    let admission = call.binding().admission();
    let binding = longhorn::TrustedBinding::new(
        admission.incarnation().as_host_value(),
        admission.workspace_generation().get(),
        admission.task_generation().get(),
        call.binding().lease_generation().get(),
        admission.task().as_host_value(),
        admission.session().as_host_value(),
        admission.attempt().as_host_value(),
    )
    .expect("Swallowtail admitted a valid binding");
    longhorn::DispatchInvocation::new(
        call.call_id().as_str(),
        registration,
        binding,
        longhorn::BindingStatus::Current,
        longhorn::DispatchDeadline::active(Duration::from_nanos(call.deadline().instant().ticks())),
        call.arguments().expose_for_execution().to_vec(),
    )
    .expect("Swallowtail admitted a valid call")
}

fn longhorn_schema(version: &str, digest: &str) -> longhorn::SchemaIdentity {
    longhorn::SchemaIdentity::new(version, digest).expect("fixture schema identity")
}

fn longhorn_bounds() -> longhorn::DispatchBounds {
    let bounds = swallowtail_runtime::RegisteredToolBounds::ceiling();
    longhorn::DispatchBounds::new(
        bounds.max_outstanding_calls(),
        bounds.max_argument_bytes(),
        bounds.max_result_bytes(),
        bounds.max_progress_item_bytes(),
        bounds.max_queued_progress_items(),
        bounds.max_call_duration(),
    )
    .expect("Swallowtail fixture bounds are positive")
}

fn longhorn_registration() -> longhorn::RegisteredCapability {
    longhorn::RegisteredCapability::new(
        "2026-09-07.1",
        "swallowtail.conformance/echo",
        longhorn_schema("1", "sha256:input"),
        longhorn_schema("1", "sha256:output"),
        longhorn::ExecutionKind::NativeClient,
        longhorn::DispatchEffect::Mutating,
        longhorn_bounds(),
    )
    .expect("fixture registration is valid")
}

fn longhorn_binding(operation_generation: u64) -> longhorn::TrustedBinding {
    longhorn::TrustedBinding::new(
        "incarnation-1",
        1,
        1,
        operation_generation,
        "task-1",
        "session-1",
        "attempt-1",
    )
    .expect("fixture binding is valid")
}

fn compose(adapter: Arc<SwallowtailAdapter>) -> LocalHostServices {
    LocalProcessHost::builder(LocalProcessLimits::default())
        .with_registered_tool_dispatcher(adapter)
        .with_registered_tool_cleanup_budget(FIXTURE_CLEANUP_BUDGET)
        .with_registered_tool_clock(Arc::new(FakeClock::default()))
        .build_services(conformance_host_id())
}

fn request(call_id: &str, tool: &str) -> RegisteredToolCallRequest {
    RegisteredToolCallRequest::new(
        RegisteredToolCallId::new(call_id).expect("fixture call id"),
        fixture_tool_id(tool),
        fixture_payload(8, 1024),
        conformance_deadline(),
    )
}

fn successful_and_refused_calls() {
    let adapter = Arc::new(SwallowtailAdapter::new());
    let services = compose(Arc::clone(&adapter));
    let harness = RegisteredToolHarness::with_clock(
        services.services().clone(),
        Arc::new(FakeClock::default()),
    );
    assert!(
        harness
            .preparation
            .prepare(
                &harness.hosts,
                swallowtail_testkit::conformance_instance(),
                swallowtail_testkit::conformance_scope(),
                conformance_turn("turn-success"),
                conformance_deadline(),
            )
            .expect("mounted preparation is ready")
            .readiness()
            .is_ready()
    );
    let lease = harness.open("turn-success");
    adapter.bind(longhorn_binding(lease.generation().get()));

    let unknown = drive_fixture(lease.call(request("call-unknown", "missing")))
        .expect_err("unknown tool must fail before dispatch");
    assert_eq!(
        unknown.diagnostic().code(),
        RegisteredToolFailureKind::UnsupportedTool.code()
    );
    assert_eq!(adapter.dispatches.load(Ordering::SeqCst), 0);

    let outcome = drive_fixture(lease.call(request("call-success", FIXTURE_NATIVE_TOOL)))
        .expect("valid call settles");
    assert!(outcome.result().is_some());
    let duplicate = drive_fixture(lease.call(request("call-success", FIXTURE_NATIVE_TOOL)))
        .expect_err("duplicate call must fail before dispatch");
    assert_eq!(
        duplicate.diagnostic().code(),
        RegisteredToolFailureKind::DuplicateCorrelation.code()
    );
    assert_eq!(adapter.dispatches.load(Ordering::SeqCst), 1);
    let metrics = adapter.inner().metrics();
    assert_eq!(metrics.callbacks, 1);
    assert_eq!(metrics.results, 1);
    assert_eq!(metrics.mutation_replays, 0);

    let port = services
        .services()
        .registered_tool_bridge()
        .expect("registered bridge mounted")
        .clone();
    assert_eq!(
        drive_fixture(port.close(
            lease,
            swallowtail_runtime::RegisteredToolCleanupCause::Completion,
        ))
        .expect("clean close"),
        CleanupOutcome::Clean
    );
}

fn revoked_call_never_reaches_adapter() {
    let adapter = Arc::new(SwallowtailAdapter::new());
    let services = compose(Arc::clone(&adapter));
    let harness = RegisteredToolHarness::with_clock(
        services.services().clone(),
        Arc::new(FakeClock::default()),
    );
    let lease = harness.open("turn-revoked");
    adapter.bind(longhorn_binding(lease.generation().get()));
    harness
        .admission
        .revoke_from(AdmissionPhase::BeforeDispatch);

    let error = drive_fixture(lease.call(request("call-revoked", FIXTURE_NATIVE_TOOL)))
        .expect_err("revoked binding must fail before dispatch");
    assert_eq!(
        error.diagnostic().code(),
        RegisteredToolFailureKind::Revoked.code()
    );
    assert_eq!(adapter.dispatches.load(Ordering::SeqCst), 0);
    assert_eq!(adapter.inner().metrics().callbacks, 0);
}

fn cancelled_call_is_terminal_once() {
    let adapter = Arc::new(SwallowtailAdapter::new());
    let services = compose(Arc::clone(&adapter));
    let harness = RegisteredToolHarness::with_clock(
        services.services().clone(),
        Arc::new(FakeClock::default()),
    );
    let lease = harness.open("turn-cancel");
    adapter.bind(longhorn_binding(lease.generation().get()));
    let mut pending = lease.call(request("call-cancel", FIXTURE_NATIVE_TOOL));
    assert!(matches!(poll_fixture_once(&mut pending), Poll::Pending));

    let closer = services.clone();
    let turn = conformance_turn("turn-cancel");
    let close = std::thread::spawn(move || {
        closer.close_operation_bridges(&turn, OperationBridgeCleanupCause::Cancellation)
    });
    let outer = drive_fixture(pending).expect("adapter returns a typed terminal outcome");
    assert!(outer.failure().is_some());
    assert_eq!(
        close
            .join()
            .expect("close thread joins")
            .expect("close succeeds"),
        CleanupOutcome::Clean
    );

    let inner = adapter.inner();
    let metrics = inner.metrics();
    assert_eq!(metrics.callbacks, 1);
    assert_eq!(metrics.cancellations, 1);
    assert_eq!(metrics.results, 0);
    assert_eq!(metrics.mutation_replays, 0);
    assert_eq!(
        adapter
            .callbacks
            .cancellation_observations
            .load(Ordering::SeqCst),
        1
    );
    let late = adapter
        .callbacks
        .retained_cancel_result
        .lock()
        .expect("retained result lock")
        .as_ref()
        .expect("cancel callback retained result")
        .record(longhorn::CallbackResult::new(
            longhorn_schema("1", "sha256:output"),
            b"late".to_vec(),
        ))
        .expect_err("post-terminal result must fail");
    assert_eq!(late.kind(), longhorn::DispatchErrorKind::PostTerminalResult);
}

fn main() {
    successful_and_refused_calls();
    revoked_call_never_reaches_adapter();
    cancelled_call_is_terminal_once();
    println!(
        "provider-free-fixture:pass readiness=ready callbacks=once results=once cancellation=once replay=none redaction=safe"
    );
}
