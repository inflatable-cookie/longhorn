use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapability, HostWindowHandle, PlannedWindowOperation,
    WindowApplyHost, WindowDiffDiagnostic, WindowDiffError, WindowDiffInput, WindowOperation,
    WindowOperationKind, accept_created_window, apply_window_diff,
};

use crate::{
    GpuiApplyAttempt, GpuiApplyConvergence, GpuiApplyError, GpuiApplyFailure, GpuiApplyFailureKind,
    GpuiApplyOutcome, GpuiApplyReadback, GpuiApplyReceipt, GpuiDesktopObservation,
    GpuiDisplayFactsSource, GpuiLogicalRect, GpuiLogicalSize, GpuiObservationError,
    GpuiWindowBackend, GpuiWindowCall, GpuiWindowCreateRequest, GpuiWindowRegistry,
    GpuiWindowRegistryError, gpui_deferred_settlement, gpui_host_capabilities,
    observe_gpui_desktop,
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
    ///
    /// Convergence is the fresh readback's remaining diff, after the host's
    /// declared deferred operations are set aside. Any remaining operation or
    /// diagnostic means the desired state is not there yet, and a readback
    /// whose diff could not be computed proves nothing.
    #[must_use]
    pub fn desired_state_reached(&self) -> bool {
        matches!(
            self.receipt.readback(),
            GpuiApplyReadback::Complete {
                convergence: GpuiApplyConvergence::Planned(remaining),
                ..
            } if remaining.is_empty()
        ) && self
            .dispositions
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

/// Why a lifecycle host could not complete an identity change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiRetagError {
    /// The lifecycle host did not move. The retag is refused, and the registry
    /// stays where it was.
    Refused(String),
    /// The lifecycle host moved the window to its new identity, but some of its
    /// pending deadlines could not be rescheduled. The registry follows the move,
    /// and the attempt reports the failure.
    Rescheduling(String),
}

/// Receives apply evidence before each native mutation.
///
/// Native events a mutation causes arrive later and look exactly like user
/// moves and resizes. The lifecycle coordinator tells them apart only from
/// evidence registered before the call, so a host's lifecycle state must see
/// this evidence. Tauri registers it through `ProgrammaticApplyObserver`. The
/// GPUI lifecycle host implements this trait for the same purpose.
pub trait GpuiApplyObserver {
    /// Registers one operation before its native call runs.
    ///
    /// # Errors
    /// An error refuses the operation. The adapter then makes no native call.
    fn register_apply(
        &mut self,
        generation: ApplyGeneration,
        operation: &WindowOperation,
    ) -> Result<(), String>;

    /// Moves lifecycle bookkeeping for `transport_handle` to `window_id`.
    ///
    /// Called only after the registry has validated the retag, and before the
    /// registry commits it. A [`GpuiRetagError::Refused`] leaves the registry
    /// unchanged. A [`GpuiRetagError::Rescheduling`] means the lifecycle host did
    /// move, so the registry commits too.
    ///
    /// # Errors
    /// See [`GpuiRetagError`].
    fn retag(
        &mut self,
        transport_handle: &HostWindowHandle,
        window_id: &WindowId,
    ) -> Result<(), GpuiRetagError>;
}

/// Executes one ordered nontransactional apply on the GPUI main thread.
///
/// The caller owns the backend for the duration of the call, because a GPUI
/// backend borrows the application context. There is no in-place variant that
/// leaves the registry with the caller across a suspension point: on this host
/// there is no suspension point to leave it across.
///
/// The observer receives each operation's evidence before its native call.
/// Pass the lifecycle host that will see the resulting window events.
///
/// The adapter reads desired state out of `input` because GPUI takes a
/// window's bounds, maximized state, focus and display as creation-time
/// options and cannot change the first two afterwards — it must know a
/// window's final placement before the window exists. Tauri can mutate after
/// creating and never had to ask, which is why `WindowDiffInput` showed
/// desired state only to the planner until a second host needed it.
pub fn execute_gpui_window_apply(
    input: WindowDiffInput,
    mut registry: GpuiWindowRegistry,
    backend: &mut impl GpuiWindowBackend,
    displays: &mut impl GpuiDisplayFactsSource,
    observer: &mut impl GpuiApplyObserver,
) -> Result<GpuiApplyOutcomeBundle, GpuiApplyError> {
    let input = input.with_capabilities(gpui_host_capabilities(backend.can_create()));
    let desired = desired_state(input.desired_windows());
    let mut host = GpuiApplyHost {
        registry: &mut registry,
        backend,
        displays,
        observer,
        desired: &desired,
    };
    let run = apply_window_diff(&mut host, input, &gpui_deferred_settlement())?;
    let dispositions = run
        .plan
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| classify(diagnostic, &run.created))
        .collect();
    let readback = match run.readback {
        Ok((observation, convergence)) => GpuiApplyReadback::Complete {
            observation,
            convergence: match convergence {
                Ok(plan) => GpuiApplyConvergence::Planned(plan),
                Err(error) => GpuiApplyConvergence::Invalid(error),
            },
        },
        Err(error) => GpuiApplyReadback::Failed(error),
    };
    Ok(GpuiApplyOutcomeBundle {
        registry,
        receipt: GpuiApplyReceipt::new(run.plan, run.attempts, readback),
        dispositions,
    })
}

struct GpuiApplyHost<'a, B, D, O> {
    registry: &'a mut GpuiWindowRegistry,
    backend: &'a mut B,
    displays: &'a mut D,
    observer: &'a mut O,
    desired: &'a BTreeMap<WindowId, DesiredState>,
}

impl<B, D, O> WindowApplyHost for GpuiApplyHost<'_, B, D, O>
where
    B: GpuiWindowBackend,
    D: GpuiDisplayFactsSource,
    O: GpuiApplyObserver,
{
    type Attempt = GpuiApplyAttempt;
    type Observation = GpuiDesktopObservation;
    type ApplyError = GpuiApplyError;
    type ReadbackError = GpuiObservationError;

    fn begin(&mut self, generation: ApplyGeneration) -> Result<(), Self::ApplyError> {
        self.registry
            .begin_generation(generation)
            .map_err(GpuiApplyError::Registry)?;
        let backend = &mut self.backend;
        self.registry
            .pending_discards
            .retain(|_, window| backend.close(*window).is_err());
        Ok(())
    }

    fn execute(&mut self, planned: &PlannedWindowOperation) -> (Self::Attempt, bool) {
        let attempt = execute_operation(
            self.registry,
            self.backend,
            self.observer,
            self.desired,
            planned.generation(),
            planned.operation(),
        );
        let succeeded = matches!(attempt.outcome(), GpuiApplyOutcome::Succeeded { .. });
        (attempt, succeeded)
    }

    fn skip(
        &self,
        planned: &PlannedWindowOperation,
        blocked_by: WindowOperationKind,
    ) -> Self::Attempt {
        let operation = planned.operation();
        GpuiApplyAttempt::new(
            planned.generation(),
            operation.window_id().clone(),
            operation.transport_handle().cloned(),
            operation.kind(),
            GpuiApplyOutcome::DependencySkipped { blocked_by },
        )
    }

    fn readback(&mut self) -> Result<Self::Observation, Self::ReadbackError> {
        if let Some((id, _)) = self.registry.pending_discards.first_key_value() {
            return Err(GpuiObservationError::PendingWindowDiscard(id.clone()));
        }
        observe_gpui_desktop(self.backend, self.registry, self.displays)
    }
}

impl From<WindowDiffError> for GpuiApplyError {
    fn from(error: WindowDiffError) -> Self {
        Self::Planning(error)
    }
}

#[derive(Clone, Copy, Debug)]
struct DesiredState {
    placement: WindowPlacement,
    maximized: bool,
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
                },
            )
        })
        .collect()
}

fn classify(
    diagnostic: &WindowDiffDiagnostic,
    created: &BTreeSet<WindowId>,
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
        HostCapability::Move if created.contains(window_id) => {
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
        // A hide is planned only for a window desired hidden, so a hide
        // diagnostic never has a visible target to call "already true". It
        // falls through to unsatisfiable, which is what GPUI cannot do.
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
    observer: &mut impl GpuiApplyObserver,
    desired: &BTreeMap<WindowId, DesiredState>,
    generation: ApplyGeneration,
    operation: &WindowOperation,
) -> GpuiApplyAttempt {
    match operation {
        WindowOperation::Retag {
            window_id,
            transport_handle,
        } => execute_retag(
            registry,
            observer,
            generation,
            window_id,
            transport_handle,
            operation,
        ),
        WindowOperation::Create { window_id } => execute_create(
            registry, backend, observer, desired, generation, window_id, operation,
        ),
        _ => execute_existing(registry, backend, observer, generation, operation),
    }
}

fn execute_retag(
    registry: &mut GpuiWindowRegistry,
    observer: &mut impl GpuiApplyObserver,
    generation: ApplyGeneration,
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
    // Validate both stores before either commits. The lifecycle host is
    // re-keyed first, so a registry refusal found afterwards would leave the
    // two disagreeing about which window is which.
    if let Err(error) = registry.check_retag(transport_handle, window_id) {
        return failed(
            generation,
            window_id.clone(),
            Some(transport_handle.clone()),
            WindowOperationKind::Retag,
            Vec::new(),
            registry_failure(GpuiWindowCall::RegistryRetag, &error),
        );
    }
    let retag_failure = match observer.retag(transport_handle, window_id) {
        Ok(()) => None,
        Err(GpuiRetagError::Refused(detail)) => {
            return failed(
                generation,
                window_id.clone(),
                Some(transport_handle.clone()),
                WindowOperationKind::Retag,
                Vec::new(),
                lifecycle_failure(detail),
            );
        }
        Err(GpuiRetagError::Rescheduling(detail)) => Some(lifecycle_failure(detail)),
    };
    registry.record_evidence(Some(transport_handle.clone()), operation.clone());
    match registry.retag(transport_handle, window_id.clone()) {
        Ok(()) => match retag_failure {
            None => succeeded(
                generation,
                window_id.clone(),
                Some(transport_handle.clone()),
                WindowOperationKind::Retag,
                vec![GpuiWindowCall::RegistryRetag],
            ),
            Some(failure) => failed(
                generation,
                window_id.clone(),
                Some(transport_handle.clone()),
                WindowOperationKind::Retag,
                vec![GpuiWindowCall::RegistryRetag],
                failure,
            ),
        },
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
    observer: &mut impl GpuiApplyObserver,
    desired: &BTreeMap<WindowId, DesiredState>,
    generation: ApplyGeneration,
    window_id: &WindowId,
    operation: &WindowOperation,
) -> GpuiApplyAttempt {
    if let Err(error) = registry.check_new_id(window_id) {
        return failed(
            generation,
            window_id.clone(),
            None,
            WindowOperationKind::Create,
            Vec::new(),
            registry_failure(GpuiWindowCall::RegistryInsert, &error),
        );
    }
    // Before the window exists, so the events its opening produces are
    // attributed to this apply and not to the user.
    if let Err(detail) = observer.register_apply(generation, operation) {
        return failed(
            generation,
            window_id.clone(),
            None,
            WindowOperationKind::Create,
            Vec::new(),
            lifecycle_failure(detail),
        );
    }
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

    match accept_created_window(
        key,
        |key| registry.insert_created(window_id.clone(), *key),
        |key| backend.close(*key),
    ) {
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
        Err(rejected) => {
            let failure = match rejected.cleanup {
                Ok(()) => {
                    completed.push(GpuiWindowCall::DiscardCreated);
                    registry_failure(GpuiWindowCall::RegistryInsert, &rejected.reason)
                }
                Err((key, error)) => {
                    registry.pending_discards.insert(window_id.clone(), key);
                    GpuiApplyFailure::new(
                        GpuiWindowCall::RegistryInsert,
                        GpuiApplyFailureKind::Registry,
                        format!(
                            "{}; could not discard created window: {error}",
                            rejected.reason
                        ),
                    )
                }
            };
            failed(
                generation,
                window_id.clone(),
                None,
                WindowOperationKind::Create,
                completed,
                failure,
            )
        }
    }
}

fn execute_existing(
    registry: &mut GpuiWindowRegistry,
    backend: &mut impl GpuiWindowBackend,
    observer: &mut impl GpuiApplyObserver,
    generation: ApplyGeneration,
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
    // Registered before the native call. A refused registration means the
    // call never happens, so no effect is left unattributed.
    if let Err(detail) = observer.register_apply(generation, operation) {
        return failed(
            generation,
            window_id,
            Some(handle),
            kind,
            Vec::new(),
            lifecycle_failure(detail),
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
        WindowOperation::Resize { inner_size, .. } => native_call(
            backend.resize(key, GpuiLogicalSize::from(*inner_size)),
            GpuiWindowCall::Resize,
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
        // A GPUI host withholds Move, Show and Hide, so the planner never
        // schedules them. Reaching this arm means the capability set and the
        // execution table disagreed.
        WindowOperation::Move { .. }
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
    generation: ApplyGeneration,
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
    generation: ApplyGeneration,
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

fn lifecycle_failure(detail: String) -> GpuiApplyFailure {
    GpuiApplyFailure::new(
        GpuiWindowCall::LifecycleBookkeeping,
        GpuiApplyFailureKind::Lifecycle,
        detail,
    )
}

fn registry_failure(call: GpuiWindowCall, error: &GpuiWindowRegistryError) -> GpuiApplyFailure {
    GpuiApplyFailure::new(call, GpuiApplyFailureKind::Registry, error.to_string())
}

fn native_failure(call: GpuiWindowCall, error: &crate::GpuiWindowError) -> GpuiApplyFailure {
    GpuiApplyFailure::new(call, GpuiApplyFailureKind::Native, error.detail())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use longhorn_core::WindowId;
    use longhorn_windowing::{HostCapability, WindowDiffDiagnostic, WindowOperationKind};

    use super::{GpuiDiagnosticDisposition, classify};

    #[test]
    fn a_hide_the_host_cannot_make_is_unsatisfiable_never_already_true() {
        // A hide is planned only for a window desired hidden, so no hide can
        // ever be "already true" by being desired visible. The host cannot
        // hide, and that is what the disposition must say.
        let window_id = WindowId::new("main").unwrap();
        let diagnostic = WindowDiffDiagnostic::UnsupportedOperation {
            operation: WindowOperationKind::Hide,
            window_id: window_id.clone(),
            transport_handle: None,
            required_capability: HostCapability::Hide,
        };

        assert_eq!(
            classify(&diagnostic, &BTreeSet::new()),
            Some(GpuiDiagnosticDisposition::Unsatisfiable {
                window_id,
                operation: WindowOperationKind::Hide,
                capability: HostCapability::Hide,
            })
        );
    }
}
