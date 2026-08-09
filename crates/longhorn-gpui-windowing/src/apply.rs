use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::{
    DesiredWindow, HostCapability, HostWindowHandle, WindowDiffDiagnostic, WindowDiffInput,
    WindowOperation, WindowOperationKind, plan_window_diff,
};

use crate::{
    GpuiApplyAttempt, GpuiApplyConvergence, GpuiApplyError, GpuiApplyFailure, GpuiApplyFailureKind,
    GpuiApplyOutcome, GpuiApplyReadback, GpuiApplyReceipt, GpuiDisplayFactsSource, GpuiLogicalRect,
    GpuiLogicalSize, GpuiWindowBackend, GpuiWindowCall, GpuiWindowCreateRequest,
    GpuiWindowRegistry, GpuiWindowRegistryError, gpui_host_capabilities, observe_gpui_desktop,
};

/// What became of one plan diagnostic a GPUI host produced.
///
/// The pure planner emits `UnsupportedOperation` for every capability a host
/// withholds. On GPUI some of those are real refusals and some are artefacts
/// of an operation vocabulary compiled from a host that can mutate a window
/// after creating it. This distinction is the adapter's answer, not the
/// planner's, so it is reported rather than hidden.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiDiagnosticDisposition {
    /// The intent was satisfied by GPUI's creation-time options instead.
    SatisfiedAtCreate {
        /// Logical target.
        window_id: WindowId,
        /// The operation the planner could not schedule.
        operation: WindowOperationKind,
    },
    /// The intent was already true, so no operation was needed.
    AlreadyTrue {
        /// Logical target.
        window_id: WindowId,
        /// The operation the planner could not schedule.
        operation: WindowOperationKind,
        /// Why it needed nothing.
        reason: &'static str,
    },
    /// The host genuinely cannot do this, and the desired state was not reached.
    Unsatisfiable {
        /// Logical target.
        window_id: WindowId,
        /// The operation the planner could not schedule.
        operation: WindowOperationKind,
        /// The capability the host does not have.
        capability: HostCapability,
    },
}

impl GpuiDiagnosticDisposition {
    /// Returns whether desired state was actually reached despite the diagnostic.
    #[must_use]
    pub const fn is_reached(&self) -> bool {
        matches!(
            self,
            Self::SatisfiedAtCreate { .. } | Self::AlreadyTrue { .. }
        )
    }

    /// Returns the logical target.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        match self {
            Self::SatisfiedAtCreate { window_id, .. }
            | Self::AlreadyTrue { window_id, .. }
            | Self::Unsatisfiable { window_id, .. } => window_id,
        }
    }
}

/// Registry plus complete execution receipt.
#[derive(Clone, Debug)]
pub struct GpuiApplyOutcomeBundle {
    registry: GpuiWindowRegistry,
    receipt: GpuiApplyReceipt,
    dispositions: Vec<GpuiDiagnosticDisposition>,
}

impl GpuiApplyOutcomeBundle {
    /// Returns the updated managed registry.
    #[must_use]
    pub const fn registry(&self) -> &GpuiWindowRegistry {
        &self.registry
    }

    /// Returns the complete apply receipt.
    #[must_use]
    pub const fn receipt(&self) -> &GpuiApplyReceipt {
        &self.receipt
    }

    /// Returns what became of every plan diagnostic.
    #[must_use]
    pub fn dispositions(&self) -> &[GpuiDiagnosticDisposition] {
        &self.dispositions
    }

    /// Returns whether every desired window reached its desired state.
    #[must_use]
    pub fn desired_state_reached(&self) -> bool {
        self.dispositions
            .iter()
            .all(GpuiDiagnosticDisposition::is_reached)
            && self
                .receipt
                .attempts()
                .iter()
                .all(|attempt| matches!(attempt.outcome(), GpuiApplyOutcome::Succeeded { .. }))
    }

    /// Consumes the bundle into registry and receipt.
    #[must_use]
    pub fn into_parts(self) -> (GpuiWindowRegistry, GpuiApplyReceipt) {
        (self.registry, self.receipt)
    }
}

/// Executes one ordered nontransactional apply on the GPUI main thread.
///
/// The caller owns the backend for the duration of the call, because a GPUI
/// backend borrows the application context. There is no in-place variant that
/// leaves the registry with the caller across a suspension point: on this host
/// there is no suspension point to leave it across.
///
/// `desired_windows` repeats what the caller already put in `input`. That is
/// not a convenience — GPUI takes a window's bounds, maximized state, focus
/// and display as creation-time options and cannot change the first two
/// afterwards, so the adapter must know a window's final placement before the
/// window exists. `WindowDiffInput` exposes desired state only to the planner,
/// because Tauri can mutate after creating and never had to ask. Making it
/// readable is the right fix and is recorded against contract 020; it is a
/// change to a crate the current release candidate has frozen, so the
/// parameter carries it until that freeze lifts.
pub fn execute_gpui_window_apply(
    input: WindowDiffInput,
    desired_windows: &[DesiredWindow],
    mut registry: GpuiWindowRegistry,
    backend: &mut impl GpuiWindowBackend,
    displays: &mut impl GpuiDisplayFactsSource,
) -> Result<GpuiApplyOutcomeBundle, GpuiApplyError> {
    let input = input.with_capabilities(gpui_host_capabilities(backend.can_create()));
    let desired = desired_state(desired_windows);
    let plan = plan_window_diff(&input).map_err(GpuiApplyError::Planning)?;
    registry
        .begin_generation(plan.generation())
        .map_err(GpuiApplyError::Registry)?;

    let mut blocked = BTreeMap::<WindowId, WindowOperationKind>::new();
    let mut created = BTreeSet::<WindowId>::new();
    let mut attempts = Vec::with_capacity(plan.operations().len());
    for planned in plan.operations() {
        let operation = planned.operation();
        let window_id = operation.window_id().clone();
        if let Some(blocked_by) = blocked.get(&window_id).copied() {
            attempts.push(GpuiApplyAttempt::new(
                planned.generation(),
                window_id,
                operation.transport_handle().cloned(),
                operation.kind(),
                GpuiApplyOutcome::DependencySkipped { blocked_by },
            ));
            continue;
        }

        let attempt = execute_operation(
            &mut registry,
            backend,
            &desired,
            planned.generation(),
            operation,
        );
        match attempt.outcome() {
            GpuiApplyOutcome::Failed { .. } => {
                blocked.insert(window_id, operation.kind());
            }
            GpuiApplyOutcome::Succeeded { .. }
                if operation.kind() == WindowOperationKind::Create =>
            {
                created.insert(window_id);
            }
            _ => {}
        }
        attempts.push(attempt);
    }

    let dispositions = plan
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| classify(diagnostic, &created, &desired))
        .collect();

    let readback = match observe_gpui_desktop(backend, &registry, displays) {
        Ok(observation) => {
            let convergence_input = input.with_live_windows(observation.windows().iter().cloned());
            let convergence = match plan_window_diff(&convergence_input) {
                Ok(receipt) => GpuiApplyConvergence::Planned(receipt),
                Err(error) => GpuiApplyConvergence::Invalid(error),
            };
            GpuiApplyReadback::Complete {
                observation,
                convergence,
            }
        }
        Err(error) => GpuiApplyReadback::Failed(error),
    };

    Ok(GpuiApplyOutcomeBundle {
        registry,
        receipt: GpuiApplyReceipt::new(plan, attempts, readback),
        dispositions,
    })
}

#[derive(Clone, Copy, Debug)]
struct DesiredState {
    placement: WindowPlacement,
    maximized: bool,
    visible: bool,
}

fn desired_state(desired_windows: &[DesiredWindow]) -> BTreeMap<WindowId, DesiredState> {
    desired_windows
        .iter()
        .map(|desired| {
            (
                desired.window_id().clone(),
                DesiredState {
                    placement: desired.placement(),
                    maximized: desired.is_maximized(),
                    visible: desired.is_visible(),
                },
            )
        })
        .collect()
}

fn classify(
    diagnostic: &WindowDiffDiagnostic,
    created: &BTreeSet<WindowId>,
    desired: &BTreeMap<WindowId, DesiredState>,
) -> Option<GpuiDiagnosticDisposition> {
    let WindowDiffDiagnostic::UnsupportedOperation {
        operation,
        window_id,
        required_capability,
        ..
    } = diagnostic
    else {
        return None;
    };
    let disposition = match required_capability {
        HostCapability::MoveResize if created.contains(window_id) => {
            GpuiDiagnosticDisposition::SatisfiedAtCreate {
                window_id: window_id.clone(),
                operation: *operation,
            }
        }
        HostCapability::Show => GpuiDiagnosticDisposition::AlreadyTrue {
            window_id: window_id.clone(),
            operation: *operation,
            reason: "a gpui window is on screen from creation",
        },
        HostCapability::Hide if desired.get(window_id).is_some_and(|state| state.visible) => {
            GpuiDiagnosticDisposition::AlreadyTrue {
                window_id: window_id.clone(),
                operation: *operation,
                reason: "the window is desired visible, so nothing had to hide",
            }
        }
        capability => GpuiDiagnosticDisposition::Unsatisfiable {
            window_id: window_id.clone(),
            operation: *operation,
            capability: *capability,
        },
    };
    Some(disposition)
}

fn execute_operation(
    registry: &mut GpuiWindowRegistry,
    backend: &mut impl GpuiWindowBackend,
    desired: &BTreeMap<WindowId, DesiredState>,
    generation: longhorn_windowing::ApplyGeneration,
    operation: &WindowOperation,
) -> GpuiApplyAttempt {
    match operation {
        WindowOperation::Retag {
            window_id,
            transport_handle,
        } => execute_retag(registry, generation, window_id, transport_handle, operation),
        WindowOperation::Create { window_id } => {
            execute_create(registry, backend, desired, generation, window_id, operation)
        }
        _ => execute_existing(registry, backend, generation, operation),
    }
}

fn execute_retag(
    registry: &mut GpuiWindowRegistry,
    generation: longhorn_windowing::ApplyGeneration,
    window_id: &WindowId,
    transport_handle: &HostWindowHandle,
    operation: &WindowOperation,
) -> GpuiApplyAttempt {
    if !registry.contains_handle(transport_handle) {
        return failed(
            generation,
            window_id.clone(),
            Some(transport_handle.clone()),
            WindowOperationKind::Retag,
            Vec::new(),
            registry_failure(
                GpuiWindowCall::ResolveManagedWindow,
                &GpuiWindowRegistryError::UnknownTransportHandle(transport_handle.clone()),
            ),
        );
    }
    registry.record_evidence(Some(transport_handle.clone()), operation.clone());
    match registry.retag(transport_handle, window_id.clone()) {
        Ok(()) => succeeded(
            generation,
            window_id.clone(),
            Some(transport_handle.clone()),
            WindowOperationKind::Retag,
            vec![GpuiWindowCall::RegistryRetag],
        ),
        Err(error) => failed(
            generation,
            window_id.clone(),
            Some(transport_handle.clone()),
            WindowOperationKind::Retag,
            Vec::new(),
            registry_failure(GpuiWindowCall::RegistryRetag, &error),
        ),
    }
}

fn execute_create(
    registry: &mut GpuiWindowRegistry,
    backend: &mut impl GpuiWindowBackend,
    desired: &BTreeMap<WindowId, DesiredState>,
    generation: longhorn_windowing::ApplyGeneration,
    window_id: &WindowId,
    operation: &WindowOperation,
) -> GpuiApplyAttempt {
    registry.record_evidence(None, operation.clone());

    // The plan says "create a neutral hidden unmaximized slot". GPUI has no
    // such slot: bounds, maximized state and initial focus are `WindowOptions`
    // fields and two of the three cannot be changed afterwards. So the adapter
    // reads the desired state the plan was derived from and opens the window
    // in it directly.
    let Some(state) = desired.get(window_id) else {
        return failed(
            generation,
            window_id.clone(),
            None,
            WindowOperationKind::Create,
            Vec::new(),
            GpuiApplyFailure::new(
                GpuiWindowCall::ComposeCreateRequest,
                GpuiApplyFailureKind::CreateComposition,
                "no desired state for a planned create; gpui cannot open a neutral slot",
            ),
        );
    };
    let mut request = GpuiWindowCreateRequest::new(placement_bounds(state.placement));
    if state.maximized {
        request = request.maximized();
    }
    let completed = vec![GpuiWindowCall::ComposeCreateRequest];

    let key = match backend.create(window_id, &request) {
        Ok(key) => key,
        Err(error) => {
            return failed(
                generation,
                window_id.clone(),
                None,
                WindowOperationKind::Create,
                completed,
                native_failure(GpuiWindowCall::OpenWindow, &error),
            );
        }
    };
    let mut completed = completed;
    completed.push(GpuiWindowCall::OpenWindow);

    match registry.insert_created(window_id.clone(), key) {
        Ok(handle) => {
            completed.push(GpuiWindowCall::RegistryInsert);
            succeeded(
                generation,
                window_id.clone(),
                Some(handle),
                WindowOperationKind::Create,
                completed,
            )
        }
        Err(error) => failed(
            generation,
            window_id.clone(),
            None,
            WindowOperationKind::Create,
            completed,
            registry_failure(GpuiWindowCall::RegistryInsert, &error),
        ),
    }
}

fn execute_existing(
    registry: &mut GpuiWindowRegistry,
    backend: &mut impl GpuiWindowBackend,
    generation: longhorn_windowing::ApplyGeneration,
    operation: &WindowOperation,
) -> GpuiApplyAttempt {
    let window_id = operation.window_id().clone();
    let kind = operation.kind();
    let (handle, key) = match registry.resolve(&window_id, operation.transport_handle()) {
        Ok(resolved) => resolved,
        Err(error) => {
            return failed(
                generation,
                window_id,
                operation.transport_handle().cloned(),
                kind,
                Vec::new(),
                registry_failure(GpuiWindowCall::ResolveManagedWindow, &error),
            );
        }
    };
    if matches!(operation, WindowOperation::Close { .. }) && registry.is_protected_primary(&handle)
    {
        return failed(
            generation,
            window_id,
            Some(handle),
            kind,
            Vec::new(),
            GpuiApplyFailure::new(
                GpuiWindowCall::ProtectPrimary,
                GpuiApplyFailureKind::ProtectedPrimary,
                "protected primary cannot be closed",
            ),
        );
    }
    registry.record_evidence(Some(handle.clone()), operation.clone());

    let mut completed = Vec::new();
    let result = match operation {
        WindowOperation::Unmaximize { .. } => native_call(
            backend.set_maximized(key, false),
            GpuiWindowCall::SetMaximized,
            &mut completed,
        ),
        WindowOperation::Maximize { .. } => native_call(
            backend.set_maximized(key, true),
            GpuiWindowCall::SetMaximized,
            &mut completed,
        ),
        WindowOperation::Focus { .. } => native_call(
            backend.activate(key),
            GpuiWindowCall::Activate,
            &mut completed,
        ),
        WindowOperation::Close { .. } => {
            native_call(backend.close(key), GpuiWindowCall::Close, &mut completed)
        }
        // A GPUI host withholds MoveResize, Show and Hide, so the planner never
        // schedules them. Reaching this arm means the capability set and the
        // execution table disagreed.
        WindowOperation::MoveResize { .. }
        | WindowOperation::Show { .. }
        | WindowOperation::Hide { .. }
        | WindowOperation::Retag { .. }
        | WindowOperation::Create { .. } => Err(GpuiApplyFailure::new(
            GpuiWindowCall::ResolveManagedWindow,
            GpuiApplyFailureKind::Native,
            format!("gpui host declared no capability for {kind:?}"),
        )),
    };

    match result {
        Ok(()) => {
            if matches!(operation, WindowOperation::Close { .. }) {
                registry.remove_closed(&handle);
            }
            succeeded(generation, window_id, Some(handle), kind, completed)
        }
        Err(failure) => failed(
            generation,
            window_id,
            Some(handle),
            kind,
            completed,
            failure,
        ),
    }
}

fn placement_bounds(placement: WindowPlacement) -> GpuiLogicalRect {
    // GPUI's window bounds are outer bounds, and its content size is set
    // separately. Longhorn's placement is an outer origin with an inner size,
    // so the create request carries the inner size as the initial extent and
    // the frame grows around it. A host that needs the distinction exact
    // resizes after creation, which is why `resize` is in the seam even though
    // `MoveResize` is not a declared capability.
    let origin = placement.outer_origin();
    let size = GpuiLogicalSize::from(placement.inner_size());
    GpuiLogicalRect::new(
        origin.x().get() as f32,
        origin.y().get() as f32,
        size.width(),
        size.height(),
    )
}

fn native_call(
    result: Result<(), crate::GpuiWindowError>,
    call: GpuiWindowCall,
    completed: &mut Vec<GpuiWindowCall>,
) -> Result<(), GpuiApplyFailure> {
    match result {
        Ok(()) => {
            completed.push(call);
            Ok(())
        }
        Err(error) => Err(native_failure(call, &error)),
    }
}

fn succeeded(
    generation: longhorn_windowing::ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperationKind,
    completed_calls: Vec<GpuiWindowCall>,
) -> GpuiApplyAttempt {
    GpuiApplyAttempt::new(
        generation,
        window_id,
        transport_handle,
        operation,
        GpuiApplyOutcome::Succeeded { completed_calls },
    )
}

fn failed(
    generation: longhorn_windowing::ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperationKind,
    completed_calls: Vec<GpuiWindowCall>,
    failure: GpuiApplyFailure,
) -> GpuiApplyAttempt {
    GpuiApplyAttempt::new(
        generation,
        window_id,
        transport_handle,
        operation,
        GpuiApplyOutcome::Failed {
            completed_calls,
            failure,
        },
    )
}

fn registry_failure(call: GpuiWindowCall, error: &GpuiWindowRegistryError) -> GpuiApplyFailure {
    GpuiApplyFailure::new(call, GpuiApplyFailureKind::Registry, error.to_string())
}

fn native_failure(call: GpuiWindowCall, error: &crate::GpuiWindowError) -> GpuiApplyFailure {
    GpuiApplyFailure::new(call, GpuiApplyFailureKind::Native, error.detail())
}
