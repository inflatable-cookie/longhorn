//! Contract 023 recording-dispatcher acceptance fixtures.

use longhorn_agent_tool_dispatch::{
    AdmittedCall, BindingStatus, CallbackFailure, CallbackResult, Cancellation, CancellationSource,
    DispatchBounds, DispatchDeadline, DispatchEffect, DispatchErrorKind, DispatchEventKind,
    DispatchFuture, DispatchInvocation, ExecutionKind, RecordingCallback, RecordingDispatcher,
    RegisteredCapability, ResultRecorder, SchemaIdentity, TrustedBinding,
};
use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

fn schema(version: &str, digest: &str) -> SchemaIdentity {
    SchemaIdentity::new(version, digest).expect("fixture schema identity")
}

fn bounds(result_bytes: usize) -> DispatchBounds {
    DispatchBounds::new(1, 64, result_bytes, 16, 2, Duration::from_secs(5)).expect("fixture bounds")
}

fn registration(
    revision: &str,
    input: SchemaIdentity,
    output: SchemaIdentity,
    kind: ExecutionKind,
) -> RegisteredCapability {
    RegisteredCapability::new(
        revision,
        "desktop/read_context",
        input,
        output,
        kind,
        DispatchEffect::Mutating,
        bounds(64),
    )
    .expect("fixture registration")
}

fn expected_registration() -> RegisteredCapability {
    registration(
        "registration-7",
        schema("1", "sha256:input"),
        schema("1", "sha256:output"),
        ExecutionKind::NativeClient,
    )
}

fn binding(
    incarnation: &str,
    workspace_generation: u64,
    task_generation: u64,
    operation_generation: u64,
) -> TrustedBinding {
    TrustedBinding::new(
        incarnation,
        workspace_generation,
        task_generation,
        operation_generation,
        "task-secret",
        "session-secret",
        "attempt-secret",
    )
    .expect("fixture binding")
}

fn expected_binding() -> TrustedBinding {
    binding("process-secret", 3, 5, 7)
}

fn invocation(
    call_id: &str,
    registration: RegisteredCapability,
    binding: TrustedBinding,
    status: BindingStatus,
) -> DispatchInvocation {
    DispatchInvocation::new(
        call_id,
        registration,
        binding,
        status,
        DispatchDeadline::active(Duration::from_secs(1)),
        b"argument-secret".to_vec(),
    )
    .expect("fixture invocation")
}

#[derive(Default)]
struct Flag(AtomicBool);

impl Flag {
    fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl CancellationSource for Flag {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

fn live_cancellation() -> (Arc<Flag>, Cancellation) {
    let flag = Arc::new(Flag::default());
    (Arc::clone(&flag), Cancellation::new(flag))
}

struct CompletingCallback {
    calls: Arc<AtomicUsize>,
    payload: Vec<u8>,
}

impl RecordingCallback for CompletingCallback {
    fn invoke(
        &self,
        _call: AdmittedCall,
        _cancellation: Cancellation,
        result: ResultRecorder,
    ) -> DispatchFuture<'_, Result<(), CallbackFailure>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let payload = self.payload.clone();
        Box::pin(ready(
            result
                .record(CallbackResult::new(schema("1", "sha256:output"), payload))
                .map_err(|_| CallbackFailure::new()),
        ))
    }
}

fn dispatcher(calls: Arc<AtomicUsize>) -> RecordingDispatcher<CompletingCallback> {
    RecordingDispatcher::new(
        expected_registration(),
        expected_binding(),
        CompletingCallback {
            calls,
            payload: b"result-secret".to_vec(),
        },
    )
}

#[test]
fn valid_call_invokes_one_callback_and_accepts_one_result() {
    let calls = Arc::new(AtomicUsize::new(0));
    let dispatcher = dispatcher(Arc::clone(&calls));
    let (_, cancellation) = live_cancellation();

    let outcome = block_on(dispatcher.dispatch(
        invocation(
            "call-1",
            expected_registration(),
            expected_binding(),
            BindingStatus::Current,
        ),
        cancellation,
    ))
    .expect("valid call settles");

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(outcome.result().payload_for_delivery(), b"result-secret");
    assert!(!outcome.permits_automatic_replay());
    assert_eq!(
        dispatcher
            .events()
            .into_iter()
            .map(|event| event.kind())
            .collect::<Vec<_>>(),
        vec![
            DispatchEventKind::Validated,
            DispatchEventKind::CallbackInvoked,
            DispatchEventKind::ResultAccepted,
        ]
    );
    let metrics = dispatcher.metrics();
    assert_eq!(metrics.callbacks, 1);
    assert_eq!(metrics.results, 1);
    assert_eq!(metrics.cancellations, 0);
    assert_eq!(metrics.mutation_replays, 0);
}

#[test]
fn registration_schema_kind_and_binding_refusals_precede_callback() {
    let cases = [
        (
            invocation(
                "unknown-capability",
                RegisteredCapability::new(
                    "registration-7",
                    "desktop/missing",
                    schema("1", "sha256:input"),
                    schema("1", "sha256:output"),
                    ExecutionKind::NativeClient,
                    DispatchEffect::Mutating,
                    bounds(64),
                )
                .expect("candidate registration"),
                expected_binding(),
                BindingStatus::Current,
            ),
            DispatchErrorKind::UnknownCapability,
        ),
        (
            invocation(
                "stale-registration",
                registration(
                    "registration-6",
                    schema("1", "sha256:input"),
                    schema("1", "sha256:output"),
                    ExecutionKind::NativeClient,
                ),
                expected_binding(),
                BindingStatus::Current,
            ),
            DispatchErrorKind::StaleRegistration,
        ),
        (
            invocation(
                "input-version",
                registration(
                    "registration-7",
                    schema("2", "sha256:input"),
                    schema("1", "sha256:output"),
                    ExecutionKind::NativeClient,
                ),
                expected_binding(),
                BindingStatus::Current,
            ),
            DispatchErrorKind::UnsupportedSchemaVersion,
        ),
        (
            invocation(
                "output-digest",
                registration(
                    "registration-7",
                    schema("1", "sha256:input"),
                    schema("1", "sha256:foreign"),
                    ExecutionKind::NativeClient,
                ),
                expected_binding(),
                BindingStatus::Current,
            ),
            DispatchErrorKind::UnknownSchema,
        ),
        (
            invocation(
                "wrong-kind",
                registration(
                    "registration-7",
                    schema("1", "sha256:input"),
                    schema("1", "sha256:output"),
                    ExecutionKind::Mcp,
                ),
                expected_binding(),
                BindingStatus::Current,
            ),
            DispatchErrorKind::ExecutionKindMismatch,
        ),
        (
            invocation(
                "wrong-effect",
                RegisteredCapability::new(
                    "registration-7",
                    "desktop/read_context",
                    schema("1", "sha256:input"),
                    schema("1", "sha256:output"),
                    ExecutionKind::NativeClient,
                    DispatchEffect::ReadOnly,
                    bounds(64),
                )
                .expect("candidate registration"),
                expected_binding(),
                BindingStatus::Current,
            ),
            DispatchErrorKind::EffectMismatch,
        ),
        (
            invocation(
                "foreign-binding",
                expected_registration(),
                binding("foreign-process", 3, 5, 7),
                BindingStatus::Current,
            ),
            DispatchErrorKind::ForeignBinding,
        ),
        (
            invocation(
                "stale-binding",
                expected_registration(),
                binding("process-secret", 3, 5, 6),
                BindingStatus::Current,
            ),
            DispatchErrorKind::StaleBinding,
        ),
        (
            invocation(
                "revoked-binding",
                expected_registration(),
                expected_binding(),
                BindingStatus::Revoked,
            ),
            DispatchErrorKind::RevokedBinding,
        ),
    ];

    for (candidate, expected) in cases {
        let calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = dispatcher(Arc::clone(&calls));
        let (_, cancellation) = live_cancellation();
        let error = block_on(dispatcher.dispatch(candidate, cancellation))
            .expect_err("candidate must fail");
        assert_eq!(error.kind(), expected);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(dispatcher.metrics().callbacks, 0);
    }
}

#[test]
fn bounds_are_positive_and_enforced_before_callback() {
    let invalid =
        DispatchBounds::new(1, 64, 64, 16, 2, Duration::ZERO).expect_err("zero duration must fail");
    assert_eq!(invalid.kind(), DispatchErrorKind::InvalidBounds);

    let calls = Arc::new(AtomicUsize::new(0));
    let dispatcher = dispatcher(Arc::clone(&calls));
    let registration = RegisteredCapability::new(
        "registration-7",
        "desktop/read_context",
        schema("1", "sha256:input"),
        schema("1", "sha256:output"),
        ExecutionKind::NativeClient,
        DispatchEffect::Mutating,
        DispatchBounds::new(2, 64, 64, 16, 2, Duration::from_secs(5)).expect("positive bounds"),
    )
    .expect("candidate registration");
    let (_, cancellation) = live_cancellation();
    let error = block_on(dispatcher.dispatch(
        invocation(
            "wide-bounds",
            registration,
            expected_binding(),
            BindingStatus::Current,
        ),
        cancellation,
    ))
    .expect_err("widened bounds must fail");

    assert_eq!(error.kind(), DispatchErrorKind::LimitExceeded);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn deadline_is_live_and_bounded_before_callback() {
    let calls = Arc::new(AtomicUsize::new(0));
    let dispatcher = dispatcher(Arc::clone(&calls));
    let (_, cancellation) = live_cancellation();
    for (call_id, deadline, expected) in [
        (
            "expired-call",
            DispatchDeadline::expired(),
            DispatchErrorKind::DeadlineExceeded,
        ),
        (
            "zero-call",
            DispatchDeadline::active(Duration::ZERO),
            DispatchErrorKind::DeadlineExceeded,
        ),
        (
            "overlong-call",
            DispatchDeadline::active(Duration::from_secs(6)),
            DispatchErrorKind::LimitExceeded,
        ),
    ] {
        let candidate = DispatchInvocation::new(
            call_id,
            expected_registration(),
            expected_binding(),
            BindingStatus::Current,
            deadline,
            b"argument-secret".to_vec(),
        )
        .expect("fixture invocation");
        let error = block_on(dispatcher.dispatch(candidate, cancellation.clone()))
            .expect_err("deadline must be rejected before callback");
        assert_eq!(error.kind(), expected);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn duplicate_call_and_result_are_terminal_and_never_reinvoke() {
    let calls = Arc::new(AtomicUsize::new(0));
    let retained = Arc::new(Mutex::new(None));
    let callback = RetainingCallback {
        calls: Arc::clone(&calls),
        retained: Arc::clone(&retained),
    };
    let dispatcher =
        RecordingDispatcher::new(expected_registration(), expected_binding(), callback);
    let (_, cancellation) = live_cancellation();
    let candidate = invocation(
        "duplicate",
        expected_registration(),
        expected_binding(),
        BindingStatus::Current,
    );

    block_on(dispatcher.dispatch(candidate.clone(), cancellation.clone()))
        .expect("first call succeeds");
    let duplicate = block_on(dispatcher.dispatch(candidate, cancellation))
        .expect_err("duplicate call must fail");
    let duplicate_result = retained
        .lock()
        .expect("retained recorder lock")
        .as_ref()
        .expect("callback retained recorder")
        .record(CallbackResult::new(
            schema("1", "sha256:output"),
            b"second".to_vec(),
        ))
        .expect_err("second result must fail");

    assert_eq!(duplicate.kind(), DispatchErrorKind::DuplicateCall);
    assert_eq!(duplicate_result.kind(), DispatchErrorKind::DuplicateResult);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(dispatcher.metrics().results, 1);
    assert_eq!(dispatcher.metrics().mutation_replays, 0);
}

struct RetainingCallback {
    calls: Arc<AtomicUsize>,
    retained: Arc<Mutex<Option<ResultRecorder>>>,
}

impl RecordingCallback for RetainingCallback {
    fn invoke(
        &self,
        _call: AdmittedCall,
        _cancellation: Cancellation,
        result: ResultRecorder,
    ) -> DispatchFuture<'_, Result<(), CallbackFailure>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.retained.lock().expect("retained recorder lock") = Some(result.clone());
        Box::pin(ready(
            result
                .record(CallbackResult::new(
                    schema("1", "sha256:output"),
                    b"result-secret".to_vec(),
                ))
                .map_err(|_| CallbackFailure::new()),
        ))
    }
}

struct CancellationCallback {
    calls: Arc<AtomicUsize>,
    observations: Arc<AtomicUsize>,
    retained: Arc<Mutex<Option<ResultRecorder>>>,
}

impl RecordingCallback for CancellationCallback {
    fn invoke(
        &self,
        _call: AdmittedCall,
        cancellation: Cancellation,
        result: ResultRecorder,
    ) -> DispatchFuture<'_, Result<(), CallbackFailure>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.retained.lock().expect("retained recorder lock") = Some(result.clone());
        Box::pin(WaitForCancellation {
            cancellation,
            result,
            observations: Arc::clone(&self.observations),
        })
    }
}

struct WaitForCancellation {
    cancellation: Cancellation,
    result: ResultRecorder,
    observations: Arc<AtomicUsize>,
}

impl Future for WaitForCancellation {
    type Output = Result<(), CallbackFailure>;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.cancellation.is_cancelled() {
            return Poll::Pending;
        }
        self.observations.fetch_add(1, Ordering::SeqCst);
        let rejection = self.result.record(CallbackResult::new(
            schema("1", "sha256:output"),
            b"must-not-land".to_vec(),
        ));
        assert_eq!(
            rejection.expect_err("cancellation wins").kind(),
            DispatchErrorKind::Cancelled
        );
        Poll::Ready(Ok(()))
    }
}

#[test]
fn cancellation_reaches_in_flight_callback_once_and_blocks_late_result() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observations = Arc::new(AtomicUsize::new(0));
    let retained = Arc::new(Mutex::new(None));
    let callback = CancellationCallback {
        calls: Arc::clone(&calls),
        observations: Arc::clone(&observations),
        retained: Arc::clone(&retained),
    };
    let dispatcher =
        RecordingDispatcher::new(expected_registration(), expected_binding(), callback);
    let (flag, cancellation) = live_cancellation();
    let mut future = dispatcher.dispatch(
        invocation(
            "cancelled",
            expected_registration(),
            expected_binding(),
            BindingStatus::Current,
        ),
        cancellation,
    );
    let mut context = Context::from_waker(Waker::noop());

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    flag.cancel();
    let error = match future.as_mut().poll(&mut context) {
        Poll::Ready(Err(error)) => error,
        other => panic!("cancelled dispatch must settle, got {other:?}"),
    };
    let late = retained
        .lock()
        .expect("retained recorder lock")
        .as_ref()
        .expect("callback retained recorder")
        .record(CallbackResult::new(
            schema("1", "sha256:output"),
            b"late".to_vec(),
        ))
        .expect_err("late result must fail");

    assert_eq!(error.kind(), DispatchErrorKind::Cancelled);
    assert_eq!(late.kind(), DispatchErrorKind::PostTerminalResult);
    assert_eq!(observations.load(Ordering::SeqCst), 1);
    assert_eq!(dispatcher.metrics().cancellations, 1);
    assert_eq!(dispatcher.metrics().results, 0);
    assert_eq!(dispatcher.metrics().mutation_replays, 0);
}

#[test]
fn diagnostics_and_debug_output_are_redacted() {
    let calls = Arc::new(AtomicUsize::new(0));
    let dispatcher = dispatcher(calls);
    let candidate = invocation(
        "call-secret",
        expected_registration(),
        expected_binding(),
        BindingStatus::Revoked,
    );
    let rendered = format!("{candidate:?} {dispatcher:?}");
    for protected in [
        "argument-secret",
        "process-secret",
        "task-secret",
        "session-secret",
        "attempt-secret",
        "call-secret",
    ] {
        assert!(!rendered.contains(protected));
    }

    let (_, cancellation) = live_cancellation();
    let error = block_on(dispatcher.dispatch(candidate, cancellation))
        .expect_err("revoked binding must fail");
    let diagnostic = error.diagnostic();
    let evidence = format!(
        "{} {} {:?}",
        diagnostic.code(),
        diagnostic.message(),
        dispatcher.events()
    );
    for protected in [
        "argument-secret",
        "process-secret",
        "task-secret",
        "session-secret",
        "attempt-secret",
        "call-secret",
    ] {
        assert!(!evidence.contains(protected));
    }
}

fn block_on<T>(mut future: DispatchFuture<'_, T>) -> T {
    let mut context = Context::from_waker(Waker::noop());
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
