//! Plan validation and native operation execution.

use std::{collections::HashSet, sync::Arc};

use longhorn_native_content::{
    ApplyPlan, AttachGeneration, AttachmentGate, DetachPolicy, InputRoutingMode,
    NativeContentMechanism, NativeContentOperation, check_attach_reservation, gate_attached,
    gate_detach, validate_plan_generation,
};

use crate::{
    IsolatedWindowAdapterEvent, IsolatedWindowError, IsolatedWindowRuntime, RuntimeAttachRequest,
    TeardownOutcome,
};

use super::{Attachment, IsolatedWindowAdapter, compare_generation, current_attachment_mut};

impl<R: IsolatedWindowRuntime> IsolatedWindowAdapter<R> {
    pub(crate) fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), IsolatedWindowError> {
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
            // Mechanism-specific extension (contract 017): a terminally failed
            // owner rejects its own generation and yields only to exactly the
            // next generation.
            if attachment.failed {
                if plan.generation() == attachment.generation {
                    return Err(IsolatedWindowError::FailedGeneration);
                }
                if attachment.generation.checked_next().ok() == Some(plan.generation()) {
                    return Ok(());
                }
            }
        }
        validate_plan_generation(
            state.latest_generation,
            state.retired_generation,
            state
                .attachment
                .as_ref()
                .map(|attachment| attachment.generation),
            plan.generation(),
            plan.operations().iter().any(|planned| {
                matches!(planned.operation(), NativeContentOperation::Attach { .. })
            }),
        )?;
        Ok(())
    }

    pub(crate) fn execute(
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

    pub(crate) fn attach(&self, generation: AttachGeneration) -> Result<(), IsolatedWindowError> {
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            // Mechanism-specific extension (contract 017): a terminally failed
            // owner retires when exactly the next generation attaches.
            if state.attachment.as_ref().is_some_and(|attachment| {
                attachment.failed && attachment.generation.checked_next().ok() == Some(generation)
            }) {
                let retired = state.attachment.as_ref().map(|value| value.generation);
                state.attachment = None;
                state.retired_generation = retired;
            }
            if check_attach_reservation(
                state.latest_generation,
                state.retired_generation,
                state.attachment.as_ref().map(|attachment| {
                    AttachmentGate::new(attachment.generation, attachment.handle.is_some())
                }),
                generation,
            )? {
                return Ok(());
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
            if let Err(error) = callback_adapter.admit_runtime_event(event) {
                longhorn_core::report_best_effort_failure(
                    "native-content.isolated-window.runtime-event",
                    format_args!("{error:?}"),
                );
            }
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
            if let Err(error) = self.runtime.teardown(&handle, self.spec.teardown_timeout()) {
                longhorn_core::report_best_effort_failure(
                    "native-content.isolated-window.teardown",
                    format_args!("{error:?}"),
                );
            }
            return Err(IsolatedWindowError::NotAttached);
        }
        self.emit(IsolatedWindowAdapterEvent::Attached { generation });
        Ok(())
    }

    pub(crate) fn detach(&self, generation: AttachGeneration) -> Result<(), IsolatedWindowError> {
        let handle = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            if !gate_detach(
                state.retired_generation,
                state.attachment.as_ref().map(|attachment| {
                    AttachmentGate::new(attachment.generation, attachment.handle.is_some())
                }),
                generation,
            )? {
                return Ok(());
            }
            let attachment = state
                .attachment
                .as_mut()
                .expect("validated attachment is current");
            let handle = attachment
                .handle
                .clone()
                .expect("validated attachment completed attach");
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
                if let Some(attachment) = state.attachment.as_mut()
                    && attachment.generation == generation
                {
                    attachment.detaching = false;
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

    pub(crate) fn handle(
        &self,
        generation: AttachGeneration,
    ) -> Result<R::Handle, IsolatedWindowError> {
        let state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        compare_generation(state.latest_generation, generation)?;
        gate_attached(
            state.retired_generation,
            state
                .attachment
                .as_ref()
                .map(|attachment| attachment.generation),
            generation,
        )?;
        let attachment = state
            .attachment
            .as_ref()
            .expect("validated attachment is current");
        if attachment.failed {
            return Err(IsolatedWindowError::FailedGeneration);
        }
        attachment
            .handle
            .clone()
            .ok_or(IsolatedWindowError::AttachInProgress)
    }

    pub(crate) fn clear_reservation(
        &self,
        generation: AttachGeneration,
    ) -> Result<(), IsolatedWindowError> {
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

    pub(crate) fn clear_detaching(
        &self,
        generation: AttachGeneration,
    ) -> Result<(), IsolatedWindowError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        if let Some(attachment) = state.attachment.as_mut()
            && attachment.generation == generation
        {
            attachment.detaching = false;
        }
        Ok(())
    }

    pub(crate) fn emit(&self, event: IsolatedWindowAdapterEvent) {
        (self.observer)(event);
    }
}
