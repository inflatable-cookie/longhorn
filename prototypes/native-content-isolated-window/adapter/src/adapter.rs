use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use longhorn_core::{ClientSize, PhysicalSize, WindowId};
use longhorn_native_content_prototype::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, ContentSizeDecision,
    ContentSizeProposal, ContentSizeProposalReceipt, DesiredState, DetachPolicy, EffectiveFocus,
    EffectiveVisibility, InputRoutingMode, NativeContentFailureCode, NativeContentIslandId,
    NativeContentMechanism, NativeContentOperation, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, StepExecution, decide_content_size,
};

use crate::{
    AdapterEvent, ChildRequest, HelperEvent, HelperEventKind, IsolatedWindowError,
    IsolatedWindowRuntime, RuntimeAttachRequest, TeardownOutcome,
};

/// Immutable island mapping and bounded helper policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsolatedWindowSpec {
    island_id: NativeContentIslandId,
    host_window_id: WindowId,
    teardown_timeout: Duration,
}

impl IsolatedWindowSpec {
    /// Constructs one mapping without outer-window geometry or product policy.
    #[must_use]
    pub const fn new(
        island_id: NativeContentIslandId,
        host_window_id: WindowId,
        teardown_timeout: Duration,
    ) -> Self {
        Self {
            island_id,
            host_window_id,
            teardown_timeout,
        }
    }

    /// Returns shared island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns stable outer-window identity without placement authority.
    #[must_use]
    pub const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }

    /// Returns the bounded helper teardown timeout.
    #[must_use]
    pub const fn teardown_timeout(&self) -> Duration {
        self.teardown_timeout
    }
}

struct Attachment<H> {
    generation: AttachGeneration,
    handle: Option<H>,
    ready: bool,
    failed: bool,
    last_host_size: Option<PhysicalSize>,
    requests: Vec<ChildRequest>,
}

struct AdapterState<H> {
    latest_generation: Option<AttachGeneration>,
    attachment: Option<Attachment<H>>,
    teardown_reports: Vec<(AttachGeneration, TeardownOutcome)>,
}

impl<H> Default for AdapterState<H> {
    fn default() -> Self {
        Self {
            latest_generation: None,
            attachment: None,
            teardown_reports: Vec::new(),
        }
    }
}

/// Generation-checked isolated-window executor over one selected helper runtime.
pub struct IsolatedWindowAdapter<R: IsolatedWindowRuntime> {
    runtime: R,
    spec: IsolatedWindowSpec,
    state: Arc<Mutex<AdapterState<R::Handle>>>,
    observer: Arc<dyn Fn(AdapterEvent) + Send + Sync>,
}

impl<R: IsolatedWindowRuntime> Clone for IsolatedWindowAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: self.state.clone(),
            observer: self.observer.clone(),
        }
    }
}

impl<R: IsolatedWindowRuntime> IsolatedWindowAdapter<R> {
    /// Creates one adapter from explicit mapping, helper policy, and observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: IsolatedWindowSpec,
        observer: Arc<dyn Fn(AdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable island mapping and bounded helper policy.
    #[must_use]
    pub const fn spec(&self) -> &IsolatedWindowSpec {
        &self.spec
    }

    /// Executes an immutable isolated-window plan with exact partial-work evidence.
    pub fn apply(&self, plan: &ApplyPlan) -> Result<ApplyReceipt, IsolatedWindowError> {
        self.validate_plan(plan)?;
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
            .map_err(|error| IsolatedWindowError::InvalidReceipt(error.to_string()))
    }

    /// Reads fresh content size, visibility, and focus from the helper runtime.
    pub fn observe(
        &self,
        generation: AttachGeneration,
    ) -> Result<ObservationUpdate, IsolatedWindowError> {
        let (handle, ready, failed) = {
            let state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
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
                Some(attachment) => (
                    attachment.handle.clone(),
                    attachment.ready,
                    attachment.failed,
                ),
            }
        };

        if failed {
            return Ok(ObservationUpdate::new(
                generation,
                AttachmentLifecycle::Failed,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::Unknown,
                Some(InputRoutingMode::NativeDirect),
            ));
        }
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
        let snapshot = self.runtime.observe(&handle)?;
        Ok(ObservationUpdate::new(
            generation,
            AttachmentLifecycle::Attached,
            if ready {
                ObservedReadiness::Ready
            } else {
                ObservedReadiness::NotReady
            },
            if snapshot.visible {
                EffectiveVisibility::Visible
            } else {
                EffectiveVisibility::Hidden
            },
            if snapshot.focused {
                EffectiveFocus::Focused
            } else {
                EffectiveFocus::Unfocused
            },
            ObservedGeometry::IsolatedContent {
                size: snapshot.content_size,
            },
            Some(InputRoutingMode::NativeDirect),
        ))
    }

    /// Scripts one controlled fake-child request for the current generation.
    pub fn script_request(
        &self,
        generation: AttachGeneration,
        request: ChildRequest,
    ) -> Result<(), IsolatedWindowError> {
        let handle = self.handle(generation)?;
        self.runtime.script_request(&handle, request)
    }

    /// Terminates the controlled current helper without beginning teardown.
    pub fn simulate_helper_loss(
        &self,
        generation: AttachGeneration,
    ) -> Result<Option<i32>, IsolatedWindowError> {
        let handle = self.handle(generation)?;
        self.runtime.simulate_helper_loss(&handle)
    }

    /// Removes and returns current consumer-admitted fake-child requests.
    pub fn take_requests(
        &self,
        generation: AttachGeneration,
    ) -> Result<Vec<ChildRequest>, IsolatedWindowError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        let attachment = state
            .attachment
            .as_mut()
            .ok_or(IsolatedWindowError::NotAttached)?;
        if attachment.generation != generation {
            return Err(compare_attached_generation(
                attachment.generation,
                generation,
            ));
        }
        Ok(std::mem::take(&mut attachment.requests))
    }

    /// Validates a current physical resize request through explicit consumer policy.
    pub fn decide_resize(
        &self,
        desired: &DesiredState,
        physical_size: PhysicalSize,
        decision: ContentSizeDecision,
    ) -> Result<ContentSizeProposalReceipt, IsolatedWindowError> {
        let generation = desired.generation();
        self.handle(generation)?;
        let scale = f64::from(desired.scale().thousandths());
        let size = ClientSize::new(
            f64::from(physical_size.width()) * 1000.0 / scale,
            f64::from(physical_size.height()) * 1000.0 / scale,
        )
        .map_err(|error| IsolatedWindowError::Runtime {
            operation: "size",
            detail: error.to_string(),
        })?;
        decide_content_size(
            desired,
            ContentSizeProposal::new(generation, desired.revision(), size),
            decision,
        )
        .map_err(|error| IsolatedWindowError::Runtime {
            operation: "size",
            detail: error.to_string(),
        })
    }

    /// Applies an explicitly admitted resize-hint request.
    pub fn set_resizable(
        &self,
        generation: AttachGeneration,
        resizable: bool,
    ) -> Result<(), IsolatedWindowError> {
        let handle = self.handle(generation)?;
        self.runtime.set_resizable(&handle, resizable)
    }

    /// Returns all bounded teardown reports without consuming them.
    pub fn teardown_reports(
        &self,
    ) -> Result<Vec<(AttachGeneration, TeardownOutcome)>, IsolatedWindowError> {
        self.state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)
            .map(|state| state.teardown_reports.clone())
    }

    /// Admits one callback only while its exact island and generation remain current.
    pub fn admit_runtime_event(&self, event: HelperEvent) -> Result<(), IsolatedWindowError> {
        if event.island_id != *self.spec.island_id() {
            return Err(IsolatedWindowError::NotAttached);
        }
        let generation = event.generation;
        let mut emitted = Some(event.kind.clone());
        let mut suppressed = None;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            compare_generation(state.latest_generation, event.generation)?;
            let attachment = state
                .attachment
                .as_mut()
                .ok_or(IsolatedWindowError::NotAttached)?;
            if attachment.generation != event.generation {
                return Err(compare_attached_generation(
                    attachment.generation,
                    event.generation,
                ));
            }
            match &event.kind {
                HelperEventKind::Progress { .. } => {}
                HelperEventKind::Ready { .. } => attachment.ready = true,
                HelperEventKind::ChildRequest {
                    request: ChildRequest::Resize { size },
                } if attachment.last_host_size == Some(*size) => {
                    emitted = None;
                    suppressed = Some(AdapterEvent::ResizeCycleSuppressed {
                        generation,
                        size: *size,
                    });
                }
                HelperEventKind::ChildRequest { request } => {
                    attachment.requests.push(request.clone());
                }
                HelperEventKind::HelperLost { .. } => attachment.failed = true,
                HelperEventKind::FocusChanged { .. }
                | HelperEventKind::VisibilityChanged { .. } => {}
            }
        }
        if let Some(event) = suppressed {
            self.emit(event);
        }
        if let Some(event) = emitted {
            self.emit(AdapterEvent::Runtime { generation, event });
        }
        Ok(())
    }

    fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), IsolatedWindowError> {
        if plan.island_id() != self.spec.island_id() {
            return Err(IsolatedWindowError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: plan.island_id().clone(),
            });
        }
        if plan.operations().iter().any(|operation| {
            matches!(
                operation.operation(),
                NativeContentOperation::Attach { mechanism, .. }
                    if *mechanism != NativeContentMechanism::IsolatedWindow
            ) || matches!(
                operation.operation(),
                NativeContentOperation::SetChildBounds { .. }
                    | NativeContentOperation::SetBackingViewport { .. }
            )
        }) {
            return Err(IsolatedWindowError::WrongMechanism);
        }
        let state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        if let Some(attachment) = state.attachment.as_ref() {
            if plan.generation() < attachment.generation {
                return Err(IsolatedWindowError::StaleGeneration {
                    current: attachment.generation,
                    supplied: plan.generation(),
                });
            }
            if plan.generation() > attachment.generation
                && !(attachment.failed
                    && attachment
                        .generation
                        .checked_next()
                        .is_ok_and(|next| next == plan.generation()))
            {
                return Err(IsolatedWindowError::CurrentGenerationAttached(
                    attachment.generation,
                ));
            }
            if attachment.failed && plan.generation() == attachment.generation {
                return Err(IsolatedWindowError::FailedGeneration);
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
    ) -> Result<(), IsolatedWindowError> {
        match operation {
            NativeContentOperation::Attach {
                host_window_id,
                mechanism: NativeContentMechanism::IsolatedWindow,
            } => {
                if host_window_id != self.spec.host_window_id() {
                    return Err(IsolatedWindowError::HostBindingMismatch);
                }
                self.attach(generation)
            }
            NativeContentOperation::Attach { .. }
            | NativeContentOperation::SetChildBounds { .. }
            | NativeContentOperation::SetBackingViewport { .. } => {
                Err(IsolatedWindowError::WrongMechanism)
            }
            NativeContentOperation::SetIsolatedContentSize { size } => {
                let handle = self.handle(generation)?;
                self.runtime.set_content_size(&handle, *size)?;
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| IsolatedWindowError::Poisoned)?;
                if let Some(attachment) = state.attachment.as_mut() {
                    attachment.last_host_size = Some(*size);
                }
                Ok(())
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
                Err(IsolatedWindowError::UnsupportedInputMode)
            }
            NativeContentOperation::RequestFocus => {
                let handle = self.handle(generation)?;
                self.runtime.focus(&handle)
            }
            NativeContentOperation::ReleaseFocusIfOwned => {
                let handle = self.handle(generation)?;
                self.runtime.release_focus(&handle)
            }
            NativeContentOperation::Detach {
                policy: DetachPolicy::OwnerProcessTermination,
            } => self.detach(generation),
            NativeContentOperation::Detach { .. } => {
                Err(IsolatedWindowError::UnsupportedDetachPolicy)
            }
        }
    }

    fn attach(&self, generation: AttachGeneration) -> Result<(), IsolatedWindowError> {
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            if state.attachment.as_ref().is_some_and(|attachment| {
                attachment.failed
                    && attachment
                        .generation
                        .checked_next()
                        .is_ok_and(|next| next == generation)
            }) {
                state.attachment = None;
            }
            if let Some(attachment) = state.attachment.as_ref() {
                if attachment.generation == generation && attachment.handle.is_some() {
                    return Ok(());
                }
                return Err(IsolatedWindowError::CurrentGenerationAttached(
                    attachment.generation,
                ));
            }
            compare_generation_allow_next(state.latest_generation, generation)?;
            state.latest_generation = Some(generation);
            state.attachment = Some(Attachment {
                generation,
                handle: None,
                ready: false,
                failed: false,
                last_host_size: None,
                requests: Vec::new(),
            });
        }
        self.emit(AdapterEvent::ListenerInstalled { generation });
        let callback_adapter = self.clone();
        let callback = Arc::new(move |event| {
            let _ = callback_adapter.admit_runtime_event(event);
        });
        self.emit(AdapterEvent::AttachStarted { generation });
        let handle = match self.runtime.attach(RuntimeAttachRequest {
            island_id: self.spec.island_id().clone(),
            generation,
            host_window_id: self.spec.host_window_id().clone(),
            callback,
        }) {
            Ok(handle) => handle,
            Err(error) => {
                self.clear_reservation(generation)?;
                return Err(error);
            }
        };
        let retained = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            match state.attachment.as_mut() {
                Some(attachment) if attachment.generation == generation => {
                    attachment.handle = Some(handle.clone());
                    true
                }
                _ => false,
            }
        };
        if !retained {
            let _ = self.runtime.teardown(&handle, self.spec.teardown_timeout());
            return Err(IsolatedWindowError::NotAttached);
        }
        self.emit(AdapterEvent::Attached { generation });
        Ok(())
    }

    fn detach(&self, generation: AttachGeneration) -> Result<(), IsolatedWindowError> {
        let handle = self.handle(generation)?;
        self.emit(AdapterEvent::DetachStarted { generation });
        let outcome = self
            .runtime
            .teardown(&handle, self.spec.teardown_timeout())?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            state.teardown_reports.push((generation, outcome.clone()));
            if !matches!(outcome, TeardownOutcome::TimedOut { .. })
                && state
                    .attachment
                    .as_ref()
                    .is_some_and(|attachment| attachment.generation == generation)
            {
                state.attachment = None;
            }
        }
        self.emit(AdapterEvent::TeardownReported {
            generation,
            outcome: outcome.clone(),
        });
        if matches!(outcome, TeardownOutcome::TimedOut { .. }) {
            return Err(IsolatedWindowError::Runtime {
                operation: "teardown",
                detail: "helper missed bounded teardown deadline".to_string(),
            });
        }
        Ok(())
    }

    fn handle(&self, generation: AttachGeneration) -> Result<R::Handle, IsolatedWindowError> {
        let state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        let attachment = state
            .attachment
            .as_ref()
            .ok_or(IsolatedWindowError::NotAttached)?;
        if attachment.generation != generation {
            return Err(compare_attached_generation(
                attachment.generation,
                generation,
            ));
        }
        if attachment.failed {
            return Err(IsolatedWindowError::FailedGeneration);
        }
        attachment
            .handle
            .clone()
            .ok_or(IsolatedWindowError::AttachInProgress)
    }

    fn clear_reservation(&self, generation: AttachGeneration) -> Result<(), IsolatedWindowError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
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
) -> Result<(), IsolatedWindowError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        Err(IsolatedWindowError::StaleGeneration { current, supplied })
    } else if supplied > current {
        Err(IsolatedWindowError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

fn compare_generation_allow_next(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), IsolatedWindowError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        return Err(IsolatedWindowError::StaleGeneration { current, supplied });
    }
    let next = current.checked_next().ok();
    if supplied == current || Some(supplied) == next {
        Ok(())
    } else {
        Err(IsolatedWindowError::FutureGeneration { current, supplied })
    }
}

fn compare_attached_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> IsolatedWindowError {
    if supplied < current {
        IsolatedWindowError::StaleGeneration { current, supplied }
    } else {
        IsolatedWindowError::FutureGeneration { current, supplied }
    }
}
