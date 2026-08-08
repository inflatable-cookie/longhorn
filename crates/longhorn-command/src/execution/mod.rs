//! Command admission, execution ports, and outcomes.

mod engine;
mod failure;
mod ports;
mod support;
mod types;

pub use engine::CommandAdmissionEngine;
pub use failure::{
    CommandAvailabilityProjectionError, CommandFailure, CommandFailureCode, CommandFailurePhase,
};
pub use ports::{
    CommandAvailabilitySource, CommandCapabilitySource, CommandContextSource, CommandExecutor,
    CommandSourceFailure,
};
pub(crate) use support::{failed_result, projection_source};
pub use types::{
    AdmittedCommandInvocation, CommandExecutionOutcome, CommandExecutionRequest,
    CommandExecutionResult, CommandExecutorOutcome,
};
