//! Public binding, invocation, result, and evidence types.

use crate::registration::{DispatchEffect, ExecutionKind, RegisteredCapability, SchemaIdentity};
use crate::{DispatchError, DispatchErrorKind};
use std::fmt;
use std::time::Duration;

const MAX_IDENTITY_BYTES: usize = 256;

pub(crate) fn validate_identity(value: &str) -> Result<(), DispatchError> {
    if value.trim().is_empty()
        || value.len() > MAX_IDENTITY_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character == '\u{7f}')
    {
        Err(DispatchError::new(DispatchErrorKind::InvalidIdentity))
    } else {
        Ok(())
    }
}

/// Durable consumer admission identity plus Swallowtail operation generation.
#[derive(Clone, Eq, PartialEq)]
pub struct TrustedBinding {
    process_incarnation: String,
    workspace_generation: u64,
    task_generation: u64,
    operation_generation: u64,
    task: String,
    session: String,
    attempt: String,
}

impl TrustedBinding {
    /// Creates one fixed, positive-generation consumer attempt binding.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        process_incarnation: impl Into<String>,
        workspace_generation: u64,
        task_generation: u64,
        operation_generation: u64,
        task: impl Into<String>,
        session: impl Into<String>,
        attempt: impl Into<String>,
    ) -> Result<Self, DispatchError> {
        let process_incarnation = process_incarnation.into();
        let task = task.into();
        let session = session.into();
        let attempt = attempt.into();
        for value in [&process_incarnation, &task, &session, &attempt] {
            validate_identity(value)?;
        }
        if workspace_generation == 0 || task_generation == 0 || operation_generation == 0 {
            return Err(DispatchError::new(DispatchErrorKind::InvalidBounds));
        }
        Ok(Self {
            process_incarnation,
            workspace_generation,
            task_generation,
            operation_generation,
            task,
            session,
            attempt,
        })
    }

    pub(crate) fn stable_identity_matches(&self, other: &Self) -> bool {
        self.process_incarnation == other.process_incarnation
            && self.task == other.task
            && self.session == other.session
            && self.attempt == other.attempt
    }

    pub(crate) const fn generations_match(&self, other: &Self) -> bool {
        self.workspace_generation == other.workspace_generation
            && self.task_generation == other.task_generation
            && self.operation_generation == other.operation_generation
    }

    /// Returns the opaque process incarnation for exact host comparison.
    #[must_use]
    pub fn process_incarnation(&self) -> &str {
        &self.process_incarnation
    }

    /// Returns the consumer workspace generation.
    #[must_use]
    pub const fn workspace_generation(&self) -> u64 {
        self.workspace_generation
    }

    /// Returns the consumer task generation.
    #[must_use]
    pub const fn task_generation(&self) -> u64 {
        self.task_generation
    }

    /// Returns the Swallowtail operation generation.
    #[must_use]
    pub const fn operation_generation(&self) -> u64 {
        self.operation_generation
    }

    /// Returns the opaque task identity for exact host comparison.
    #[must_use]
    pub fn task(&self) -> &str {
        &self.task
    }

    /// Returns the opaque session identity for exact host comparison.
    #[must_use]
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Returns the opaque attempt identity for exact host comparison.
    #[must_use]
    pub fn attempt(&self) -> &str {
        &self.attempt
    }
}

impl fmt::Debug for TrustedBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrustedBinding")
            .field("process_incarnation", &"<opaque>")
            .field("workspace_generation", &self.workspace_generation)
            .field("task_generation", &self.task_generation)
            .field("operation_generation", &self.operation_generation)
            .field("task", &"<opaque>")
            .field("session", &"<opaque>")
            .field("attempt", &"<opaque>")
            .finish()
    }
}

/// Current live status supplied by the trusted admission owner.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BindingStatus {
    /// The fixed binding remains current.
    Current,
    /// The fixed binding has been revoked.
    Revoked,
}

/// Host-observed monotonic deadline state for one candidate call.
///
/// The composing host calculates the remaining duration from its trusted
/// clock. Longhorn rejects expired, zero, or registration-exceeding values
/// before invoking the callback.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DispatchDeadline {
    /// The trusted clock reports this positive duration remains.
    Active(Duration),
    /// The trusted clock reports that the deadline has elapsed.
    Expired,
}

impl DispatchDeadline {
    /// Creates an active deadline from the host-observed remaining duration.
    #[must_use]
    pub const fn active(remaining: Duration) -> Self {
        Self::Active(remaining)
    }

    /// Creates an elapsed deadline marker.
    #[must_use]
    pub const fn expired() -> Self {
        Self::Expired
    }

    pub(crate) fn validate(self, maximum: Duration) -> Result<(), DispatchError> {
        match self {
            Self::Expired | Self::Active(Duration::ZERO) => {
                Err(DispatchError::new(DispatchErrorKind::DeadlineExceeded))
            }
            Self::Active(remaining) if remaining > maximum => {
                Err(DispatchError::new(DispatchErrorKind::LimitExceeded))
            }
            Self::Active(_) => Ok(()),
        }
    }
}

/// One candidate call before Longhorn validation and admission.
#[derive(Clone, Eq, PartialEq)]
pub struct DispatchInvocation {
    call_id: String,
    registration: RegisteredCapability,
    binding: TrustedBinding,
    binding_status: BindingStatus,
    deadline: DispatchDeadline,
    arguments: Vec<u8>,
}

impl DispatchInvocation {
    /// Creates one invocation from host-normalized facts and opaque arguments.
    pub fn new(
        call_id: impl Into<String>,
        registration: RegisteredCapability,
        binding: TrustedBinding,
        binding_status: BindingStatus,
        deadline: DispatchDeadline,
        arguments: impl Into<Vec<u8>>,
    ) -> Result<Self, DispatchError> {
        let call_id = call_id.into();
        validate_identity(&call_id)?;
        Ok(Self {
            call_id,
            registration,
            binding,
            binding_status,
            deadline,
            arguments: arguments.into(),
        })
    }

    /// Returns the opaque call identity for exact host comparison.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// Returns the normalized registration carried by the call.
    #[must_use]
    pub const fn registration(&self) -> &RegisteredCapability {
        &self.registration
    }

    /// Returns the trusted binding carried by the call.
    #[must_use]
    pub const fn binding(&self) -> &TrustedBinding {
        &self.binding
    }

    /// Returns the live binding status supplied by the admission owner.
    #[must_use]
    pub const fn binding_status(&self) -> BindingStatus {
        self.binding_status
    }

    /// Returns the host-observed typed deadline state.
    #[must_use]
    pub const fn deadline(&self) -> DispatchDeadline {
        self.deadline
    }

    /// Returns opaque arguments for callback execution only.
    #[must_use]
    pub fn arguments_for_execution(&self) -> &[u8] {
        &self.arguments
    }
}

impl fmt::Debug for DispatchInvocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DispatchInvocation")
            .field("call_id", &"<opaque>")
            .field("registration", &self.registration)
            .field("binding", &self.binding)
            .field("binding_status", &self.binding_status)
            .field("deadline", &self.deadline)
            .field("arguments", &"<redacted>")
            .field("argument_bytes", &self.arguments.len())
            .finish()
    }
}

/// Validated callback input created only after every pre-dispatch check.
#[derive(Clone, Eq, PartialEq)]
pub struct AdmittedCall {
    call_id: String,
    capability: String,
    execution_kind: ExecutionKind,
    effect: DispatchEffect,
    arguments: Vec<u8>,
}

impl AdmittedCall {
    pub(crate) fn from_invocation(invocation: &DispatchInvocation) -> Self {
        Self {
            call_id: invocation.call_id.clone(),
            capability: invocation.registration.capability().to_owned(),
            execution_kind: invocation.registration.execution_kind(),
            effect: invocation.registration.effect(),
            arguments: invocation.arguments.clone(),
        }
    }

    /// Returns the opaque call identity for exact host comparison.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// Returns the admitted capability identity.
    #[must_use]
    pub fn capability(&self) -> &str {
        &self.capability
    }

    /// Returns the admitted execution kind.
    #[must_use]
    pub const fn execution_kind(&self) -> ExecutionKind {
        self.execution_kind
    }

    /// Returns the admitted effect posture.
    #[must_use]
    pub const fn effect(&self) -> DispatchEffect {
        self.effect
    }

    /// Returns opaque arguments for callback execution only.
    #[must_use]
    pub fn arguments_for_execution(&self) -> &[u8] {
        &self.arguments
    }
}

impl fmt::Debug for AdmittedCall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedCall")
            .field("call_id", &"<opaque>")
            .field("capability", &self.capability)
            .field("execution_kind", &self.execution_kind)
            .field("effect", &self.effect)
            .field("arguments", &"<redacted>")
            .field("argument_bytes", &self.arguments.len())
            .finish()
    }
}

/// One callback-produced bounded result and its exact output schema identity.
#[derive(Clone, Eq, PartialEq)]
pub struct CallbackResult {
    schema: SchemaIdentity,
    payload: Vec<u8>,
}

impl CallbackResult {
    /// Creates one result for later validation by the bound result recorder.
    #[must_use]
    pub fn new(schema: SchemaIdentity, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            schema,
            payload: payload.into(),
        }
    }

    /// Returns the exact output schema identity.
    #[must_use]
    pub const fn schema(&self) -> &SchemaIdentity {
        &self.schema
    }

    /// Returns result bytes for host delivery only.
    #[must_use]
    pub fn payload_for_delivery(&self) -> &[u8] {
        &self.payload
    }
}

impl fmt::Debug for CallbackResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CallbackResult")
            .field("schema", &self.schema)
            .field("payload", &"<redacted>")
            .field("payload_bytes", &self.payload.len())
            .finish()
    }
}

/// One successful terminal dispatch outcome.
#[derive(Clone, Eq, PartialEq)]
pub struct DispatchOutcome {
    call_id: String,
    effect: DispatchEffect,
    result: CallbackResult,
}

impl DispatchOutcome {
    pub(crate) fn new(call_id: String, effect: DispatchEffect, result: CallbackResult) -> Self {
        Self {
            call_id,
            effect,
            result,
        }
    }

    /// Returns the opaque call identity for exact host comparison.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// Returns the accepted bounded result.
    #[must_use]
    pub const fn result(&self) -> &CallbackResult {
        &self.result
    }

    /// Returns the effect posture fixed before dispatch.
    #[must_use]
    pub const fn effect(&self) -> DispatchEffect {
        self.effect
    }

    /// Reports whether Longhorn may automatically replay this call.
    ///
    /// Contract 023 prohibits transport replay. A consumer retry always needs
    /// a fresh attempt and operation generation, regardless of effect posture.
    #[must_use]
    pub const fn permits_automatic_replay(&self) -> bool {
        false
    }
}

impl fmt::Debug for DispatchOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DispatchOutcome")
            .field("call_id", &"<opaque>")
            .field("effect", &self.effect)
            .field("result", &self.result)
            .finish()
    }
}

/// Safe event kind emitted by the recording dispatcher.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DispatchEventKind {
    /// Every pre-dispatch check passed.
    Validated,
    /// The injected callback was invoked.
    CallbackInvoked,
    /// One bounded result was accepted.
    ResultAccepted,
    /// Cancellation became the terminal outcome.
    Cancelled,
    /// A typed failure was recorded without detail text.
    Rejected(DispatchErrorKind),
}

/// Ordered, payload-free dispatcher evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DispatchEvent {
    sequence: u64,
    kind: DispatchEventKind,
}

impl DispatchEvent {
    pub(crate) const fn new(sequence: u64, kind: DispatchEventKind) -> Self {
        Self { sequence, kind }
    }

    /// Returns the monotonic recording sequence.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// Returns the safe event kind.
    #[must_use]
    pub const fn kind(self) -> DispatchEventKind {
        self.kind
    }
}

/// Safe aggregate counts from one recording dispatcher.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct DispatchMetrics {
    /// Callbacks invoked after validation.
    pub callbacks: usize,
    /// Results accepted as terminal.
    pub results: usize,
    /// Calls terminally cancelled.
    pub cancellations: usize,
    /// Calls or results rejected.
    pub rejections: usize,
    /// Automatic mutation replays. This is always zero.
    pub mutation_replays: usize,
}
