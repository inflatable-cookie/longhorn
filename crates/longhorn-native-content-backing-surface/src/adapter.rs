use std::sync::{Arc, Mutex};

use longhorn_core::{NativeContentFailureCode, PhysicalPoint, WindowId};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, DetachPolicy, EffectiveFocus,
    EffectiveVisibility, InputRoutingMode, NativeContentCoordinator, NativeContentIslandId,
    NativeContentMechanism, NativeContentOperation, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, StepExecution,
};
use serde::Serialize;

use crate::runtime::contains;
use crate::{
    BackingSurfaceAdapterEvent, BackingSurfaceError, BackingSurfaceRuntime,
    BackingSurfaceRuntimeEvent, BackingSurfaceRuntimeEventKind, BackingSurfaceSnapshot,
    BackingSurfaceSpec, InputAdmission, InputRejection, RuntimeAttachRequest,
};

/// Exact result of one reversible detach request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackingSurfaceDetachOutcome {
    /// Current storage and renderer resources were detached.
    Detached,
    /// The generation had already detached successfully.
    AlreadyDetached,
}

/// Adapter-local reversible detach evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackingSurfaceDetachReceipt {
    island_id: NativeContentIslandId,
    generation: AttachGeneration,
    outcome: BackingSurfaceDetachOutcome,
}

impl BackingSurfaceDetachReceipt {
    /// Returns the detached island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the exact generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns whether this call detached or confirmed prior detach.
    #[must_use]
    pub const fn outcome(&self) -> BackingSurfaceDetachOutcome {
        self.outcome
    }
}

/// Local host-destruction invalidation result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackingSurfaceHostDestroyOutcome {
    /// This call invalidated current callback authority.
    Invalidated,
    /// Callback authority was already invalidated.
    AlreadyInvalidated,
}

/// Exact local invalidation and reversible-detach evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackingSurfaceHostDestroyReceipt {
    island_id: NativeContentIslandId,
    generation: AttachGeneration,
    outcome: BackingSurfaceHostDestroyOutcome,
    detach: BackingSurfaceDetachOutcome,
}

impl BackingSurfaceHostDestroyReceipt {
    /// Returns the invalidated island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the invalidated generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns whether this call established local invalidation.
    #[must_use]
    pub const fn outcome(&self) -> BackingSurfaceHostDestroyOutcome {
        self.outcome
    }

    /// Returns exact reversible-detach evidence.
    #[must_use]
    pub const fn detach(&self) -> BackingSurfaceDetachOutcome {
        self.detach
    }
}

struct Attachment<H> {
    generation: AttachGeneration,
    handle: Option<H>,
    snapshot: Option<BackingSurfaceSnapshot>,
    host_focused: bool,
    detaching: bool,
    last_event_sequence: u64,
}

struct AdapterState<H> {
    latest_generation: Option<AttachGeneration>,
    retired_generation: Option<AttachGeneration>,
    invalidated_generation: Option<AttachGeneration>,
    attachment: Option<Attachment<H>>,
}

impl<H> Default for AdapterState<H> {
    fn default() -> Self {
        Self {
            latest_generation: None,
            retired_generation: None,
            invalidated_generation: None,
            attachment: None,
        }
    }
}

/// Generation-checked backing-surface executor over one consumer runtime port.
pub struct BackingSurfaceAdapter<R: BackingSurfaceRuntime> {
    runtime: R,
    spec: BackingSurfaceSpec,
    state: Arc<Mutex<AdapterState<R::Handle>>>,
    observer: Arc<dyn Fn(BackingSurfaceAdapterEvent) + Send + Sync>,
}

impl<R: BackingSurfaceRuntime> Clone for BackingSurfaceAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: Arc::clone(&self.state),
            observer: Arc::clone(&self.observer),
        }
    }
}

impl<R: BackingSurfaceRuntime> BackingSurfaceAdapter<R> {
    /// Creates one adapter from an explicit mapping and lifecycle observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: BackingSurfaceSpec,
        observer: Arc<dyn Fn(BackingSurfaceAdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable island and host mapping.
    #[must_use]
    pub const fn spec(&self) -> &BackingSurfaceSpec {
        &self.spec
    }

    /// Executes a current immutable plan and returns coordinator-validated evidence.
    pub fn apply(
        &self,
        authority: &NativeContentCoordinator,
        plan: &ApplyPlan,
    ) -> Result<ApplyReceipt, BackingSurfaceError> {
        self.validate_plan(plan)?;
        authority.receipt(plan, std::iter::empty())?;

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
        authority.receipt(plan, executions).map_err(Into::into)
    }

    /// Reads fresh storage and render evidence without fabricating focus or visibility.
    pub fn observe(
        &self,
        generation: AttachGeneration,
    ) -> Result<ObservationUpdate, BackingSurfaceError> {
        let target = {
            let state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            if state.retired_generation == Some(generation)
                || state.invalidated_generation == Some(generation)
            {
                None
            } else {
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
                Some((attachment.handle.clone(), attachment.detaching))
            }
        };
        let Some((handle, detaching)) = target else {
            return Ok(ObservationUpdate::absent(generation));
        };
        let Some(handle) = handle else {
            return Ok(ObservationUpdate::new(
                generation,
                AttachmentLifecycle::Attaching,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::Unknown,
                Some(InputRoutingMode::Disabled),
            ));
        };
        let snapshot = self.runtime.observe(&handle)?;
        self.replace_snapshot(generation, snapshot.clone())?;
        Ok(observation(generation, &snapshot, detaching))
    }

    /// Refreshes full-host storage after native host resize or scale change.
    pub fn refresh_host_geometry(
        &self,
        generation: AttachGeneration,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError> {
        let handle = self.handle(generation)?;
        let snapshot = self.runtime.observe(&handle)?;
        self.replace_snapshot(generation, snapshot.clone())?;
        Ok(snapshot)
    }

    /// Updates consumer-supplied host focus used only by the physical input gate.
    pub fn update_host_focus(
        &self,
        generation: AttachGeneration,
        focused: bool,
    ) -> Result<(), BackingSurfaceError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        let attachment = current_attachment_mut(&mut state, generation)?;
        attachment.host_focused = focused;
        Ok(())
    }

    /// Gates one physical point before the consumer dispatches typed semantic input.
    pub fn admit_input(
        &self,
        generation: AttachGeneration,
        point: PhysicalPoint,
    ) -> Result<InputAdmission, BackingSurfaceError> {
        let state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        let attachment = current_attachment(&state, generation)?;
        let snapshot = attachment
            .snapshot
            .as_ref()
            .ok_or(BackingSurfaceError::AttachInProgress)?;
        if !snapshot.presentation_enabled {
            return Ok(InputAdmission::Rejected(
                InputRejection::PresentationDisabled,
            ));
        }
        if snapshot.clip.size().is_empty() || snapshot.storage_bounds.size().is_empty() {
            return Ok(InputAdmission::Rejected(InputRejection::EmptyViewport));
        }
        if !contains(&snapshot.clip, point) {
            return Ok(InputAdmission::Rejected(InputRejection::OutsideViewport));
        }
        if !contains(&snapshot.storage_bounds, point) {
            return Ok(InputAdmission::Rejected(InputRejection::OutsideStorage));
        }
        if !attachment.host_focused {
            return Ok(InputAdmission::Rejected(InputRejection::HostUnfocused));
        }
        if snapshot.input_routing != InputRoutingMode::RendererForwarded {
            return Ok(InputAdmission::Rejected(InputRejection::RoutingDisabled));
        }
        Ok(InputAdmission::Admitted)
    }

    /// Admits one exact current runtime callback and rejects duplicate ordering.
    pub fn admit_runtime_event(
        &self,
        event: BackingSurfaceRuntimeEvent,
    ) -> Result<(), BackingSurfaceError> {
        if event.island_id != *self.spec.island_id() {
            return Err(BackingSurfaceError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: event.island_id,
            });
        }
        if event.host_window_id != *self.spec.host_window_id() {
            return Err(BackingSurfaceError::HostBindingMismatch);
        }
        let generation = event.generation;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            let attachment = current_attachment_mut(&mut state, generation)?;
            if event.sequence <= attachment.last_event_sequence {
                return Err(BackingSurfaceError::StaleEventSequence {
                    current: attachment.last_event_sequence,
                    supplied: event.sequence,
                });
            }
            if let Some(snapshot) = attachment.snapshot.as_mut() {
                match event.kind {
                    BackingSurfaceRuntimeEventKind::FramePresented { sequence } => {
                        if sequence < snapshot.frame_sequence {
                            return Err(BackingSurfaceError::StaleFrameSequence {
                                current: snapshot.frame_sequence,
                                supplied: sequence,
                            });
                        }
                        snapshot.frame_sequence = sequence;
                    }
                    BackingSurfaceRuntimeEventKind::StorageChanged { bounds } => {
                        snapshot.storage_bounds = bounds;
                    }
                }
            }
            attachment.last_event_sequence = event.sequence;
        }
        self.emit(BackingSurfaceAdapterEvent::Runtime {
            generation,
            event: event.kind,
        });
        Ok(())
    }

    /// Invalidates callbacks before reversible native detach after host destruction.
    pub fn host_destroyed(
        &self,
        host_window_id: &WindowId,
        generation: AttachGeneration,
    ) -> Result<BackingSurfaceHostDestroyReceipt, BackingSurfaceError> {
        if host_window_id != self.spec.host_window_id() {
            return Err(BackingSurfaceError::HostBindingMismatch);
        }
        let invalidation = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            if state.invalidated_generation == Some(generation) {
                BackingSurfaceHostDestroyOutcome::AlreadyInvalidated
            } else {
                state.invalidated_generation = Some(generation);
                BackingSurfaceHostDestroyOutcome::Invalidated
            }
        };
        if invalidation == BackingSurfaceHostDestroyOutcome::Invalidated {
            self.emit(BackingSurfaceAdapterEvent::HostInvalidated { generation });
        }
        let detach = self.detach_generation(generation)?;
        Ok(BackingSurfaceHostDestroyReceipt {
            island_id: self.spec.island_id().clone(),
            generation,
            outcome: invalidation,
            detach,
        })
    }

    /// Explicitly performs or confirms reversible detach for one generation.
    pub fn detach(
        &self,
        generation: AttachGeneration,
    ) -> Result<BackingSurfaceDetachReceipt, BackingSurfaceError> {
        let outcome = self.detach_generation(generation)?;
        Ok(BackingSurfaceDetachReceipt {
            island_id: self.spec.island_id().clone(),
            generation,
            outcome,
        })
    }

    fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), BackingSurfaceError> {
        if plan.island_id() != self.spec.island_id() {
            return Err(BackingSurfaceError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: plan.island_id().clone(),
            });
        }
        if plan.operations().iter().any(|planned| {
            matches!(
                planned.operation(),
                NativeContentOperation::Attach { mechanism, .. }
                    if *mechanism != NativeContentMechanism::BackingSurface
            ) || matches!(
                planned.operation(),
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
        if let Some(attachment) = state.attachment.as_ref() {
            if plan.generation() < attachment.generation {
                return Err(BackingSurfaceError::StaleGeneration {
                    current: attachment.generation,
                    supplied: plan.generation(),
                });
            }
            if plan.generation() > attachment.generation {
                return Err(BackingSurfaceError::CurrentGenerationAttached(
                    attachment.generation,
                ));
            }
            if state.invalidated_generation == Some(plan.generation()) {
                return Err(BackingSurfaceError::GenerationInvalidated(
                    plan.generation(),
                ));
            }
        } else {
            compare_generation_allow_next(state.latest_generation, plan.generation())?;
            if state.retired_generation == Some(plan.generation())
                && plan.operations().iter().any(|planned| {
                    matches!(planned.operation(), NativeContentOperation::Attach { .. })
                })
            {
                return Err(BackingSurfaceError::GenerationRetired(plan.generation()));
            }
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
            NativeContentOperation::SetInputRouting {
                mode: InputRoutingMode::RendererForwarded | InputRoutingMode::Disabled,
            } => {
                let mode = match operation {
                    NativeContentOperation::SetInputRouting { mode } => *mode,
                    _ => unreachable!(),
                };
                let handle = self.handle(generation)?;
                let snapshot = self.runtime.set_input_routing(&handle, mode)?;
                self.replace_snapshot(generation, snapshot)
            }
            NativeContentOperation::SetInputRouting { .. } => {
                Err(BackingSurfaceError::UnsupportedInputMode)
            }
            NativeContentOperation::RequestFocus | NativeContentOperation::ReleaseFocusIfOwned => {
                Err(BackingSurfaceError::UnsupportedFocusOperation)
            }
            NativeContentOperation::Detach {
                policy: DetachPolicy::Reversible,
            } => self.detach_generation(generation).map(|_| ()),
            NativeContentOperation::Detach { .. } => {
                Err(BackingSurfaceError::UnsupportedDetachPolicy)
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
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            if let Some(attachment) = state.attachment.as_ref() {
                if attachment.generation == generation && attachment.handle.is_some() {
                    return Ok(());
                }
                return Err(BackingSurfaceError::CurrentGenerationAttached(
                    attachment.generation,
                ));
            }
            compare_generation_allow_next(state.latest_generation, generation)?;
            if state.retired_generation == Some(generation) {
                return Err(BackingSurfaceError::GenerationRetired(generation));
            }
            state.latest_generation = Some(generation);
            state.invalidated_generation = None;
            state.attachment = Some(Attachment {
                generation,
                handle: None,
                snapshot: None,
                host_focused: false,
                detaching: false,
                last_event_sequence: 0,
            });
        }

        self.emit(BackingSurfaceAdapterEvent::ListenerInstalled { generation });
        let callback_adapter = self.clone();
        let callback = Arc::new(move |event| {
            if let Err(error) = callback_adapter.admit_runtime_event(event) {
                longhorn_core::report_best_effort_failure(
                    "native-content.backing-surface.runtime-event",
                    format_args!("{error:?}"),
                );
            }
        });
        self.emit(BackingSurfaceAdapterEvent::AttachStarted { generation });
        let (handle, snapshot) = match self.runtime.attach(RuntimeAttachRequest {
            generation,
            spec: self.spec.clone(),
            callback,
        }) {
            Ok(result) => result,
            Err(error) => {
                self.clear_reservation(generation)?;
                return Err(error);
            }
        };
        if let Err(error) = validate_snapshot(&snapshot) {
            if let Err(error) = self.runtime.detach(&handle) {
                longhorn_core::report_best_effort_failure(
                    "native-content.backing-surface.detach",
                    format_args!("{error:?}"),
                );
            }
            self.clear_reservation(generation)?;
            return Err(error);
        }
        if !snapshot.native_storage_attached {
            if let Err(error) = self.runtime.detach(&handle) {
                longhorn_core::report_best_effort_failure(
                    "native-content.backing-surface.detach",
                    format_args!("{error:?}"),
                );
            }
            self.clear_reservation(generation)?;
            return Err(BackingSurfaceError::Runtime {
                operation: "attach",
                detail: "runtime returned detached native storage".to_string(),
            });
        }

        let retained = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            let current = state.invalidated_generation != Some(generation);
            match state.attachment.as_mut() {
                Some(attachment) if attachment.generation == generation && current => {
                    attachment.handle = Some(handle.clone());
                    attachment.snapshot = Some(snapshot);
                    true
                }
                _ => false,
            }
        };
        if !retained {
            if let Err(error) = self.runtime.detach(&handle) {
                longhorn_core::report_best_effort_failure(
                    "native-content.backing-surface.detach",
                    format_args!("{error:?}"),
                );
            }
            return Err(BackingSurfaceError::GenerationInvalidated(generation));
        }
        self.emit(BackingSurfaceAdapterEvent::Attached { generation });
        Ok(())
    }

    fn detach_generation(
        &self,
        generation: AttachGeneration,
    ) -> Result<BackingSurfaceDetachOutcome, BackingSurfaceError> {
        let handle = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            let Some(attachment) = state.attachment.as_mut() else {
                if state.retired_generation == Some(generation) {
                    return Ok(BackingSurfaceDetachOutcome::AlreadyDetached);
                }
                return Err(BackingSurfaceError::NotAttached);
            };
            if attachment.generation != generation {
                return Err(compare_attached_generation(
                    attachment.generation,
                    generation,
                ));
            }
            let handle = attachment
                .handle
                .clone()
                .ok_or(BackingSurfaceError::AttachInProgress)?;
            attachment.detaching = true;
            handle
        };
        self.emit(BackingSurfaceAdapterEvent::DetachStarted { generation });
        if let Err(error) = self.runtime.detach(&handle) {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            if state.invalidated_generation != Some(generation) {
                if let Some(attachment) = state.attachment.as_mut() {
                    if attachment.generation == generation {
                        attachment.detaching = false;
                    }
                }
            }
            return Err(error);
        }
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            if state
                .attachment
                .as_ref()
                .is_some_and(|attachment| attachment.generation == generation)
            {
                state.attachment = None;
            }
            state.retired_generation = Some(generation);
        }
        self.emit(BackingSurfaceAdapterEvent::Detached { generation });
        Ok(BackingSurfaceDetachOutcome::Detached)
    }

    fn optional_handle(
        &self,
        generation: AttachGeneration,
    ) -> Result<Option<R::Handle>, BackingSurfaceError> {
        let state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        if state.invalidated_generation == Some(generation) {
            return Err(BackingSurfaceError::GenerationInvalidated(generation));
        }
        if state.retired_generation == Some(generation) {
            return Err(BackingSurfaceError::GenerationRetired(generation));
        }
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

    fn handle(&self, generation: AttachGeneration) -> Result<R::Handle, BackingSurfaceError> {
        self.optional_handle(generation)?
            .ok_or(BackingSurfaceError::AttachInProgress)
    }

    fn replace_snapshot(
        &self,
        generation: AttachGeneration,
        snapshot: BackingSurfaceSnapshot,
    ) -> Result<(), BackingSurfaceError> {
        validate_snapshot(&snapshot)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        let attachment = current_attachment_mut(&mut state, generation)?;
        if let Some(current) = attachment.snapshot.as_ref() {
            if snapshot.frame_sequence < current.frame_sequence {
                return Err(BackingSurfaceError::StaleFrameSequence {
                    current: current.frame_sequence,
                    supplied: snapshot.frame_sequence,
                });
            }
        }
        attachment.snapshot = Some(snapshot);
        Ok(())
    }

    fn clear_reservation(&self, generation: AttachGeneration) -> Result<(), BackingSurfaceError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        if state.attachment.as_ref().is_some_and(|attachment| {
            attachment.generation == generation && attachment.handle.is_none()
        }) {
            state.attachment = None;
        }
        Ok(())
    }

    fn emit(&self, event: BackingSurfaceAdapterEvent) {
        (self.observer)(event);
    }
}

fn validate_snapshot(snapshot: &BackingSurfaceSnapshot) -> Result<(), BackingSurfaceError> {
    if matches!(
        snapshot.input_routing,
        InputRoutingMode::RendererForwarded | InputRoutingMode::Disabled
    ) {
        Ok(())
    } else {
        Err(BackingSurfaceError::UnsupportedInputMode)
    }
}

fn observation(
    generation: AttachGeneration,
    snapshot: &BackingSurfaceSnapshot,
    detaching: bool,
) -> ObservationUpdate {
    ObservationUpdate::new(
        generation,
        if !snapshot.native_storage_attached {
            AttachmentLifecycle::Failed
        } else if detaching {
            AttachmentLifecycle::Detaching
        } else {
            AttachmentLifecycle::Attached
        },
        if snapshot.native_storage_attached {
            ObservedReadiness::Ready
        } else {
            ObservedReadiness::NotReady
        },
        EffectiveVisibility::Unknown,
        EffectiveFocus::Unknown,
        ObservedGeometry::BackingSurface {
            storage_bounds: snapshot.storage_bounds,
            clip: snapshot.clip,
        },
        Some(snapshot.input_routing),
    )
}

fn current_attachment<H>(
    state: &AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&Attachment<H>, BackingSurfaceError> {
    compare_generation(state.latest_generation, generation)?;
    if state.invalidated_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationInvalidated(generation));
    }
    if state.retired_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationRetired(generation));
    }
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
    Ok(attachment)
}

fn current_attachment_mut<H>(
    state: &mut AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&mut Attachment<H>, BackingSurfaceError> {
    compare_generation(state.latest_generation, generation)?;
    if state.invalidated_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationInvalidated(generation));
    }
    if state.retired_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationRetired(generation));
    }
    let attachment = state
        .attachment
        .as_mut()
        .ok_or(BackingSurfaceError::NotAttached)?;
    if attachment.generation != generation {
        return Err(compare_attached_generation(
            attachment.generation,
            generation,
        ));
    }
    Ok(attachment)
}

fn compare_generation(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), BackingSurfaceError> {
    let Some(current) = current else {
        return Ok(());
    };
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
    if supplied < current {
        return Err(BackingSurfaceError::StaleGeneration { current, supplied });
    }
    if supplied == current || current.checked_next().ok() == Some(supplied) {
        Ok(())
    } else {
        Err(BackingSurfaceError::FutureGeneration { current, supplied })
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
