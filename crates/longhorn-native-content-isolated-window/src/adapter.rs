use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use longhorn_core::{ClientSize, NativeContentFailureCode, NativeContentRequestId, PhysicalSize};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, ContentSizeDecision,
    ContentSizeProposal, ContentSizeProposalReceipt, DetachPolicy, EffectiveFocus,
    EffectiveVisibility, InputRoutingMode, NativeContentCoordinator, NativeContentMechanism,
    NativeContentOperation, ObservationUpdate, ObservedGeometry, ObservedReadiness, StepExecution,
};

use crate::{
    HelperSnapshot, IsolatedContentRequest, IsolatedContentRequestKind, IsolatedWindowAdapterEvent,
    IsolatedWindowError, IsolatedWindowRuntime, IsolatedWindowRuntimeEvent,
    IsolatedWindowRuntimeEventKind, IsolatedWindowSpec, RuntimeAttachRequest, TeardownOutcome,
};

const MAX_PENDING_CONTENT_REQUESTS: usize = 128;

struct Attachment<H> {
    generation: AttachGeneration,
    handle: Option<H>,
    ready: bool,
    detaching: bool,
    failed: bool,
    last_host_size: Option<PhysicalSize>,
    requests: Vec<IsolatedContentRequest>,
    seen_request_ids: HashSet<NativeContentRequestId>,
}

struct AdapterState<H> {
    latest_generation: Option<AttachGeneration>,
    retired_generation: Option<AttachGeneration>,
    attachment: Option<Attachment<H>>,
    teardown_reports: Vec<(AttachGeneration, TeardownOutcome)>,
}

impl<H> Default for AdapterState<H> {
    fn default() -> Self {
        Self {
            latest_generation: None,
            retired_generation: None,
            attachment: None,
            teardown_reports: Vec::new(),
        }
    }
}

/// Generation-checked isolated-window executor over one injected owner runtime.
pub struct IsolatedWindowAdapter<R: IsolatedWindowRuntime> {
    runtime: R,
    spec: IsolatedWindowSpec,
    state: Arc<Mutex<AdapterState<R::Handle>>>,
    observer: Arc<dyn Fn(IsolatedWindowAdapterEvent) + Send + Sync>,
}

impl<R: IsolatedWindowRuntime> Clone for IsolatedWindowAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: Arc::clone(&self.state),
            observer: Arc::clone(&self.observer),
        }
    }
}

impl<R: IsolatedWindowRuntime> IsolatedWindowAdapter<R> {
    /// Creates one adapter from explicit mapping, timeout policy, and observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: IsolatedWindowSpec,
        observer: Arc<dyn Fn(IsolatedWindowAdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable mapping and timeout policy.
    #[must_use]
    pub const fn spec(&self) -> &IsolatedWindowSpec {
        &self.spec
    }

    /// Executes a current immutable plan and returns coordinator-validated evidence.
    pub fn apply(
        &self,
        authority: &NativeContentCoordinator,
        plan: &ApplyPlan,
    ) -> Result<ApplyReceipt, IsolatedWindowError> {
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

    /// Reads fresh content size, visibility, and focus from the owner runtime.
    pub fn observe(
        &self,
        generation: AttachGeneration,
    ) -> Result<ObservationUpdate, IsolatedWindowError> {
        let attachment = {
            let state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
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
                    attachment.failed,
                )),
            }
        };

        let Some((handle, ready, detaching, failed)) = attachment else {
            return Ok(ObservationUpdate::absent(generation));
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
        let HelperSnapshot {
            content_size,
            visible,
            focused,
        } = self.runtime.observe(&handle, self.spec.request_timeout())?;
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
            if visible {
                EffectiveVisibility::Visible
            } else {
                EffectiveVisibility::Hidden
            },
            if focused {
                EffectiveFocus::Focused
            } else {
                EffectiveFocus::Unfocused
            },
            ObservedGeometry::IsolatedContent { size: content_size },
            Some(InputRoutingMode::NativeDirect),
        ))
    }

    /// Removes and returns current consumer-admitted content requests.
    pub fn take_requests(
        &self,
        generation: AttachGeneration,
    ) -> Result<Vec<IsolatedContentRequest>, IsolatedWindowError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        let attachment = current_attachment_mut(&mut state, generation)?;
        if attachment.failed {
            return Err(IsolatedWindowError::FailedGeneration);
        }
        Ok(std::mem::take(&mut attachment.requests))
    }

    /// Applies consumer policy to one resize request without mutating desired state.
    pub fn decide_resize(
        &self,
        authority: &NativeContentCoordinator,
        generation: AttachGeneration,
        request: &IsolatedContentRequest,
        decision: ContentSizeDecision,
    ) -> Result<ContentSizeProposalReceipt, IsolatedWindowError> {
        self.handle(generation)?;
        let IsolatedContentRequestKind::Resize { size } = request.request else {
            return Err(IsolatedWindowError::NotResizeRequest);
        };
        let desired = authority.desired();
        if desired.generation() != generation {
            return Err(compare_attached_generation(
                desired.generation(),
                generation,
            ));
        }
        let scale = f64::from(desired.scale().thousandths());
        let semantic = ClientSize::new(
            f64::from(size.width()) * 1000.0 / scale,
            f64::from(size.height()) * 1000.0 / scale,
        )
        .map_err(|error| IsolatedWindowError::Runtime {
            operation: "size",
            detail: error.to_string(),
        })?;
        authority
            .decide_content_size(
                ContentSizeProposal::new(generation, desired.revision(), semantic),
                decision,
            )
            .map_err(Into::into)
    }

    /// Applies one explicitly admitted native resize hint.
    pub fn set_resizable(
        &self,
        generation: AttachGeneration,
        resizable: bool,
    ) -> Result<(), IsolatedWindowError> {
        let handle = self.handle(generation)?;
        self.runtime
            .set_resizable(&handle, resizable, self.spec.request_timeout())
    }

    /// Returns all exact teardown reports without consuming them.
    pub fn teardown_reports(
        &self,
    ) -> Result<Vec<(AttachGeneration, TeardownOutcome)>, IsolatedWindowError> {
        self.state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)
            .map(|state| state.teardown_reports.clone())
    }

    /// Admits callbacks only while exact island and generation remain current.
    pub fn admit_runtime_event(
        &self,
        event: IsolatedWindowRuntimeEvent,
    ) -> Result<(), IsolatedWindowError> {
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
            compare_generation(state.latest_generation, generation)?;
            if state.retired_generation == Some(generation) {
                return Err(IsolatedWindowError::GenerationRetired(generation));
            }
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
            match &event.kind {
                IsolatedWindowRuntimeEventKind::Progress { .. } => {}
                IsolatedWindowRuntimeEventKind::Ready { .. } => attachment.ready = true,
                IsolatedWindowRuntimeEventKind::ContentRequest { request } => {
                    if !attachment
                        .seen_request_ids
                        .insert(request.request_id.clone())
                    {
                        return Err(IsolatedWindowError::DuplicateCorrelation);
                    }
                    if attachment.requests.len() >= MAX_PENDING_CONTENT_REQUESTS {
                        return Err(IsolatedWindowError::RequestCapacity);
                    }
                    if matches!(
                        request.request,
                        IsolatedContentRequestKind::Resize { size }
                            if attachment.last_host_size == Some(size)
                    ) {
                        emitted = None;
                        suppressed = Some(IsolatedWindowAdapterEvent::ResizeCycleSuppressed {
                            generation,
                            size: attachment.last_host_size.expect("matched host size"),
                        });
                    } else {
                        attachment.requests.push(request.clone());
                    }
                }
                IsolatedWindowRuntimeEventKind::HelperLost { .. } => attachment.failed = true,
                IsolatedWindowRuntimeEventKind::FocusChanged { .. }
                | IsolatedWindowRuntimeEventKind::VisibilityChanged { .. } => {}
            }
        }
        if let Some(event) = suppressed {
            self.emit(event);
        }
        if let Some(event) = emitted {
            self.emit(IsolatedWindowAdapterEvent::Runtime { generation, event });
        }
        Ok(())
    }

    /// Returns whether a usable, non-failed owner is current.
    pub fn is_attached(&self, generation: AttachGeneration) -> Result<bool, IsolatedWindowError> {
        let state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        Ok(state.attachment.as_ref().is_some_and(|attachment| {
            attachment.generation == generation && attachment.handle.is_some() && !attachment.failed
        }))
    }

    fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), IsolatedWindowError> {
        if plan.island_id() != self.spec.island_id() {
            return Err(IsolatedWindowError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: plan.island_id().clone(),
            });
        }
        if plan.operations().iter().any(|planned| {
            matches!(
                planned.operation(),
                NativeContentOperation::Attach { mechanism, .. }
                    if *mechanism != NativeContentMechanism::IsolatedWindow
            ) || matches!(
                planned.operation(),
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
                    && attachment.generation.checked_next().ok() == Some(plan.generation()))
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
            if state.retired_generation == Some(plan.generation())
                && plan.operations().iter().any(|planned| {
                    matches!(planned.operation(), NativeContentOperation::Attach { .. })
                })
            {
                return Err(IsolatedWindowError::GenerationRetired(plan.generation()));
            }
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
                self.runtime
                    .set_content_size(&handle, *size, self.spec.request_timeout())?;
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| IsolatedWindowError::Poisoned)?;
                current_attachment_mut(&mut state, generation)?.last_host_size = Some(*size);
                Ok(())
            }
            NativeContentOperation::Show => {
                let handle = self.handle(generation)?;
                self.runtime.show(&handle, self.spec.request_timeout())
            }
            NativeContentOperation::Hide { .. } => {
                let handle = self.handle(generation)?;
                self.runtime.hide(&handle, self.spec.request_timeout())
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
                self.runtime.focus(&handle, self.spec.request_timeout())
            }
            NativeContentOperation::ReleaseFocusIfOwned => {
                let handle = self.handle(generation)?;
                self.runtime
                    .release_focus(&handle, self.spec.request_timeout())
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
                attachment.failed && attachment.generation.checked_next().ok() == Some(generation)
            }) {
                let retired = state.attachment.as_ref().map(|value| value.generation);
                state.attachment = None;
                state.retired_generation = retired;
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
            if state.retired_generation == Some(generation) {
                return Err(IsolatedWindowError::GenerationRetired(generation));
            }
            state.latest_generation = Some(generation);
            state.attachment = Some(Attachment {
                generation,
                handle: None,
                ready: false,
                detaching: false,
                failed: false,
                last_host_size: None,
                requests: Vec::new(),
                seen_request_ids: HashSet::new(),
            });
        }

        self.emit(IsolatedWindowAdapterEvent::ListenerInstalled { generation });
        let callback_adapter = self.clone();
        let callback = Arc::new(move |event| {
            let _ = callback_adapter.admit_runtime_event(event);
        });
        self.emit(IsolatedWindowAdapterEvent::AttachStarted { generation });
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
        self.emit(IsolatedWindowAdapterEvent::Attached { generation });
        Ok(())
    }

    fn detach(&self, generation: AttachGeneration) -> Result<(), IsolatedWindowError> {
        let handle = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            let Some(attachment) = state.attachment.as_mut() else {
                if state.retired_generation == Some(generation) {
                    return Ok(());
                }
                return Err(IsolatedWindowError::NotAttached);
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
                .ok_or(IsolatedWindowError::AttachInProgress)?;
            attachment.detaching = true;
            handle
        };
        self.emit(IsolatedWindowAdapterEvent::DetachStarted { generation });
        let outcome = match self.runtime.teardown(&handle, self.spec.teardown_timeout()) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.clear_detaching(generation)?;
                return Err(error);
            }
        };
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            state.teardown_reports.push((generation, outcome.clone()));
            if matches!(outcome, TeardownOutcome::TimedOut { .. }) {
                if let Some(attachment) = state.attachment.as_mut() {
                    if attachment.generation == generation {
                        attachment.detaching = false;
                    }
                }
            } else {
                if state
                    .attachment
                    .as_ref()
                    .is_some_and(|value| value.generation == generation)
                {
                    state.attachment = None;
                }
                state.retired_generation = Some(generation);
            }
        }
        self.emit(IsolatedWindowAdapterEvent::TeardownReported {
            generation,
            outcome: outcome.clone(),
        });
        if matches!(outcome, TeardownOutcome::TimedOut { .. }) {
            return Err(IsolatedWindowError::Runtime {
                operation: "teardown",
                detail: "owner missed bounded teardown deadline".to_string(),
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
        if state.retired_generation == Some(generation) {
            return Err(IsolatedWindowError::GenerationRetired(generation));
        }
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

    fn clear_detaching(&self, generation: AttachGeneration) -> Result<(), IsolatedWindowError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        if let Some(attachment) = state.attachment.as_mut() {
            if attachment.generation == generation {
                attachment.detaching = false;
            }
        }
        Ok(())
    }

    fn emit(&self, event: IsolatedWindowAdapterEvent) {
        (self.observer)(event);
    }
}

fn current_attachment_mut<H>(
    state: &mut AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&mut Attachment<H>, IsolatedWindowError> {
    compare_generation(state.latest_generation, generation)?;
    if state.retired_generation == Some(generation) {
        return Err(IsolatedWindowError::GenerationRetired(generation));
    }
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
    Ok(attachment)
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
    if supplied == current || current.checked_next().ok() == Some(supplied) {
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
