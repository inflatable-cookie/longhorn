use longhorn_windowing::{
    ApplyGeneration, DeferredSettlement, HostCapabilities, HostCapability, PlannedWindowOperation,
    WindowApplyHost, WindowDiffError, WindowDiffInput, WindowOperationKind, apply_window_diff,
};
use std::{error::Error, fmt};
use tauri::{AppHandle, Runtime};

use super::operations::execute_operation;
use crate::{
    ApplyConvergence, ApplyReadback, DesktopObservation, ManagedDesktopReadback,
    ManagedWindowRegistry, ManagedWindowRegistryError, TauriApplyReceipt, TauriObservationError,
    TauriWindowFactory, WindowApplyAttempt, WindowApplyOutcome, WindowMutationBackend,
};

/// Derives exact Card 018 capabilities from native support and factory availability.
#[must_use]
pub fn tauri_host_capabilities(can_create: bool) -> HostCapabilities {
    let mut capabilities = vec![
        HostCapability::Retag,
        HostCapability::Move,
        HostCapability::Resize,
        HostCapability::Maximize,
        HostCapability::Unmaximize,
        HostCapability::Show,
        HostCapability::Hide,
        HostCapability::Focus,
        HostCapability::Close,
    ];
    if can_create {
        capabilities.push(HostCapability::Create);
    }
    HostCapabilities::from_capabilities(capabilities)
}

/// Tauri settles every window operation before it can be read back.
///
/// `maximize`, `set_position` and `set_size` are observable on the next probe,
/// so a convergence diff over fresh evidence is a verdict here rather than
/// merely evidence. GPUI is not like this — see contract 020's divergence
/// register.
#[must_use]
pub fn tauri_deferred_settlement() -> DeferredSettlement {
    DeferredSettlement::immediate()
}

/// Executes one ordered apply while leaving registry ownership with the caller.
///
/// This form supports composed hosts that must restore registry ownership after
/// either a successful apply or a pre-execution failure.
pub fn execute_tauri_window_apply_in_place<R, F, B, Q>(
    app: &AppHandle<R>,
    input: WindowDiffInput,
    registry: &mut ManagedWindowRegistry<R>,
    factory: &mut F,
    backend: &mut B,
    readback: &mut Q,
) -> Result<TauriApplyReceipt, TauriApplyError>
where
    R: Runtime,
    F: TauriWindowFactory<R>,
    B: WindowMutationBackend<R>,
    Q: ManagedDesktopReadback<R>,
{
    let input = input.with_capabilities(tauri_host_capabilities(factory.can_create()));
    let mut host = TauriApplyHost {
        app,
        registry,
        factory,
        backend,
        readback,
    };
    let run = apply_window_diff(&mut host, input, &tauri_deferred_settlement())?;
    let readback = match run.readback {
        Ok((observation, convergence)) => ApplyReadback::Complete {
            observation,
            convergence: match convergence {
                Ok(plan) => ApplyConvergence::Planned(plan),
                Err(error) => ApplyConvergence::Invalid(error),
            },
        },
        Err(error) => ApplyReadback::Failed(error),
    };
    Ok(TauriApplyReceipt::new(run.plan, run.attempts, readback))
}

struct TauriApplyHost<'a, R: Runtime, F, B, Q> {
    app: &'a AppHandle<R>,
    registry: &'a mut ManagedWindowRegistry<R>,
    factory: &'a mut F,
    backend: &'a mut B,
    readback: &'a mut Q,
}

impl<R, F, B, Q> WindowApplyHost for TauriApplyHost<'_, R, F, B, Q>
where
    R: Runtime,
    F: TauriWindowFactory<R>,
    B: WindowMutationBackend<R>,
    Q: ManagedDesktopReadback<R>,
{
    type Attempt = WindowApplyAttempt;
    type Observation = DesktopObservation;
    type ApplyError = TauriApplyError;
    type ReadbackError = TauriObservationError;

    fn begin(&mut self, generation: ApplyGeneration) -> Result<(), Self::ApplyError> {
        self.registry
            .begin_generation(generation)
            .map_err(TauriApplyError::Registry)?;
        let backend = &mut self.backend;
        self.registry
            .pending_discards
            .retain(|_, window| backend.discard_created(window).is_err());
        Ok(())
    }

    fn execute(&mut self, planned: &PlannedWindowOperation) -> (Self::Attempt, bool) {
        let attempt = execute_operation(
            self.app,
            self.registry,
            self.factory,
            self.backend,
            planned.generation(),
            planned.operation(),
        );
        let succeeded = matches!(attempt.outcome(), WindowApplyOutcome::Succeeded { .. });
        (attempt, succeeded)
    }

    fn skip(
        &self,
        planned: &PlannedWindowOperation,
        blocked_by: WindowOperationKind,
    ) -> Self::Attempt {
        let operation = planned.operation();
        WindowApplyAttempt::new(
            planned.generation(),
            operation.window_id().clone(),
            operation.transport_handle().cloned(),
            operation.kind(),
            WindowApplyOutcome::DependencySkipped { blocked_by },
        )
    }

    fn readback(&mut self) -> Result<Self::Observation, Self::ReadbackError> {
        if let Some((id, _)) = self.registry.pending_discards.first_key_value() {
            return Err(TauriObservationError::PendingWindowDiscard(id.clone()));
        }
        self.readback.readback(self.app, self.registry)
    }
}

impl From<WindowDiffError> for TauriApplyError {
    fn from(error: WindowDiffError) -> Self {
        Self::Planning(error)
    }
}

/// Failure before ordered operation execution can begin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TauriApplyError {
    /// Pure diff planning rejected input identity.
    Planning(WindowDiffError),
    /// Registry rejected generation or initial execution state.
    Registry(ManagedWindowRegistryError),
}

impl fmt::Display for TauriApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planning(source) => write!(formatter, "window apply planning failed: {source}"),
            Self::Registry(source) => write!(formatter, "window registry failed: {source}"),
        }
    }
}

impl Error for TauriApplyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Planning(source) => Some(source),
            Self::Registry(source) => Some(source),
        }
    }
}
