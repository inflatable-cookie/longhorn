use std::sync::{Arc, Mutex};

use longhorn_core::WindowId;
use longhorn_native_content_prototype::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, DetachPolicy, EffectiveFocus,
    EffectiveVisibility, InputRoutingMode, NativeContentFailureCode, NativeContentMechanism,
    NativeContentOperation, ObservationUpdate, ObservedGeometry, ObservedReadiness, StepExecution,
};
use serde::Serialize;

use crate::{
    AdapterEvent, ChildWebviewError, ChildWebviewRuntime, ChildWebviewSpec, RuntimeAttachRequest,
    RuntimeEvent, RuntimeEventKind,
};

/// Attachment identity removed when its native host is destroyed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InvalidatedAttachment {
    island_id: longhorn_native_content_prototype::NativeContentIslandId,
    generation: AttachGeneration,
}

impl InvalidatedAttachment {
    /// Returns invalidated island identity.
    #[must_use]
    pub const fn island_id(&self) -> &longhorn_native_content_prototype::NativeContentIslandId {
        &self.island_id
    }

    /// Returns invalidated attach generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }
}

struct Attachment<H> {
    generation: AttachGeneration,
    handle: Option<H>,
    ready: bool,
}

struct AdapterState<H> {
    latest_generation: Option<AttachGeneration>,
    attachment: Option<Attachment<H>>,
}

impl<H> Default for AdapterState<H> {
    fn default() -> Self {
        Self {
            latest_generation: None,
            attachment: None,
        }
    }
}

/// Generation-checked child-only executor over one selected runtime port.
pub struct ChildWebviewAdapter<R: ChildWebviewRuntime> {
    runtime: R,
    spec: ChildWebviewSpec,
    state: Arc<Mutex<AdapterState<R::Handle>>>,
    observer: Arc<dyn Fn(AdapterEvent) + Send + Sync>,
}

impl<R: ChildWebviewRuntime> Clone for ChildWebviewAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: self.state.clone(),
            observer: self.observer.clone(),
        }
    }
}

impl<R: ChildWebviewRuntime> ChildWebviewAdapter<R> {
    /// Creates one adapter from explicit construction/security policy and an injected observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: ChildWebviewSpec,
        observer: Arc<dyn Fn(AdapterEvent) + Send + Sync>,
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
    pub const fn spec(&self) -> &ChildWebviewSpec {
        &self.spec
    }

    /// Executes an immutable child-view plan and returns exact partial-work evidence.
    pub fn apply(&self, plan: &ApplyPlan) -> Result<ApplyReceipt, ChildWebviewError> {
        self.validate_plan(plan)?;
        let mut executions = Vec::new();

        for planned in plan.operations() {
            let result = self.execute(plan.generation(), planned.operation());
            match result {
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
            .map_err(|error| ChildWebviewError::InvalidReceipt(error.to_string()))
    }

    /// Reads fresh native bounds while preserving unobservable visibility and focus as unknown.
    pub fn observe(
        &self,
        generation: AttachGeneration,
    ) -> Result<ObservationUpdate, ChildWebviewError> {
        let (handle, ready) = {
            let state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            match state.attachment.as_ref() {
                None => {
                    return Ok(ObservationUpdate::new(
                        generation,
                        AttachmentLifecycle::Absent,
                        ObservedReadiness::Unknown,
                        EffectiveVisibility::Unknown,
                        EffectiveFocus::Unknown,
                        ObservedGeometry::Unknown,
                        None,
                    ));
                }
                Some(attachment) if attachment.generation != generation => {
                    return Err(compare_attached_generation(
                        attachment.generation,
                        generation,
                    ));
                }
                Some(attachment) => (attachment.handle.clone(), attachment.ready),
            }
        };

        let Some(handle) = handle else {
            return Ok(ObservationUpdate::new(
                generation,
                AttachmentLifecycle::Attaching,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::Unknown,
                Some(InputRoutingMode::NativeDirect),
            ));
        };
        let bounds = self.runtime.bounds(&handle)?;
        Ok(ObservationUpdate::new(
            generation,
            AttachmentLifecycle::Attached,
            if ready {
                ObservedReadiness::Ready
            } else {
                ObservedReadiness::NotReady
            },
            EffectiveVisibility::Unknown,
            EffectiveFocus::Unknown,
            ObservedGeometry::ChildBounds { bounds },
            Some(InputRoutingMode::NativeDirect),
        ))
    }

    /// Records renderer unmount without closing or forgetting native content.
    pub fn renderer_unmounted(
        &self,
        generation: AttachGeneration,
    ) -> Result<(), ChildWebviewError> {
        self.handle(generation)?;
        self.emit(AdapterEvent::RendererUnmounted { generation });
        Ok(())
    }

    /// Evaluates a controlled test probe inside the current child generation.
    pub fn evaluate(
        &self,
        generation: AttachGeneration,
        script: &str,
    ) -> Result<(), ChildWebviewError> {
        let handle = self.handle(generation)?;
        self.runtime.evaluate(&handle, script)
    }

    /// Invalidates attachment authority when the mapped host window is destroyed.
    pub fn host_destroyed(
        &self,
        host_window_id: &WindowId,
    ) -> Result<Option<InvalidatedAttachment>, ChildWebviewError> {
        if host_window_id != self.spec.host_window_id() {
            return Ok(None);
        }
        let invalidated = {
            let mut state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
            state
                .attachment
                .take()
                .map(|attachment| InvalidatedAttachment {
                    island_id: self.spec.island_id().clone(),
                    generation: attachment.generation,
                })
        };
        if let Some(invalidated) = invalidated.as_ref() {
            self.emit(AdapterEvent::HostInvalidated {
                generation: invalidated.generation,
            });
        }
        Ok(invalidated)
    }

    /// Admits one callback only while its exact generation and transport label remain current.
    pub fn admit_runtime_event(&self, event: RuntimeEvent) -> Result<(), ChildWebviewError> {
        if event.island_id != *self.spec.island_id()
            || event.webview_label != self.spec.webview_label().as_str()
        {
            return Err(ChildWebviewError::NotAttached);
        }
        {
            let mut state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
            compare_generation(state.latest_generation, event.generation)?;
            let attachment = state
                .attachment
                .as_mut()
                .ok_or(ChildWebviewError::NotAttached)?;
            if attachment.generation != event.generation {
                return Err(compare_attached_generation(
                    attachment.generation,
                    event.generation,
                ));
            }
            if matches!(event.kind, RuntimeEventKind::PageLoadFinished { .. }) {
                attachment.ready = true;
            }
        }
        self.emit(AdapterEvent::Runtime {
            generation: event.generation,
            event: event.kind,
        });
        Ok(())
    }

    /// Returns whether a usable native handle is current for the supplied generation.
    pub fn is_attached(&self, generation: AttachGeneration) -> Result<bool, ChildWebviewError> {
        let state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        Ok(state.attachment.as_ref().is_some_and(|attachment| {
            attachment.generation == generation && attachment.handle.is_some()
        }))
    }

    fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), ChildWebviewError> {
        if plan.island_id() != self.spec.island_id() {
            return Err(ChildWebviewError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: plan.island_id().clone(),
            });
        }
        if plan.operations().iter().any(|operation| {
            matches!(
                operation.operation(),
                NativeContentOperation::Attach { mechanism, .. }
                    if *mechanism != NativeContentMechanism::ChildView
            ) || matches!(
                operation.operation(),
                NativeContentOperation::SetIsolatedContentSize { .. }
                    | NativeContentOperation::SetBackingViewport { .. }
            )
        }) {
            return Err(ChildWebviewError::WrongMechanism);
        }

        let state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
        if let Some(attachment) = state.attachment.as_ref() {
            if plan.generation() < attachment.generation {
                return Err(ChildWebviewError::StaleGeneration {
                    current: attachment.generation,
                    supplied: plan.generation(),
                });
            }
            if plan.generation() > attachment.generation {
                return Err(ChildWebviewError::CurrentGenerationAttached(
                    attachment.generation,
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
    ) -> Result<(), ChildWebviewError> {
        match operation {
            NativeContentOperation::Attach {
                host_window_id,
                mechanism: NativeContentMechanism::ChildView,
            } => {
                if host_window_id != self.spec.host_window_id() {
                    return Err(ChildWebviewError::HostBindingMismatch);
                }
                self.attach(generation)
            }
            NativeContentOperation::Attach { .. }
            | NativeContentOperation::SetIsolatedContentSize { .. }
            | NativeContentOperation::SetBackingViewport { .. } => {
                Err(ChildWebviewError::WrongMechanism)
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
                self.handle(generation)?;
                Ok(())
            }
            NativeContentOperation::SetInputRouting { .. } => {
                Err(ChildWebviewError::UnsupportedInputMode)
            }
            NativeContentOperation::RequestFocus => {
                let handle = self.handle(generation)?;
                self.runtime.focus(&handle)
            }
            NativeContentOperation::ReleaseFocusIfOwned => {
                Err(ChildWebviewError::UnsupportedFocusRelease)
            }
            NativeContentOperation::Detach {
                policy: DetachPolicy::Reversible,
            } => self.detach(generation),
            NativeContentOperation::Detach { .. } => {
                Err(ChildWebviewError::UnsupportedDetachPolicy)
            }
        }
    }

    fn attach(&self, generation: AttachGeneration) -> Result<(), ChildWebviewError> {
        {
            let mut state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
            if let Some(attachment) = state.attachment.as_ref() {
                if attachment.generation == generation && attachment.handle.is_some() {
                    return Ok(());
                }
                return Err(ChildWebviewError::CurrentGenerationAttached(
                    attachment.generation,
                ));
            }
            compare_generation_allow_next(state.latest_generation, generation)?;
            state.latest_generation = Some(generation);
            state.attachment = Some(Attachment {
                generation,
                handle: None,
                ready: false,
            });
        }

        self.emit(AdapterEvent::ListenerInstalled { generation });
        let callback_adapter = self.clone();
        let callback = Arc::new(move |event| {
            let _ = callback_adapter.admit_runtime_event(event);
        });
        self.emit(AdapterEvent::AttachStarted { generation });
        let request = RuntimeAttachRequest {
            generation,
            spec: self.spec.clone(),
            callback,
        };
        let handle = match self.runtime.attach(request) {
            Ok(handle) => handle,
            Err(error) => {
                self.clear_reservation(generation)?;
                return Err(error);
            }
        };

        let retained = {
            let mut state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
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
            return Err(ChildWebviewError::NotAttached);
        }
        self.emit(AdapterEvent::Attached { generation });
        Ok(())
    }

    fn detach(&self, generation: AttachGeneration) -> Result<(), ChildWebviewError> {
        let handle = {
            let state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            match state.attachment.as_ref() {
                None => return Ok(()),
                Some(attachment) if attachment.generation != generation => {
                    return Err(compare_attached_generation(
                        attachment.generation,
                        generation,
                    ));
                }
                Some(attachment) => attachment
                    .handle
                    .clone()
                    .ok_or(ChildWebviewError::AttachInProgress)?,
            }
        };
        self.emit(AdapterEvent::DetachStarted { generation });
        self.runtime.close(&handle)?;
        {
            let mut state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
            if state
                .attachment
                .as_ref()
                .is_some_and(|attachment| attachment.generation == generation)
            {
                state.attachment = None;
            }
        }
        self.emit(AdapterEvent::Detached { generation });
        Ok(())
    }

    fn handle(&self, generation: AttachGeneration) -> Result<R::Handle, ChildWebviewError> {
        let state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        let attachment = state
            .attachment
            .as_ref()
            .ok_or(ChildWebviewError::NotAttached)?;
        if attachment.generation != generation {
            return Err(compare_attached_generation(
                attachment.generation,
                generation,
            ));
        }
        attachment
            .handle
            .clone()
            .ok_or(ChildWebviewError::AttachInProgress)
    }

    fn clear_reservation(&self, generation: AttachGeneration) -> Result<(), ChildWebviewError> {
        let mut state = self.state.lock().map_err(|_| ChildWebviewError::Poisoned)?;
        if state.attachment.as_ref().is_some_and(|attachment| {
            attachment.generation == generation && attachment.handle.is_none()
        }) {
            state.attachment = None;
        }
        Ok(())
    }

    fn emit(&self, event: AdapterEvent) {
        (self.observer)(event);
    }
}

fn compare_generation(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), ChildWebviewError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        Err(ChildWebviewError::StaleGeneration { current, supplied })
    } else if supplied > current {
        Err(ChildWebviewError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

fn compare_generation_allow_next(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), ChildWebviewError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        return Err(ChildWebviewError::StaleGeneration { current, supplied });
    }
    let next = current.checked_next().ok();
    if supplied == current || Some(supplied) == next {
        Ok(())
    } else {
        Err(ChildWebviewError::FutureGeneration { current, supplied })
    }
}

fn compare_attached_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> ChildWebviewError {
    if supplied < current {
        ChildWebviewError::StaleGeneration { current, supplied }
    } else {
        ChildWebviewError::FutureGeneration { current, supplied }
    }
}
