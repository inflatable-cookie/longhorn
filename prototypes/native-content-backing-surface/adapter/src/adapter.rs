use std::sync::{Arc, Mutex};

use longhorn_core::{PhysicalPoint, WindowId};
use longhorn_native_content_prototype::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, DetachPolicy, EffectiveFocus,
    EffectiveVisibility, InputRoutingMode, NativeContentFailureCode, NativeContentIslandId,
    NativeContentMechanism, NativeContentOperation, NativeContentRevision, ObservationUpdate,
    ObservedGeometry, ObservedReadiness, StepExecution,
};
use serde::Serialize;

use crate::runtime::contains;
use crate::{
    AdapterEvent, BackingSurfaceError, BackingSurfaceRuntime, DetachOutcome, InputAdmission,
    InputRejection, RuntimeAttachRequest, RuntimeEvent, RuntimeSnapshot,
};

/// Immutable island mapping and selected backing-view lifecycle policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackingSurfaceSpec {
    island_id: NativeContentIslandId,
    host_window_id: WindowId,
    detach_policy: DetachPolicy,
}

impl BackingSurfaceSpec {
    /// Constructs one mapping without renderer or product input authority.
    #[must_use]
    pub const fn new(
        island_id: NativeContentIslandId,
        host_window_id: WindowId,
        detach_policy: DetachPolicy,
    ) -> Self {
        Self {
            island_id,
            host_window_id,
            detach_policy,
        }
    }

    /// Returns shared island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns stable host-window binding.
    #[must_use]
    pub const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }

    /// Returns the selected native detach policy.
    #[must_use]
    pub const fn detach_policy(&self) -> DetachPolicy {
        self.detach_policy
    }
}

struct Attachment<H> {
    generation: AttachGeneration,
    handle: H,
    snapshot: RuntimeSnapshot,
    host_focused: bool,
}

struct AdapterState<H> {
    latest_generation: Option<AttachGeneration>,
    latest_desired_revision: Option<NativeContentRevision>,
    attachment: Option<Attachment<H>>,
}

impl<H> Default for AdapterState<H> {
    fn default() -> Self {
        Self {
            latest_generation: None,
            latest_desired_revision: None,
            attachment: None,
        }
    }
}

/// Attachment identity invalidated by host destruction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InvalidatedAttachment {
    island_id: NativeContentIslandId,
    generation: AttachGeneration,
    detach_outcome: DetachOutcome,
}

impl InvalidatedAttachment {
    /// Returns invalidated island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns invalidated attach generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns selected native ownership evidence.
    #[must_use]
    pub const fn detach_outcome(&self) -> DetachOutcome {
        self.detach_outcome
    }
}

/// Generation-checked backing-surface executor over one consumer runtime.
pub struct BackingSurfaceAdapter<R: BackingSurfaceRuntime> {
    runtime: R,
    spec: BackingSurfaceSpec,
    state: Arc<Mutex<AdapterState<R::Handle>>>,
    observer: Arc<dyn Fn(AdapterEvent) + Send + Sync>,
}

impl<R: BackingSurfaceRuntime> Clone for BackingSurfaceAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: self.state.clone(),
            observer: self.observer.clone(),
        }
    }
}

impl<R: BackingSurfaceRuntime> BackingSurfaceAdapter<R> {
    /// Creates one adapter from explicit native policy and observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: BackingSurfaceSpec,
        observer: Arc<dyn Fn(AdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable backing-surface policy.
    #[must_use]
    pub const fn spec(&self) -> &BackingSurfaceSpec {
        &self.spec
    }

    /// Executes one immutable backing-only plan with exact partial evidence.
    pub fn apply(&self, plan: &ApplyPlan) -> Result<ApplyReceipt, BackingSurfaceError> {
        self.validate_plan(plan)?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            state.latest_desired_revision = Some(plan.desired_revision());
        }
        let mut executions = Vec::new();
        for planned in plan.operations() {
            match self.execute(plan.generation(), planned.operation()) {
                Ok(()) => executions.push(StepExecution::Applied {
                    step: planned.step(),
                }),
                Err(error) => {
                    executions.push(StepExecution::Failed {
                        step: planned.step(),
                        code: NativeContentFailureCode::new(error.failure_code())
                            .expect("adapter failure codes use the bounded grammar"),
                    });
                    break;
                }
            }
        }
        ApplyReceipt::build(plan, executions)
            .map_err(|error| BackingSurfaceError::InvalidReceipt(error.to_string()))
    }

    /// Reads fresh storage and clip evidence without inventing focus or visibility.
    pub fn observe(
        &self,
        generation: AttachGeneration,
    ) -> Result<ObservationUpdate, BackingSurfaceError> {
        let handle = self.handle(generation)?;
        let snapshot = self.runtime.refresh(&handle)?;
        self.replace_snapshot(generation, snapshot.clone())?;
        Ok(observation(generation, &snapshot))
    }

    /// Refreshes full-host geometry after native resize or scale evidence changes.
    pub fn refresh_host_geometry(
        &self,
        generation: AttachGeneration,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        let handle = self.handle(generation)?;
        let snapshot = self.runtime.refresh(&handle)?;
        self.replace_snapshot(generation, snapshot.clone())?;
        Ok(snapshot)
    }

    /// Updates consumer-supplied host focus evidence used only by the input gate.
    pub fn update_host_focus(
        &self,
        generation: AttachGeneration,
        focused: bool,
    ) -> Result<(), BackingSurfaceError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        let attachment = state
            .attachment
            .as_mut()
            .ok_or(BackingSurfaceError::NotAttached)?;
        attachment.host_focused = focused;
        Ok(())
    }

    /// Gates one physical point before the consumer invokes its typed semantic callback.
    pub fn admit_input(
        &self,
        generation: AttachGeneration,
        point: PhysicalPoint,
    ) -> Result<InputAdmission, BackingSurfaceError> {
        let state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        let attachment = state
            .attachment
            .as_ref()
            .ok_or(BackingSurfaceError::NotAttached)?;
        let snapshot = &attachment.snapshot;
        if !snapshot.presentation_enabled {
            return Ok(InputAdmission::Rejected(
                InputRejection::PresentationDisabled,
            ));
        }
        if snapshot.clip.size().is_empty() {
            return Ok(InputAdmission::Rejected(InputRejection::EmptyViewport));
        }
        if !contains(&snapshot.clip, point) {
            return Ok(InputAdmission::Rejected(InputRejection::OutsideViewport));
        }
        if !attachment.host_focused {
            return Ok(InputAdmission::Rejected(InputRejection::HostUnfocused));
        }
        if snapshot.input_routing != InputRoutingMode::RendererForwarded {
            return Ok(InputAdmission::Rejected(InputRejection::RoutingDisabled));
        }
        Ok(InputAdmission::Admitted)
    }

    /// Admits a native callback only for the exact current host and generation.
    pub fn admit_runtime_event(&self, event: RuntimeEvent) -> Result<(), BackingSurfaceError> {
        if event.island_id != *self.spec.island_id()
            || event.host_window_id != *self.spec.host_window_id()
        {
            return Err(BackingSurfaceError::HostBindingMismatch);
        }
        {
            let state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            compare_generation(state.latest_generation, event.generation)?;
            let attachment = state
                .attachment
                .as_ref()
                .ok_or(BackingSurfaceError::NotAttached)?;
            if attachment.generation != event.generation {
                return Err(compare_attached_generation(
                    attachment.generation,
                    event.generation,
                ));
            }
        }
        self.emit(AdapterEvent::Runtime {
            generation: event.generation,
            event: event.kind,
        });
        Ok(())
    }

    /// Invalidates callback authority before applying the declared native release.
    pub fn host_destroyed(
        &self,
        host_window_id: &WindowId,
    ) -> Result<Option<InvalidatedAttachment>, BackingSurfaceError> {
        if host_window_id != self.spec.host_window_id() {
            return Ok(None);
        }
        let attachment = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            state.attachment.take()
        };
        let Some(attachment) = attachment else {
            return Ok(None);
        };
        self.emit(AdapterEvent::HostInvalidated {
            generation: attachment.generation,
        });
        let outcome = self.runtime.detach(&attachment.handle)?;
        self.emit(AdapterEvent::Detached {
            generation: attachment.generation,
            outcome,
        });
        Ok(Some(InvalidatedAttachment {
            island_id: self.spec.island_id().clone(),
            generation: attachment.generation,
            detach_outcome: outcome,
        }))
    }

    fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), BackingSurfaceError> {
        if plan.island_id() != self.spec.island_id() {
            return Err(BackingSurfaceError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: plan.island_id().clone(),
            });
        }
        if plan.operations().iter().any(|operation| {
            matches!(
                operation.operation(),
                NativeContentOperation::Attach { mechanism, .. }
                    if *mechanism != NativeContentMechanism::BackingSurface
            ) || matches!(
                operation.operation(),
                NativeContentOperation::SetChildBounds { .. }
                    | NativeContentOperation::SetIsolatedContentSize { .. }
            )
        }) {
            return Err(BackingSurfaceError::WrongMechanism);
        }
        let state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        if let Some(current) = state.latest_desired_revision {
            if plan.desired_revision() < current {
                return Err(BackingSurfaceError::StalePlan {
                    current,
                    supplied: plan.desired_revision(),
                });
            }
        }
        if let Some(attachment) = state.attachment.as_ref() {
            if plan.generation() != attachment.generation {
                return Err(compare_attached_generation(
                    attachment.generation,
                    plan.generation(),
                ));
            }
        } else {
            compare_generation_allow_next(state.latest_generation, plan.generation())?;
        }
        Ok(())
    }

    fn execute(
        &self,
        generation: AttachGeneration,
        operation: &NativeContentOperation,
    ) -> Result<(), BackingSurfaceError> {
        match operation {
            NativeContentOperation::Attach {
                host_window_id,
                mechanism: NativeContentMechanism::BackingSurface,
            } => {
                if host_window_id != self.spec.host_window_id() {
                    return Err(BackingSurfaceError::HostBindingMismatch);
                }
                self.attach(generation)
            }
            NativeContentOperation::SetBackingViewport { clip } => {
                let handle = self.handle(generation)?;
                let snapshot = self.runtime.set_viewport(&handle, *clip)?;
                self.replace_snapshot(generation, snapshot)
            }
            NativeContentOperation::Show => {
                let handle = self.handle(generation)?;
                let snapshot = self.runtime.set_presentation_enabled(&handle, true)?;
                self.replace_snapshot(generation, snapshot)
            }
            NativeContentOperation::Hide { .. } => {
                let handle = self.handle(generation)?;
                let snapshot = self.runtime.set_presentation_enabled(&handle, false)?;
                self.replace_snapshot(generation, snapshot)
            }
            NativeContentOperation::SetInputRouting { mode }
                if matches!(
                    mode,
                    InputRoutingMode::RendererForwarded | InputRoutingMode::Disabled
                ) =>
            {
                let handle = self.handle(generation)?;
                let snapshot = self.runtime.set_input_routing(&handle, *mode)?;
                self.replace_snapshot(generation, snapshot)
            }
            NativeContentOperation::SetInputRouting { .. } => {
                Err(BackingSurfaceError::UnsupportedInputMode)
            }
            NativeContentOperation::Detach { policy } if *policy == self.spec.detach_policy() => {
                self.detach(generation)
            }
            NativeContentOperation::Detach { .. } => {
                Err(BackingSurfaceError::UnsupportedDetachPolicy)
            }
            NativeContentOperation::RequestFocus | NativeContentOperation::ReleaseFocusIfOwned => {
                Err(BackingSurfaceError::Runtime {
                    operation: "focus",
                    detail: "backing-surface focus remains host evidence".to_string(),
                })
            }
            NativeContentOperation::Attach { .. }
            | NativeContentOperation::SetChildBounds { .. }
            | NativeContentOperation::SetIsolatedContentSize { .. } => {
                Err(BackingSurfaceError::WrongMechanism)
            }
        }
    }

    fn attach(&self, generation: AttachGeneration) -> Result<(), BackingSurfaceError> {
        {
            let state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            if state.attachment.is_some() {
                return Err(BackingSurfaceError::CurrentGenerationAttached(generation));
            }
        }
        self.emit(AdapterEvent::AttachStarted { generation });
        let (handle, snapshot) = self.runtime.attach(RuntimeAttachRequest {
            island_id: self.spec.island_id().clone(),
            generation,
            host_window_id: self.spec.host_window_id().clone(),
        })?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            compare_generation_allow_next(state.latest_generation, generation)?;
            state.latest_generation = Some(generation);
            state.attachment = Some(Attachment {
                generation,
                handle,
                snapshot,
                host_focused: false,
            });
        }
        self.emit(AdapterEvent::Attached { generation });
        Ok(())
    }

    fn detach(&self, generation: AttachGeneration) -> Result<(), BackingSurfaceError> {
        let attachment = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            state
                .attachment
                .take()
                .ok_or(BackingSurfaceError::NotAttached)?
        };
        let outcome = self.runtime.detach(&attachment.handle)?;
        self.emit(AdapterEvent::Detached {
            generation,
            outcome,
        });
        Ok(())
    }

    fn handle(&self, generation: AttachGeneration) -> Result<R::Handle, BackingSurfaceError> {
        let state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        let attachment = state
            .attachment
            .as_ref()
            .ok_or(BackingSurfaceError::NotAttached)?;
        if attachment.generation != generation {
            return Err(compare_attached_generation(
                attachment.generation,
                generation,
            ));
        }
        Ok(attachment.handle.clone())
    }

    fn replace_snapshot(
        &self,
        generation: AttachGeneration,
        snapshot: RuntimeSnapshot,
    ) -> Result<(), BackingSurfaceError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        let attachment = state
            .attachment
            .as_mut()
            .ok_or(BackingSurfaceError::NotAttached)?;
        attachment.snapshot = snapshot;
        Ok(())
    }

    fn emit(&self, event: AdapterEvent) {
        (self.observer)(event);
    }
}

fn observation(generation: AttachGeneration, snapshot: &RuntimeSnapshot) -> ObservationUpdate {
    ObservationUpdate::new(
        generation,
        AttachmentLifecycle::Attached,
        ObservedReadiness::Ready,
        EffectiveVisibility::Unknown,
        EffectiveFocus::Unknown,
        ObservedGeometry::BackingSurface {
            storage_bounds: snapshot.storage_bounds,
            clip: snapshot.clip,
        },
        Some(snapshot.input_routing),
    )
}

fn compare_generation(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), BackingSurfaceError> {
    let current = current.ok_or(BackingSurfaceError::NotAttached)?;
    if supplied < current {
        Err(BackingSurfaceError::StaleGeneration { current, supplied })
    } else if supplied > current {
        Err(BackingSurfaceError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

fn compare_generation_allow_next(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), BackingSurfaceError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied <= current {
        return Err(BackingSurfaceError::StaleGeneration { current, supplied });
    }
    let next = current
        .checked_next()
        .map_err(|error| BackingSurfaceError::Runtime {
            operation: "generation",
            detail: error.to_string(),
        })?;
    if supplied > next {
        Err(BackingSurfaceError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

fn compare_attached_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> BackingSurfaceError {
    if supplied < current {
        BackingSurfaceError::StaleGeneration { current, supplied }
    } else {
        BackingSurfaceError::FutureGeneration { current, supplied }
    }
}
