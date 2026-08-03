use std::sync::{Arc, Mutex};

use longhorn_core::{NativeContentFailureCode, WindowId};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, DetachPolicy, EffectiveFocus,
    EffectiveVisibility, InputRoutingMode, NativeContentCoordinator, NativeContentIslandId,
    NativeContentMechanism, NativeContentOperation, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, StepExecution,
};
use serde::Serialize;
use tauri::Url;

use crate::{
    ChildViewAdapterEvent, ChildViewError, ChildViewRuntime, ChildViewRuntimeEvent,
    ChildViewRuntimeEventKind, ChildViewSpec, RuntimeAttachRequest,
};

/// Result of applying one exact host-destruction notification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildViewHostDestroyOutcome {
    /// A live or attaching child was invalidated.
    Invalidated,
    /// The same generation was already invalidated.
    AlreadyInvalidated,
}

/// Adapter-local host-destruction evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChildViewHostDestroyReceipt {
    island_id: NativeContentIslandId,
    generation: AttachGeneration,
    outcome: ChildViewHostDestroyOutcome,
}

impl ChildViewHostDestroyReceipt {
    /// Returns the invalidated island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the exact invalidated generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns whether this call performed or confirmed invalidation.
    #[must_use]
    pub const fn outcome(&self) -> ChildViewHostDestroyOutcome {
        self.outcome
    }
}

/// Result of one adapter shutdown attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildViewTeardownOutcome {
    /// The retained child was closed.
    Closed,
    /// No child remained to close.
    AlreadyDetached,
}

/// Result of one policy-admitted document request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildViewNavigationOutcome {
    /// The requested URL was already current; no navigation was submitted.
    Unchanged,
    /// The native runtime accepted one navigation request.
    Submitted,
}

/// Adapter-local evidence for one generation-bound document request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChildViewNavigationReceipt {
    island_id: NativeContentIslandId,
    generation: AttachGeneration,
    previous_url: Url,
    requested_url: Url,
    outcome: ChildViewNavigationOutcome,
}

impl ChildViewNavigationReceipt {
    /// Returns the retained island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the exact retained attach generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns the fresh URL observed before the request.
    #[must_use]
    pub const fn previous_url(&self) -> &Url {
        &self.previous_url
    }

    /// Returns the consumer-requested URL.
    #[must_use]
    pub const fn requested_url(&self) -> &Url {
        &self.requested_url
    }

    /// Returns whether native navigation was unnecessary or submitted.
    #[must_use]
    pub const fn outcome(&self) -> ChildViewNavigationOutcome {
        self.outcome
    }
}

/// Adapter-local bounded teardown evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChildViewTeardownReceipt {
    island_id: NativeContentIslandId,
    generation: Option<AttachGeneration>,
    outcome: ChildViewTeardownOutcome,
}

impl ChildViewTeardownReceipt {
    /// Returns the adapter island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the closed generation, when one existed.
    #[must_use]
    pub const fn generation(&self) -> Option<AttachGeneration> {
        self.generation
    }

    /// Returns the exact teardown result.
    #[must_use]
    pub const fn outcome(&self) -> ChildViewTeardownOutcome {
        self.outcome
    }
}

struct Attachment<H> {
    generation: AttachGeneration,
    handle: Option<H>,
    ready: bool,
    detaching: bool,
    input_routing: InputRoutingMode,
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

/// Generation-checked child-only executor over one selected runtime port.
pub struct ChildViewAdapter<R: ChildViewRuntime> {
    runtime: R,
    spec: ChildViewSpec,
    state: Arc<Mutex<AdapterState<R::Handle>>>,
    observer: Arc<dyn Fn(ChildViewAdapterEvent) + Send + Sync>,
}

impl<R: ChildViewRuntime> Clone for ChildViewAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: self.state.clone(),
            observer: self.observer.clone(),
        }
    }
}

impl<R: ChildViewRuntime> ChildViewAdapter<R> {
    /// Creates one adapter from injected construction/security policy and an observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: ChildViewSpec,
        observer: Arc<dyn Fn(ChildViewAdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable construction and security policy.
    #[must_use]
    pub const fn spec(&self) -> &ChildViewSpec {
        &self.spec
    }

    /// Executes a current immutable plan and returns coordinator-validated evidence.
    pub fn apply(
        &self,
        authority: &NativeContentCoordinator,
        plan: &ApplyPlan,
    ) -> Result<ApplyReceipt, ChildViewError> {
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

    /// Reads fresh native bounds while preserving unobservable states as unknown.
    pub fn observe(
        &self,
        generation: AttachGeneration,
    ) -> Result<ObservationUpdate, ChildViewError> {
        let snapshot = {
            let state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            match state.attachment.as_ref() {
                None => None,
                Some(attachment) if attachment.generation != generation => {
                    return Err(compare_attached_generation(
                        attachment.generation,
                        generation,
                    ));
                }
                Some(attachment) => Some((
                    attachment.handle.clone(),
                    attachment.ready,
                    attachment.detaching,
                    attachment.input_routing,
                )),
            }
        };

        let Some((handle, ready, detaching, input_routing)) = snapshot else {
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
                Some(input_routing),
            ));
        };
        let bounds = self.runtime.bounds(&handle)?;
        Ok(ObservationUpdate::new(
            generation,
            if detaching {
                AttachmentLifecycle::Detaching
            } else {
                AttachmentLifecycle::Attached
            },
            if ready {
                ObservedReadiness::Ready
            } else {
                ObservedReadiness::NotReady
            },
            EffectiveVisibility::Unknown,
            EffectiveFocus::Unknown,
            ObservedGeometry::ChildBounds { bounds },
            Some(input_routing),
        ))
    }

    /// Records renderer unmount without closing or forgetting native content.
    pub fn renderer_unmounted(&self, generation: AttachGeneration) -> Result<(), ChildViewError> {
        self.handle(generation)?;
        self.emit(ChildViewAdapterEvent::RendererUnmounted { generation });
        Ok(())
    }

    /// Reads the fresh current document for one exact retained generation.
    pub fn current_url(&self, generation: AttachGeneration) -> Result<Url, ChildViewError> {
        let handle = self.handle(generation)?;
        self.runtime.current_url(&handle)
    }

    /// Applies consumer policy and submits at most one native navigation.
    pub fn navigate(
        &self,
        generation: AttachGeneration,
        requested_url: Url,
    ) -> Result<ChildViewNavigationReceipt, ChildViewError> {
        let handle = self.handle(generation)?;
        if !self.spec.allows_navigation(&requested_url) {
            return Err(ChildViewError::NavigationDenied(requested_url));
        }
        let previous_url = self.runtime.current_url(&handle)?;
        let outcome = if previous_url == requested_url {
            ChildViewNavigationOutcome::Unchanged
        } else {
            self.runtime.navigate(&handle, requested_url.clone())?;
            ChildViewNavigationOutcome::Submitted
        };
        Ok(ChildViewNavigationReceipt {
            island_id: self.spec.island_id().clone(),
            generation,
            previous_url,
            requested_url,
            outcome,
        })
    }

    /// Invalidates exact attachment authority after its mapped host is destroyed.
    pub fn host_destroyed(
        &self,
        host_window_id: &WindowId,
        generation: AttachGeneration,
    ) -> Result<ChildViewHostDestroyReceipt, ChildViewError> {
        if host_window_id != self.spec.host_window_id() {
            return Err(ChildViewError::HostBindingMismatch);
        }
        let outcome = {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            if state.invalidated_generation == Some(generation) {
                ChildViewHostDestroyOutcome::AlreadyInvalidated
            } else {
                let attachment = state.attachment.as_ref().ok_or_else(|| {
                    if state.retired_generation == Some(generation) {
                        ChildViewError::GenerationRetired(generation)
                    } else {
                        ChildViewError::NotAttached
                    }
                })?;
                if attachment.generation != generation {
                    return Err(compare_attached_generation(
                        attachment.generation,
                        generation,
                    ));
                }
                state.attachment = None;
                state.retired_generation = Some(generation);
                state.invalidated_generation = Some(generation);
                ChildViewHostDestroyOutcome::Invalidated
            }
        };
        if outcome == ChildViewHostDestroyOutcome::Invalidated {
            self.emit(ChildViewAdapterEvent::HostInvalidated { generation });
        }
        Ok(ChildViewHostDestroyReceipt {
            island_id: self.spec.island_id().clone(),
            generation,
            outcome,
        })
    }

    /// Closes any retained child once; a failed close preserves it for retry.
    pub fn teardown(&self) -> Result<ChildViewTeardownReceipt, ChildViewError> {
        let generation = {
            let state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            state
                .attachment
                .as_ref()
                .map(|attachment| attachment.generation)
        };
        let Some(generation) = generation else {
            return Ok(ChildViewTeardownReceipt {
                island_id: self.spec.island_id().clone(),
                generation: None,
                outcome: ChildViewTeardownOutcome::AlreadyDetached,
            });
        };
        self.detach(generation)?;
        Ok(ChildViewTeardownReceipt {
            island_id: self.spec.island_id().clone(),
            generation: Some(generation),
            outcome: ChildViewTeardownOutcome::Closed,
        })
    }

    /// Admits callbacks only while exact generation and transport identity remain current.
    pub fn admit_runtime_event(&self, event: ChildViewRuntimeEvent) -> Result<(), ChildViewError> {
        if event.island_id != *self.spec.island_id()
            || event.child_label != *self.spec.child_label()
        {
            return Err(ChildViewError::NotAttached);
        }
        {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            compare_generation(state.latest_generation, event.generation)?;
            if state.retired_generation == Some(event.generation) {
                return Err(ChildViewError::GenerationRetired(event.generation));
            }
            let attachment = state
                .attachment
                .as_mut()
                .ok_or(ChildViewError::NotAttached)?;
            if attachment.generation != event.generation {
                return Err(compare_attached_generation(
                    attachment.generation,
                    event.generation,
                ));
            }
            match event.kind {
                ChildViewRuntimeEventKind::PageLoadStarted => attachment.ready = false,
                ChildViewRuntimeEventKind::PageLoadFinished => attachment.ready = true,
            }
        }
        self.emit(ChildViewAdapterEvent::Runtime {
            generation: event.generation,
            event: event.kind,
        });
        Ok(())
    }

    /// Returns whether a usable native handle is current for the supplied generation.
    pub fn is_attached(&self, generation: AttachGeneration) -> Result<bool, ChildViewError> {
        let state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        Ok(state.attachment.as_ref().is_some_and(|attachment| {
            attachment.generation == generation && attachment.handle.is_some()
        }))
    }

    fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), ChildViewError> {
        if plan.island_id() != self.spec.island_id() {
            return Err(ChildViewError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: plan.island_id().clone(),
            });
        }
        if plan.operations().iter().any(|planned| {
            matches!(
                planned.operation(),
                NativeContentOperation::Attach { mechanism, .. }
                    if *mechanism != NativeContentMechanism::ChildView
            ) || matches!(
                planned.operation(),
                NativeContentOperation::SetIsolatedContentSize { .. }
                    | NativeContentOperation::SetBackingViewport { .. }
            )
        }) {
            return Err(ChildViewError::WrongMechanism);
        }

        let state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
        if let Some(attachment) = state.attachment.as_ref() {
            if plan.generation() < attachment.generation {
                return Err(ChildViewError::StaleGeneration {
                    current: attachment.generation,
                    supplied: plan.generation(),
                });
            }
            if plan.generation() > attachment.generation {
                return Err(ChildViewError::CurrentGenerationAttached(
                    attachment.generation,
                ));
            }
        } else {
            compare_generation_allow_next(state.latest_generation, plan.generation())?;
            if state.retired_generation == Some(plan.generation())
                && plan.operations().iter().any(|planned| {
                    matches!(planned.operation(), NativeContentOperation::Attach { .. })
                })
            {
                return Err(ChildViewError::GenerationRetired(plan.generation()));
            }
        }
        Ok(())
    }

    fn execute(
        &self,
        generation: AttachGeneration,
        operation: &NativeContentOperation,
    ) -> Result<(), ChildViewError> {
        match operation {
            NativeContentOperation::Attach {
                host_window_id,
                mechanism: NativeContentMechanism::ChildView,
            } => {
                if host_window_id != self.spec.host_window_id() {
                    return Err(ChildViewError::HostBindingMismatch);
                }
                self.attach(generation)
            }
            NativeContentOperation::Attach { .. }
            | NativeContentOperation::SetIsolatedContentSize { .. }
            | NativeContentOperation::SetBackingViewport { .. } => {
                Err(ChildViewError::WrongMechanism)
            }
            NativeContentOperation::SetChildBounds { bounds } => {
                let handle = self.handle(generation)?;
                self.runtime.set_bounds(&handle, *bounds)
            }
            NativeContentOperation::Show => {
                let handle = self.handle(generation)?;
                self.runtime.show(&handle)
            }
            NativeContentOperation::Hide { .. } => {
                let handle = self.handle(generation)?;
                self.runtime.hide(&handle)
            }
            NativeContentOperation::SetInputRouting {
                mode: InputRoutingMode::NativeDirect,
            } => {
                let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
                let attachment = current_attachment_mut(&mut state, generation)?;
                if attachment.handle.is_none() {
                    return Err(ChildViewError::AttachInProgress);
                }
                attachment.input_routing = InputRoutingMode::NativeDirect;
                Ok(())
            }
            NativeContentOperation::SetInputRouting { .. } => {
                Err(ChildViewError::UnsupportedInputMode)
            }
            NativeContentOperation::RequestFocus => {
                let handle = self.handle(generation)?;
                self.runtime.focus(&handle)
            }
            NativeContentOperation::ReleaseFocusIfOwned => {
                Err(ChildViewError::UnsupportedFocusRelease)
            }
            NativeContentOperation::Detach {
                policy: DetachPolicy::Reversible,
            } => self.detach(generation),
            NativeContentOperation::Detach { .. } => Err(ChildViewError::UnsupportedDetachPolicy),
        }
    }

    fn attach(&self, generation: AttachGeneration) -> Result<(), ChildViewError> {
        {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            if let Some(attachment) = state.attachment.as_ref() {
                if attachment.generation == generation && attachment.handle.is_some() {
                    return Ok(());
                }
                return Err(ChildViewError::CurrentGenerationAttached(
                    attachment.generation,
                ));
            }
            compare_generation_allow_next(state.latest_generation, generation)?;
            if state.retired_generation == Some(generation) {
                return Err(ChildViewError::GenerationRetired(generation));
            }
            state.latest_generation = Some(generation);
            state.invalidated_generation = None;
            state.attachment = Some(Attachment {
                generation,
                handle: None,
                ready: false,
                detaching: false,
                input_routing: InputRoutingMode::NativeDirect,
            });
        }

        self.emit(ChildViewAdapterEvent::ListenerInstalled { generation });
        let callback_adapter = self.clone();
        let callback = Arc::new(move |event| {
            let _ = callback_adapter.admit_runtime_event(event);
        });
        self.emit(ChildViewAdapterEvent::AttachStarted { generation });
        let handle = match self.runtime.attach(RuntimeAttachRequest {
            generation,
            spec: self.spec.clone(),
            callback,
        }) {
            Ok(handle) => handle,
            Err(error) => {
                self.clear_reservation(generation)?;
                return Err(error);
            }
        };

        let retained = {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            match state.attachment.as_mut() {
                Some(attachment) if attachment.generation == generation => {
                    attachment.handle = Some(handle.clone());
                    true
                }
                _ => false,
            }
        };
        if !retained {
            let _ = self.runtime.close(&handle);
            return Err(ChildViewError::NotAttached);
        }
        self.emit(ChildViewAdapterEvent::Attached { generation });
        Ok(())
    }

    fn detach(&self, generation: AttachGeneration) -> Result<(), ChildViewError> {
        let handle = {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            let Some(attachment) = state.attachment.as_mut() else {
                if state.retired_generation == Some(generation) {
                    return Ok(());
                }
                return Err(ChildViewError::NotAttached);
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
                .ok_or(ChildViewError::AttachInProgress)?;
            attachment.detaching = true;
            handle
        };
        self.emit(ChildViewAdapterEvent::DetachStarted { generation });
        if let Err(error) = self.runtime.close(&handle) {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            if let Some(attachment) = state.attachment.as_mut() {
                if attachment.generation == generation {
                    attachment.detaching = false;
                }
            }
            return Err(error);
        }
        {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            if state
                .attachment
                .as_ref()
                .is_some_and(|attachment| attachment.generation == generation)
            {
                state.attachment = None;
            }
            state.retired_generation = Some(generation);
        }
        self.emit(ChildViewAdapterEvent::Detached { generation });
        Ok(())
    }

    fn handle(&self, generation: AttachGeneration) -> Result<R::Handle, ChildViewError> {
        let state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        if state.retired_generation == Some(generation) {
            return Err(ChildViewError::GenerationRetired(generation));
        }
        let attachment = state
            .attachment
            .as_ref()
            .ok_or(ChildViewError::NotAttached)?;
        if attachment.generation != generation {
            return Err(compare_attached_generation(
                attachment.generation,
                generation,
            ));
        }
        attachment
            .handle
            .clone()
            .ok_or(ChildViewError::AttachInProgress)
    }

    fn clear_reservation(&self, generation: AttachGeneration) -> Result<(), ChildViewError> {
        let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
        if state.attachment.as_ref().is_some_and(|attachment| {
            attachment.generation == generation && attachment.handle.is_none()
        }) {
            state.attachment = None;
        }
        Ok(())
    }

    fn emit(&self, event: ChildViewAdapterEvent) {
        (self.observer)(event);
    }
}

fn current_attachment_mut<H>(
    state: &mut AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&mut Attachment<H>, ChildViewError> {
    compare_generation(state.latest_generation, generation)?;
    if state.retired_generation == Some(generation) {
        return Err(ChildViewError::GenerationRetired(generation));
    }
    let attachment = state
        .attachment
        .as_mut()
        .ok_or(ChildViewError::NotAttached)?;
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
) -> Result<(), ChildViewError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        Err(ChildViewError::StaleGeneration { current, supplied })
    } else if supplied > current {
        Err(ChildViewError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

fn compare_generation_allow_next(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), ChildViewError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        return Err(ChildViewError::StaleGeneration { current, supplied });
    }
    if supplied == current || current.checked_next().ok() == Some(supplied) {
        Ok(())
    } else {
        Err(ChildViewError::FutureGeneration { current, supplied })
    }
}

fn compare_attached_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> ChildViewError {
    if supplied < current {
        ChildViewError::StaleGeneration { current, supplied }
    } else {
        ChildViewError::FutureGeneration { current, supplied }
    }
}
