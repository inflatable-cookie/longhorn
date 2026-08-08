//! Shared execution helpers.

use longhorn_core::CommandRequestId;

use super::{
    CommandAvailabilityProjectionError, CommandExecutionOutcome, CommandExecutionResult,
    CommandFailure, CommandFailurePhase, CommandSourceFailure,
};

pub(crate) fn projection_source(
    phase: CommandFailurePhase,
    failure: CommandSourceFailure,
) -> CommandAvailabilityProjectionError {
    CommandAvailabilityProjectionError(CommandFailure::source(phase, failure))
}

pub(crate) fn failed_result(
    request_id: CommandRequestId,
    failure: CommandFailure,
) -> CommandExecutionResult {
    CommandExecutionResult::new(request_id, CommandExecutionOutcome::Failed { failure })
}
