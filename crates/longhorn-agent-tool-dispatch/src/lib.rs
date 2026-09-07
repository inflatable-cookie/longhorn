//! Transport-neutral typed host dispatch for contextual agent tools.
//!
//! This opt-in crate implements Longhorn contract 023's validation and
//! recording callback boundary. Registrations, schemas, durable admission,
//! operation generations, transports, and provider execution remain owned by
//! the composing host. The crate depends on none of them and contains none of
//! contract 022's developer control surface.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod cancellation;
mod dispatcher;
mod error;
mod registration;
mod types;

pub use cancellation::{Cancellation, CancellationSource};
pub use dispatcher::{DispatchFuture, RecordingCallback, RecordingDispatcher, ResultRecorder};
pub use error::{CallbackFailure, DispatchError, DispatchErrorKind, SafeDiagnostic};
pub use registration::{
    DispatchBounds, DispatchEffect, ExecutionKind, RegisteredCapability, SchemaIdentity,
};
pub use types::{
    AdmittedCall, BindingStatus, CallbackResult, DispatchDeadline, DispatchEvent,
    DispatchEventKind, DispatchInvocation, DispatchMetrics, DispatchOutcome, TrustedBinding,
};
