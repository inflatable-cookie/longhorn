//! Narrow Tauri host assembly for injected operation authority and executor ports.

mod authority;
mod commands;
mod error;
mod handler;

pub use authority::{OperationExecutorError, OperationExecutorPort, OperationHostAuthority};
pub use commands::{
    OPERATION_CHANGED_EVENT, OperationHostService, TauriOperationState, longhorn_operation_cancel,
    longhorn_operation_mutate, longhorn_operation_snapshot, operation_cancellation_changed_event,
    operation_mutation_changed_event, publish_operation_changed,
};
pub use error::{OperationHostError, OperationHostErrorCode};
pub use handler::OperationHandlerAssembly;
