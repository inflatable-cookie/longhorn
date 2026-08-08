//! Mutation and cancellation commands, receipts, and results.

mod command;
mod receipt;
mod result;

pub use command::{OperationCancellationCommand, OperationMutationCommand};
pub use receipt::{
    OperationMutationReceiptProjection, OperationRemovalProjection,
    OperationRemovalReasonProjection, OperationTeardownOutcomeProjection,
};
pub use result::{
    OperationCancellationOutcomeProjection, OperationCancellationReceiptProjection,
    OperationCancellationResult, OperationExecutorDispatchProjection, OperationMutationResult,
    OperationRejection, OperationRejectionCode,
};
