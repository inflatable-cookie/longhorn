//! Plan validation and native operation execution.

use std::sync::Arc;

use longhorn_native_content::{
    ApplyPlan, AttachGeneration, AttachmentGate, DetachPolicy, InputRoutingMode,
    NativeContentMechanism, NativeContentOperation, check_attach_reservation,
    clear_attach_reservation, complete_reserved_attach, gate_detach, validate_plan_generation,
};

use crate::{
    BackingSurfaceAdapterEvent, BackingSurfaceError, BackingSurfaceRuntime, BackingSurfaceSnapshot,
    RuntimeAttachRequest,
};

use super::{
    Attachment, BackingSurfaceAdapter, BackingSurfaceDetachOutcome, compare_generation,
    current_attachment, current_attachment_mut, reject_invalidated, validate_snapshot,
};

impl<R: BackingSurfaceRuntime> BackingSurfaceAdapter<R> {
    pub(crate) fn validate_plan(&self, plan: &ApplyPlan) -> Result<(), BackingSurfaceError> {
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
        let attached = state
            .attachment
            .as_ref()
            .map(|attachment| attachment.generation);
        validate_plan_generation(
            state.latest_generation,
            state.invalidated_generation.or(state.retired_generation),
            attached,
            plan.generation(),
            plan.operations().iter().any(|planned| {
                matches!(planned.operation(), NativeContentOperation::Attach { .. })
            }),
        )?;
        if attached.is_some() {
            reject_invalidated(state.invalidated_generation, plan.generation())?;
        }
        Ok(())
    }

    pub(crate) fn execute(
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
                mode: mode @ (InputRoutingMode::RendererForwarded | InputRoutingMode::Disabled),
            } => {
                let handle = self.handle(generation)?;
                let snapshot = self.runtime.set_input_routing(&handle, *mode)?;
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

    pub(crate) fn attach(&self, generation: AttachGeneration) -> Result<(), BackingSurfaceError> {
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            if check_attach_reservation(
                state.latest_generation,
                state.invalidated_generation.or(state.retired_generation),
                state.attachment.as_ref().map(|attachment| {
                    AttachmentGate::new(attachment.generation, attachment.handle.is_some())
                }),
                generation,
            )? {
                return Ok(());
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

        complete_reserved_attach(
            [
                BackingSurfaceAdapterEvent::ListenerInstalled { generation },
                BackingSurfaceAdapterEvent::AttachStarted { generation },
                BackingSurfaceAdapterEvent::Attached { generation },
            ],
            self.observer.as_ref(),
            || {
                let callback_adapter = self.clone();
                let callback = Arc::new(move |event| {
                    if let Err(error) = callback_adapter.admit_runtime_event(event) {
                        longhorn_core::report_best_effort_failure(
                            "native-content.backing-surface.runtime-event",
                            format_args!("{error:?}"),
                        );
                    }
                });
                self.runtime.attach(RuntimeAttachRequest {
                    generation,
                    spec: self.spec.clone(),
                    callback,
                })
            },
            |(handle, snapshot)| self.retain_attach(generation, handle, snapshot),
            || self.clear_reservation(generation),
        )
    }

    fn retain_attach(
        &self,
        generation: AttachGeneration,
        handle: R::Handle,
        snapshot: BackingSurfaceSnapshot,
    ) -> Result<(), BackingSurfaceError> {
        let validation = validate_snapshot(&snapshot).and_then(|()| {
            if snapshot.native_storage_attached {
                Ok(())
            } else {
                Err(BackingSurfaceError::Runtime {
                    operation: "attach",
                    detail: "runtime returned detached native storage".to_string(),
                })
            }
        });
        let invalidated = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            let invalidated = state.invalidated_generation == Some(generation);
            // A backing reservation stays in place through host destruction.
            // Keep the returned handle until teardown succeeds, including on retry.
            let attachment = state
                .attachment
                .as_mut()
                .ok_or(BackingSurfaceError::NotAttached)?;
            compare_generation(Some(attachment.generation), generation)?;
            if invalidated || validation.is_ok() {
                attachment.handle = Some(handle.clone());
                attachment.snapshot = Some(snapshot);
            }
            invalidated
        };
        if invalidated {
            self.detach_generation(generation)?;
            return Err(BackingSurfaceError::GenerationInvalidated(generation));
        }
        if validation.is_err()
            && let Err(error) = self.runtime.detach(&handle)
        {
            longhorn_core::report_best_effort_failure(
                "native-content.backing-surface.detach",
                format_args!("{error:?}"),
            );
        }
        validation
    }

    pub(crate) fn detach_generation(
        &self,
        generation: AttachGeneration,
    ) -> Result<BackingSurfaceDetachOutcome, BackingSurfaceError> {
        let handle = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            compare_generation(state.latest_generation, generation)?;
            if state.attachment.is_none() && state.latest_generation == Some(generation) {
                state.retired_generation = Some(generation);
                return Ok(BackingSurfaceDetachOutcome::AlreadyDetached);
            }
            let Some(attachment) = gate_detach(
                state.retired_generation,
                state.attachment.as_mut(),
                generation,
                |attachment| {
                    AttachmentGate::new(attachment.generation, attachment.handle.is_some())
                },
            )?
            else {
                return Ok(BackingSurfaceDetachOutcome::AlreadyDetached);
            };
            if attachment.detaching {
                return Err(BackingSurfaceError::Runtime {
                    operation: "detach",
                    detail: "storage detach is already in progress".to_string(),
                });
            }
            let handle = attachment
                .handle
                .clone()
                .ok_or(BackingSurfaceError::AttachInProgress)?;
            attachment.detaching = true;
            handle
        };
        (self.observer)(BackingSurfaceAdapterEvent::DetachStarted { generation });
        if let Err(error) = self.runtime.detach(&handle) {
            let mut state = self
                .state
                .lock()
                .map_err(|_| BackingSurfaceError::Poisoned)?;
            if let Some(attachment) = state.attachment.as_mut()
                && attachment.generation == generation
            {
                attachment.detaching = false;
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
        (self.observer)(BackingSurfaceAdapterEvent::Detached { generation });
        Ok(BackingSurfaceDetachOutcome::Detached)
    }

    pub(crate) fn optional_handle(
        &self,
        generation: AttachGeneration,
    ) -> Result<Option<R::Handle>, BackingSurfaceError> {
        let state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        Ok(current_attachment(&state, generation)?.handle.clone())
    }

    pub(crate) fn handle(
        &self,
        generation: AttachGeneration,
    ) -> Result<R::Handle, BackingSurfaceError> {
        self.optional_handle(generation)?
            .ok_or(BackingSurfaceError::AttachInProgress)
    }

    pub(crate) fn replace_snapshot(
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
        if let Some(current) = attachment.snapshot.as_ref()
            && snapshot.frame_sequence < current.frame_sequence
        {
            return Err(BackingSurfaceError::StaleFrameSequence {
                current: current.frame_sequence,
                supplied: snapshot.frame_sequence,
            });
        }
        attachment.snapshot = Some(snapshot);
        Ok(())
    }

    pub(crate) fn clear_reservation(
        &self,
        generation: AttachGeneration,
    ) -> Result<(), BackingSurfaceError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        clear_attach_reservation(&mut state.attachment, generation, |attachment| {
            AttachmentGate::new(attachment.generation, attachment.handle.is_some())
        });
        Ok(())
    }
}
