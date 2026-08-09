use std::{collections::BTreeMap, error::Error, fmt};

use longhorn_core::WindowId;
use longhorn_windowing::{
    DeferredSettlement, HostCapabilities, HostCapability, WindowDiffError, WindowDiffInput,
    WindowOperationKind, plan_window_diff,
};
use tauri::{AppHandle, Runtime};

use super::operations::execute_operation;
use crate::{
    ApplyConvergence, ApplyReadback, ManagedDesktopReadback, ManagedWindowRegistry,
    ManagedWindowRegistryError, TauriApplyReceipt, TauriWindowFactory, WindowApplyAttempt,
    WindowApplyOutcome, WindowMutationBackend,
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

/// Registry plus complete native execution receipt.
pub struct TauriApplyOutcome<R: Runtime> {
    registry: ManagedWindowRegistry<R>,
    receipt: TauriApplyReceipt,
}

impl<R: Runtime> TauriApplyOutcome<R> {
    /// Returns the updated managed registry.
    #[must_use]
    pub const fn registry(&self) -> &ManagedWindowRegistry<R> {
        &self.registry
    }

    /// Consumes the outcome into the updated managed registry.
    #[must_use]
    pub fn into_registry(self) -> ManagedWindowRegistry<R> {
        self.registry
    }

    /// Returns the complete apply receipt.
    #[must_use]
    pub const fn receipt(&self) -> &TauriApplyReceipt {
        &self.receipt
    }

    /// Consumes the outcome into registry and receipt.
    #[must_use]
    pub fn into_parts(self) -> (ManagedWindowRegistry<R>, TauriApplyReceipt) {
        (self.registry, self.receipt)
    }
}

/// Executes one ordered nontransactional apply on the current main thread.
pub fn execute_tauri_window_apply<R, F, B, Q>(
    app: &AppHandle<R>,
    input: WindowDiffInput,
    mut registry: ManagedWindowRegistry<R>,
    mut factory: F,
    mut backend: B,
    mut readback: Q,
) -> Result<TauriApplyOutcome<R>, TauriApplyError>
where
    R: Runtime,
    F: TauriWindowFactory<R>,
    B: WindowMutationBackend<R>,
    Q: ManagedDesktopReadback<R>,
{
    let receipt = execute_tauri_window_apply_in_place(
        app,
        input,
        &mut registry,
        &mut factory,
        &mut backend,
        &mut readback,
    )?;
    Ok(TauriApplyOutcome { registry, receipt })
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
    let plan = plan_window_diff(&input).map_err(TauriApplyError::Planning)?;
    registry
        .begin_generation(plan.generation())
        .map_err(TauriApplyError::Registry)?;

    let mut blocked = BTreeMap::<WindowId, WindowOperationKind>::new();
    let mut attempts = Vec::with_capacity(plan.operations().len());
    for planned in plan.operations() {
        let operation = planned.operation();
        let window_id = operation.window_id().clone();
        if let Some(blocked_by) = blocked.get(&window_id).copied() {
            attempts.push(WindowApplyAttempt::new(
                planned.generation(),
                window_id,
                operation.transport_handle().cloned(),
                operation.kind(),
                WindowApplyOutcome::DependencySkipped { blocked_by },
            ));
            continue;
        }

        let attempt = execute_operation(
            app,
            registry,
            factory,
            backend,
            planned.generation(),
            operation,
        );
        if matches!(attempt.outcome(), WindowApplyOutcome::Failed { .. }) {
            blocked.insert(window_id, operation.kind());
        }
        attempts.push(attempt);
    }

    let readback = match readback.readback(app, registry) {
        Ok(observation) => {
            let convergence_input = input.with_live_windows(observation.windows().iter().cloned());
            let convergence = match plan_window_diff(&convergence_input) {
                Ok(receipt) => ApplyConvergence::Planned(
                    receipt.without_deferred(&tauri_deferred_settlement()),
                ),
                Err(error) => ApplyConvergence::Invalid(error),
            };
            ApplyReadback::Complete {
                observation,
                convergence,
            }
        }
        Err(error) => ApplyReadback::Failed(error),
    };

    Ok(TauriApplyReceipt::new(plan, attempts, readback))
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
