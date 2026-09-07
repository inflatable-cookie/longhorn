//! Validation-first recording dispatcher.

use crate::{
    AdmittedCall, BindingStatus, CallbackFailure, CallbackResult, Cancellation, DispatchEffect,
    DispatchError, DispatchErrorKind, DispatchEvent, DispatchEventKind, DispatchInvocation,
    DispatchMetrics, DispatchOutcome, RegisteredCapability, SchemaIdentity, TrustedBinding,
};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};

const MAX_REMEMBERED_CALLS: usize = 256;

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
    active: BTreeSet<String>,
    seen: BTreeSet<String>,
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
            active: BTreeSet::new(),
            seen: BTreeSet::new(),
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

    fn settle_failure(&mut self, call_id: &str, error: DispatchError) {
        self.active.remove(call_id);
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
    state: Arc<Mutex<DispatcherState>>,
}

impl ResultRecorder {
    /// Accepts the first matching bounded result for the bound call.
    ///
    /// Cancellation wins before result validation. A second result is
    /// rejected as duplicate; any result after cancellation or failure is
    /// rejected as post-terminal.
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
        if !state.active.contains(&self.call_id) {
            let error = DispatchError::new(DispatchErrorKind::PostTerminalResult);
            state.reject(error);
            return Err(error);
        }
        if self.cancellation.is_cancelled() {
            let error = DispatchError::new(DispatchErrorKind::Cancelled);
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
        state.active.remove(&self.call_id);
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
            if let Err(error) = self.validate(&invocation) {
                locked(&self.state).reject(error);
                return Err(error);
            }
            let admitted = AdmittedCall::from_invocation(&invocation);
            let recorder = {
                let mut state = locked(&self.state);
                if state.seen.contains(invocation.call_id()) {
                    let error = DispatchError::new(DispatchErrorKind::DuplicateCall);
                    state.reject(error);
                    return Err(error);
                }
                if state.seen.len() >= MAX_REMEMBERED_CALLS
                    || state.active.len()
                        >= invocation.registration().bounds().max_outstanding_calls()
                {
                    let error = DispatchError::new(DispatchErrorKind::LimitExceeded);
                    state.reject(error);
                    return Err(error);
                }
                state.seen.insert(invocation.call_id().to_owned());
                state.active.insert(invocation.call_id().to_owned());
                state.record(DispatchEventKind::Validated);
                state.callbacks = state.callbacks.saturating_add(1);
                state.record(DispatchEventKind::CallbackInvoked);
                ResultRecorder {
                    call_id: invocation.call_id().to_owned(),
                    expected_schema: invocation.registration().output_schema().clone(),
                    max_result_bytes: invocation.registration().bounds().max_result_bytes(),
                    effect: invocation.registration().effect(),
                    cancellation: cancellation.clone(),
                    state: Arc::clone(&self.state),
                }
            };

            let callback = self
                .callback
                .invoke(admitted, cancellation.clone(), recorder)
                .await;
            let mut state = locked(&self.state);
            if state.active.contains(invocation.call_id()) {
                let error = if cancellation.is_cancelled() {
                    DispatchError::new(DispatchErrorKind::Cancelled)
                } else if callback.is_err() {
                    DispatchError::new(DispatchErrorKind::CallbackFailed)
                } else {
                    DispatchError::new(DispatchErrorKind::MissingResult)
                };
                state.settle_failure(invocation.call_id(), error);
            }
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

    fn validate(&self, invocation: &DispatchInvocation) -> Result<(), DispatchError> {
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
        if !actual.bounds().fits_within(expected.bounds())
            || invocation.arguments_for_execution().len() > actual.bounds().max_argument_bytes()
        {
            return Err(DispatchError::new(DispatchErrorKind::LimitExceeded));
        }
        Ok(())
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
            mutation_replays: 0,
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
            .field("active_calls", &state.active.len())
            .field("remembered_calls", &state.seen.len())
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
