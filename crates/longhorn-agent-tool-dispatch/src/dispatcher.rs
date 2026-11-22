//! Validation-first recording dispatcher.

use crate::{
    AdmittedCall, BindingStatus, CallbackFailure, CallbackResult, Cancellation, DispatchDeadline,
    DispatchEffect, DispatchError, DispatchErrorKind, DispatchEvent, DispatchEventKind,
    DispatchInvocation, DispatchMetrics, DispatchOutcome, RegisteredCapability, SchemaIdentity,
    TrustedBinding,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

/// Sendable future returned by callback and dispatcher ports.
pub type DispatchFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Consumer-owned callback invoked only after one call passes every check.
pub trait RecordingCallback: Send + Sync {
    /// Handles one admitted call and records exactly one result.
    ///
    /// Returning `Ok(())` without recording a result is a typed failure.
    fn invoke(
        &self,
        call: AdmittedCall,
        cancellation: Cancellation,
        result: ResultRecorder,
    ) -> DispatchFuture<'_, Result<(), CallbackFailure>>;
}

#[derive(Clone)]
enum Terminal {
    Result(DispatchOutcome),
    Failed(DispatchError),
}

struct DispatcherState {
    /// Calls whose callback future is currently executing or suspended.
    /// Occupancy is released only when the callback completes or its dispatch
    /// future is dropped — never early when `ResultRecorder::record` settles.
    in_flight: BTreeSet<String>,
    /// First terminal outcome per call id (kept for duplicate/post-terminal
    /// enforcement).
    terminal: BTreeMap<String, Terminal>,
    events: Vec<DispatchEvent>,
    callbacks: usize,
    results: usize,
    cancellations: usize,
    rejections: usize,
}

impl DispatcherState {
    fn new() -> Self {
        Self {
            in_flight: BTreeSet::new(),
            terminal: BTreeMap::new(),
            events: Vec::new(),
            callbacks: 0,
            results: 0,
            cancellations: 0,
            rejections: 0,
        }
    }

    fn record(&mut self, kind: DispatchEventKind) {
        let sequence = u64::try_from(self.events.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        self.events.push(DispatchEvent::new(sequence, kind));
    }

    fn reject(&mut self, error: DispatchError) {
        self.rejections = self.rejections.saturating_add(1);
        self.record(DispatchEventKind::Rejected(error.kind()));
    }

    /// Records the first terminal failure for `call_id` without releasing
    /// `in_flight` occupancy (the callback may still be running).
    fn settle_failure(&mut self, call_id: &str, error: DispatchError) {
        if self.terminal.contains_key(call_id) {
            return;
        }
        self.terminal
            .insert(call_id.to_owned(), Terminal::Failed(error));
        if error.kind() == DispatchErrorKind::Cancelled {
            self.cancellations = self.cancellations.saturating_add(1);
            self.record(DispatchEventKind::Cancelled);
        } else {
            self.reject(error);
        }
    }
}

/// Bound terminal-result port passed to exactly one callback invocation.
#[derive(Clone)]
pub struct ResultRecorder {
    call_id: String,
    expected_schema: SchemaIdentity,
    max_result_bytes: usize,
    effect: DispatchEffect,
    cancellation: Cancellation,
    deadline_at: Instant,
    state: Arc<Mutex<DispatcherState>>,
}

impl ResultRecorder {
    /// Accepts the first matching bounded result for the bound call.
    ///
    /// Cancellation and deadline expiry win before result validation. A second
    /// result is rejected as duplicate; any result after cancellation,
    /// deadline expiry, or failure is rejected as post-terminal. Recording a
    /// terminal result does not release in-flight callback capacity until the
    /// callback future finishes or is dropped.
    pub fn record(&self, result: CallbackResult) -> Result<(), DispatchError> {
        let mut state = locked(&self.state);
        match state.terminal.get(&self.call_id) {
            Some(Terminal::Result(_)) => {
                let error = DispatchError::new(DispatchErrorKind::DuplicateResult);
                state.reject(error);
                return Err(error);
            }
            Some(Terminal::Failed(_)) => {
                let error = DispatchError::new(DispatchErrorKind::PostTerminalResult);
                state.reject(error);
                return Err(error);
            }
            None => {}
        }
        if !state.in_flight.contains(&self.call_id) {
            let error = DispatchError::new(DispatchErrorKind::PostTerminalResult);
            state.reject(error);
            return Err(error);
        }
        if self.cancellation.is_cancelled() {
            let error = DispatchError::new(DispatchErrorKind::Cancelled);
            state.settle_failure(&self.call_id, error);
            return Err(error);
        }
        if Instant::now() >= self.deadline_at {
            let error = DispatchError::new(DispatchErrorKind::DeadlineExceeded);
            state.settle_failure(&self.call_id, error);
            return Err(error);
        }
        let error = if result.schema().version() != self.expected_schema.version() {
            Some(DispatchError::new(
                DispatchErrorKind::UnsupportedSchemaVersion,
            ))
        } else if result.schema().digest() != self.expected_schema.digest() {
            Some(DispatchError::new(DispatchErrorKind::UnknownSchema))
        } else if result.payload_for_delivery().len() > self.max_result_bytes {
            Some(DispatchError::new(DispatchErrorKind::LimitExceeded))
        } else {
            None
        };
        if let Some(error) = error {
            state.settle_failure(&self.call_id, error);
            return Err(error);
        }
        let outcome = DispatchOutcome::new(self.call_id.clone(), self.effect, result);
        state
            .terminal
            .insert(self.call_id.clone(), Terminal::Result(outcome));
        state.results = state.results.saturating_add(1);
        state.record(DispatchEventKind::ResultAccepted);
        Ok(())
    }
}

impl std::fmt::Debug for ResultRecorder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResultRecorder")
            .field("call_id", &"<opaque>")
            .field("expected_schema", &self.expected_schema)
            .field("max_result_bytes", &self.max_result_bytes)
            .field("effect", &self.effect)
            .field("cancellation", &self.cancellation)
            .finish()
    }
}

/// RAII cleanup for an admitted call: if the dispatch future is dropped before
/// normal settlement, releases `in_flight` capacity and records a terminal
/// cancellation if no terminal outcome was recorded yet.
struct ActiveCallGuard {
    call_id: String,
    state: Arc<Mutex<DispatcherState>>,
    disarmed: bool,
}

impl Drop for ActiveCallGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        let mut state = locked(&self.state);
        state.in_flight.remove(&self.call_id);
        if !state.terminal.contains_key(&self.call_id) {
            state.settle_failure(
                &self.call_id,
                DispatchError::new(DispatchErrorKind::Cancelled),
            );
        }
    }
}

/// Pure-`std` timer reactor shared by active dispatch deadlines so an idle
/// callback future that never wakes still wakes the executor on expiry.
struct TimerDriver {
    state: Mutex<TimerState>,
    wake_thread: Condvar,
}

struct TimerState {
    next_id: u64,
    entries: BTreeMap<u64, (Instant, Waker)>,
    heap: BinaryHeap<Reverse<(Instant, u64)>>,
}

impl TimerDriver {
    fn global() -> &'static Self {
        static DRIVER: OnceLock<TimerDriver> = OnceLock::new();
        DRIVER.get_or_init(|| {
            let driver = TimerDriver {
                state: Mutex::new(TimerState {
                    next_id: 1,
                    entries: BTreeMap::new(),
                    heap: BinaryHeap::new(),
                }),
                wake_thread: Condvar::new(),
            };
            // Spawn once for the process; the daemon thread parks on `Condvar`
            // whenever no deadline is registered.
            let _ = thread::Builder::new()
                .name("longhorn-dispatch-timer".to_owned())
                .spawn(|| Self::run(Self::global()));
            driver
        })
    }

    fn run(&self) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            let now = Instant::now();
            let mut ready = Vec::new();
            while let Some(&Reverse((deadline, id))) = guard.heap.peek() {
                if deadline > now {
                    break;
                }
                guard.heap.pop();
                if let Some((registered_deadline, waker)) = guard.entries.remove(&id) {
                    if registered_deadline <= now {
                        ready.push(waker);
                    } else {
                        guard.entries.insert(id, (registered_deadline, waker));
                        guard.heap.push(Reverse((registered_deadline, id)));
                    }
                }
            }
            if !ready.is_empty() {
                drop(guard);
                for waker in ready {
                    waker.wake();
                }
                guard = self
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                continue;
            }
            match guard.heap.peek() {
                None => {
                    guard = self
                        .wake_thread
                        .wait(guard)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                }
                Some(&Reverse((earliest, _))) => {
                    let wait = earliest.saturating_duration_since(Instant::now());
                    if !wait.is_zero() {
                        let (next, _) = self
                            .wake_thread
                            .wait_timeout(guard, wait)
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        guard = next;
                    }
                }
            }
        }
    }

    fn register(&self, existing: Option<u64>, deadline: Instant, waker: Waker) -> u64 {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = existing.unwrap_or_else(|| {
            let id = guard.next_id;
            guard.next_id = guard.next_id.wrapping_add(1).max(1);
            id
        });
        let notify = guard
            .heap
            .peek()
            .is_none_or(|&Reverse((earliest, _))| deadline < earliest);
        guard.entries.insert(id, (deadline, waker));
        if existing.is_none() {
            guard.heap.push(Reverse((deadline, id)));
        }
        if notify {
            self.wake_thread.notify_one();
        }
        id
    }

    fn cancel(&self, id: u64) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.entries.remove(&id);
    }
}

/// Polls a callback future while racing it against a waking deadline timer.
struct BoundedCallback<'a> {
    inner: DispatchFuture<'a, Result<(), CallbackFailure>>,
    deadline_at: Instant,
    timer_id: Option<u64>,
}

enum CallbackPoll {
    Completed(Result<(), CallbackFailure>),
    DeadlineExceeded,
}

impl Drop for BoundedCallback<'_> {
    fn drop(&mut self) {
        if let Some(id) = self.timer_id.take() {
            TimerDriver::global().cancel(id);
        }
    }
}

impl Future for BoundedCallback<'_> {
    type Output = CallbackPoll;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline_at {
            return Poll::Ready(CallbackPoll::DeadlineExceeded);
        }
        match self.inner.as_mut().poll(cx) {
            Poll::Ready(outcome) => Poll::Ready(CallbackPoll::Completed(outcome)),
            Poll::Pending => {
                if Instant::now() >= self.deadline_at {
                    return Poll::Ready(CallbackPoll::DeadlineExceeded);
                }
                let id = TimerDriver::global().register(
                    self.timer_id,
                    self.deadline_at,
                    cx.waker().clone(),
                );
                self.timer_id = Some(id);
                Poll::Pending
            }
        }
    }
}

/// Stateful dispatcher that records safe callback-order evidence.
pub struct RecordingDispatcher<C> {
    expected_registration: RegisteredCapability,
    expected_binding: TrustedBinding,
    callback: C,
    state: Arc<Mutex<DispatcherState>>,
}

impl<C> RecordingDispatcher<C>
where
    C: RecordingCallback,
{
    /// Creates one dispatcher for an exact registration and trusted binding.
    #[must_use]
    pub fn new(
        expected_registration: RegisteredCapability,
        expected_binding: TrustedBinding,
        callback: C,
    ) -> Self {
        Self {
            expected_registration,
            expected_binding,
            callback,
            state: Arc::new(Mutex::new(DispatcherState::new())),
        }
    }

    /// Validates, invokes, and settles one call exactly once.
    pub fn dispatch(
        &self,
        invocation: DispatchInvocation,
        cancellation: Cancellation,
    ) -> DispatchFuture<'_, Result<DispatchOutcome, DispatchError>> {
        Box::pin(async move {
            let remaining = match self.validate(&invocation) {
                Ok(remaining) => remaining,
                Err(error) => {
                    locked(&self.state).reject(error);
                    return Err(error);
                }
            };
            let Some(deadline_at) = Instant::now().checked_add(remaining) else {
                let error = DispatchError::new(DispatchErrorKind::LimitExceeded);
                locked(&self.state).reject(error);
                return Err(error);
            };
            let admitted = AdmittedCall::from_invocation(&invocation);
            let recorder = {
                let mut state = locked(&self.state);
                if state.in_flight.contains(invocation.call_id())
                    || state.terminal.contains_key(invocation.call_id())
                {
                    let error = DispatchError::new(DispatchErrorKind::DuplicateCall);
                    state.reject(error);
                    return Err(error);
                }
                if state.callbacks >= self.expected_registration.bounds().max_calls_per_binding()
                    || state.in_flight.len()
                        >= invocation.registration().bounds().max_outstanding_calls()
                {
                    let error = DispatchError::new(DispatchErrorKind::LimitExceeded);
                    state.reject(error);
                    return Err(error);
                }
                state.in_flight.insert(invocation.call_id().to_owned());
                state.record(DispatchEventKind::Validated);
                state.callbacks = state.callbacks.saturating_add(1);
                state.record(DispatchEventKind::CallbackInvoked);
                ResultRecorder {
                    call_id: invocation.call_id().to_owned(),
                    expected_schema: invocation.registration().output_schema().clone(),
                    max_result_bytes: invocation.registration().bounds().max_result_bytes(),
                    effect: invocation.registration().effect(),
                    cancellation: cancellation.clone(),
                    deadline_at,
                    state: Arc::clone(&self.state),
                }
            };
            let mut active_guard = ActiveCallGuard {
                call_id: invocation.call_id().to_owned(),
                state: Arc::clone(&self.state),
                disarmed: false,
            };

            let callback = BoundedCallback {
                inner: self
                    .callback
                    .invoke(admitted, cancellation.clone(), recorder),
                deadline_at,
                timer_id: None,
            }
            .await;
            let mut state = locked(&self.state);
            state.in_flight.remove(invocation.call_id());
            if !state.terminal.contains_key(invocation.call_id()) {
                let error = match callback {
                    CallbackPoll::DeadlineExceeded => {
                        DispatchError::new(DispatchErrorKind::DeadlineExceeded)
                    }
                    CallbackPoll::Completed(outcome) => {
                        if cancellation.is_cancelled() {
                            DispatchError::new(DispatchErrorKind::Cancelled)
                        } else if Instant::now() >= deadline_at {
                            DispatchError::new(DispatchErrorKind::DeadlineExceeded)
                        } else if outcome.is_err() {
                            DispatchError::new(DispatchErrorKind::CallbackFailed)
                        } else {
                            DispatchError::new(DispatchErrorKind::MissingResult)
                        }
                    }
                };
                state.settle_failure(invocation.call_id(), error);
            }
            active_guard.disarmed = true;
            match state.terminal.get(invocation.call_id()).cloned() {
                Some(Terminal::Result(outcome)) => Ok(outcome),
                Some(Terminal::Failed(error)) => Err(error),
                None => {
                    let error = DispatchError::new(DispatchErrorKind::MissingResult);
                    state.reject(error);
                    Err(error)
                }
            }
        })
    }

    fn validate(&self, invocation: &DispatchInvocation) -> Result<Duration, DispatchError> {
        let actual = invocation.registration();
        let expected = &self.expected_registration;
        if actual.registration_revision() != expected.registration_revision() {
            return Err(DispatchError::new(DispatchErrorKind::StaleRegistration));
        }
        if actual.capability() != expected.capability() {
            return Err(DispatchError::new(DispatchErrorKind::UnknownCapability));
        }
        validate_schema(actual.input_schema(), expected.input_schema())?;
        validate_schema(actual.output_schema(), expected.output_schema())?;
        if actual.execution_kind() != expected.execution_kind() {
            return Err(DispatchError::new(DispatchErrorKind::ExecutionKindMismatch));
        }
        if actual.effect() != expected.effect() {
            return Err(DispatchError::new(DispatchErrorKind::EffectMismatch));
        }
        if !invocation
            .binding()
            .stable_identity_matches(&self.expected_binding)
        {
            return Err(DispatchError::new(DispatchErrorKind::ForeignBinding));
        }
        if !invocation
            .binding()
            .generations_match(&self.expected_binding)
        {
            return Err(DispatchError::new(DispatchErrorKind::StaleBinding));
        }
        if invocation.binding_status() == BindingStatus::Revoked {
            return Err(DispatchError::new(DispatchErrorKind::RevokedBinding));
        }
        actual.bounds().validate()?;
        invocation
            .deadline()
            .validate(actual.bounds().max_call_duration())?;
        let DispatchDeadline::Active(remaining) = invocation.deadline() else {
            return Err(DispatchError::new(DispatchErrorKind::DeadlineExceeded));
        };
        if !actual.bounds().fits_within(expected.bounds())
            || invocation.arguments_for_execution().len() > actual.bounds().max_argument_bytes()
        {
            return Err(DispatchError::new(DispatchErrorKind::LimitExceeded));
        }
        Ok(remaining)
    }

    /// Returns ordered payload-free evidence.
    #[must_use]
    pub fn events(&self) -> Vec<DispatchEvent> {
        locked(&self.state).events.clone()
    }

    /// Returns safe aggregate counts.
    #[must_use]
    pub fn metrics(&self) -> DispatchMetrics {
        let state = locked(&self.state);
        DispatchMetrics {
            callbacks: state.callbacks,
            results: state.results,
            cancellations: state.cancellations,
            rejections: state.rejections,
        }
    }
}

impl<C> std::fmt::Debug for RecordingDispatcher<C> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = locked(&self.state);
        formatter
            .debug_struct("RecordingDispatcher")
            .field("expected_registration", &self.expected_registration)
            .field("expected_binding", &self.expected_binding)
            .field("active_calls", &state.in_flight.len())
            .field(
                "remembered_calls",
                &state.in_flight.len().saturating_add(state.terminal.len()),
            )
            .field("terminal_calls", &state.terminal.len())
            .field("callback", &"<consumer callback>")
            .finish()
    }
}

fn validate_schema(
    actual: &SchemaIdentity,
    expected: &SchemaIdentity,
) -> Result<(), DispatchError> {
    if actual.version() != expected.version() {
        Err(DispatchError::new(
            DispatchErrorKind::UnsupportedSchemaVersion,
        ))
    } else if actual.digest() != expected.digest() {
        Err(DispatchError::new(DispatchErrorKind::UnknownSchema))
    } else {
        Ok(())
    }
}

fn locked(state: &Mutex<DispatcherState>) -> MutexGuard<'_, DispatcherState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
