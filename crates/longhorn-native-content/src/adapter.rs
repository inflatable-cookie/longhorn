//! Shared sequencing for mechanism adapters. Native operations stay in the adapters.

use crate::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentGate, NativeContentCoordinator,
    NativeContentFailureCode, PlannedOperation, ReceiptError, StepExecution,
};

/// Validates the coordinator cursor, prepares adapter state, and executes until the first failure.
///
/// The adapter checks its own island, mechanism, and generation before calling this function.
/// `failure_code` supplies the adapter's stable, grammar-checked operation codes.
pub fn execute_adapter_plan<E: From<ReceiptError>>(
    authority: &NativeContentCoordinator,
    plan: &ApplyPlan,
    prepare: impl FnOnce() -> Result<(), E>,
    mut execute: impl FnMut(&PlannedOperation) -> Result<(), E>,
    failure_code: impl Fn(&E) -> &'static str,
) -> Result<ApplyReceipt, E> {
    authority.validate_plan_cursor(plan)?;
    prepare()?;
    let mut executions = Vec::with_capacity(plan.operations().len());
    for planned in plan.operations() {
        let step = planned.step();
        match execute(planned) {
            Ok(()) => executions.push(StepExecution::Applied { step }),
            Err(error) => {
                let code = NativeContentFailureCode::new(failure_code(&error))
                    .expect("adapter failure codes use the bounded grammar");
                executions.push(StepExecution::Failed { step, code });
                break;
            }
        }
    }
    authority.receipt(plan, executions).map_err(Into::into)
}

/// Runs native attach after the adapter reserves a generation.
///
/// Events describe listener installation, attach start and successful retention, in that order.
/// The adapter owns handle adoption and native cleanup in `retain`. On error,
/// `clear_reservation` releases any still-incomplete reservation of this generation.
pub fn complete_reserved_attach<T, E, Event>(
    [listener_installed, attach_started, attached]: [Event; 3],
    emit: impl Fn(Event),
    attach: impl FnOnce() -> Result<T, E>,
    retain: impl FnOnce(T) -> Result<(), E>,
    clear_reservation: impl FnOnce() -> Result<(), E>,
) -> Result<(), E> {
    emit(listener_installed);
    emit(attach_started);
    if let Err(error) = attach().and_then(retain) {
        clear_reservation()?;
        return Err(error);
    }
    emit(attached);
    Ok(())
}

/// Drops an incomplete reservation only if it still belongs to the supplied generation.
pub fn clear_attach_reservation<T>(
    slot: &mut Option<T>,
    generation: AttachGeneration,
    inspect: impl FnOnce(&T) -> AttachmentGate,
) {
    if slot.as_ref().map(inspect) == Some(AttachmentGate::new(generation, false)) {
        *slot = None;
    }
}
