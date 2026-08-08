//! Public adapter operations.

use longhorn_core::{ClientSize, NativeContentFailureCode, NativeContentRequestId, PhysicalSize};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, ContentSizeDecision,
    ContentSizeProposal, ContentSizeProposalReceipt, EffectiveFocus, EffectiveVisibility,
    InputRoutingMode, NativeContentCoordinator, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, StepExecution,
};

use crate::{
    HelperSnapshot, IsolatedContentRequest, IsolatedContentRequestKind, IsolatedWindowAdapterEvent,
    IsolatedWindowError, IsolatedWindowRuntimeEvent, IsolatedWindowRuntimeEventKind,
    TeardownOutcome,
};

use super::{
    IsolatedWindowAdapter, MAX_PENDING_CONTENT_REQUESTS, compare_attached_generation,
    compare_generation, compare_generation_allow_next, current_attachment_mut,
};

impl<R: crate::IsolatedWindowRuntime> IsolatedWindowAdapter<R> {
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
}
