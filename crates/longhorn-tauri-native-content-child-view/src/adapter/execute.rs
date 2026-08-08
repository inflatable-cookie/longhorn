//! Plan validation and native operation execution.

use std::sync::Arc;

use longhorn_native_content::{
    ApplyPlan, AttachGeneration, DetachPolicy, InputRoutingMode, NativeContentMechanism,
    NativeContentOperation,
};

use crate::{ChildViewAdapterEvent, ChildViewError, ChildViewRuntime, RuntimeAttachRequest};

use super::{
    Attachment, ChildViewAdapter, compare_attached_generation, compare_generation,
    compare_generation_allow_next, current_attachment_mut,
};

impl<R: ChildViewRuntime> ChildViewAdapter<R> {
    pub(crate) fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), ChildViewError> {
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

    pub(crate) fn execute(
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

    pub(crate) fn attach(&self, generation: AttachGeneration) -> Result<(), ChildViewError> {
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
            if let Err(error) = callback_adapter.admit_runtime_event(event) {
                longhorn_core::report_best_effort_failure(
                    "native-content.child-view.runtime-event",
                    format_args!("{error:?}"),
                );
            }
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
            if let Err(error) = self.runtime.close(&handle) {
                longhorn_core::report_best_effort_failure(
                    "native-content.child-view.close",
                    format_args!("{error:?}"),
                );
            }
            return Err(ChildViewError::NotAttached);
        }
        self.emit(ChildViewAdapterEvent::Attached { generation });
        Ok(())
    }

    pub(crate) fn detach(&self, generation: AttachGeneration) -> Result<(), ChildViewError> {
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
            if let Some(attachment) = state.attachment.as_mut()
                && attachment.generation == generation
            {
                attachment.detaching = false;
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

    pub(crate) fn handle(&self, generation: AttachGeneration) -> Result<R::Handle, ChildViewError> {
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

    pub(crate) fn clear_reservation(
        &self,
        generation: AttachGeneration,
    ) -> Result<(), ChildViewError> {
        let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
        if state.attachment.as_ref().is_some_and(|attachment| {
            attachment.generation == generation && attachment.handle.is_none()
        }) {
            state.attachment = None;
        }
        Ok(())
    }

    pub(crate) fn emit(&self, event: ChildViewAdapterEvent) {
        (self.observer)(event);
    }
}
