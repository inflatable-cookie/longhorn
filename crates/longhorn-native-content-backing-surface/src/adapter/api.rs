//! Public adapter operations.

use longhorn_core::{NativeContentFailureCode, PhysicalPoint, WindowId};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, EffectiveFocus,
    EffectiveVisibility, InputRoutingMode, NativeContentCoordinator, ObservationUpdate,
    ObservedGeometry, ObservedReadiness, StepExecution,
};

use crate::{
    BackingSurfaceAdapterEvent, BackingSurfaceError, BackingSurfaceRuntimeEvent,
    BackingSurfaceRuntimeEventKind, BackingSurfaceSnapshot, InputAdmission, InputRejection,
};

use crate::runtime::contains;

use super::{
    BackingSurfaceAdapter, BackingSurfaceDetachReceipt, BackingSurfaceHostDestroyOutcome,
    BackingSurfaceHostDestroyReceipt, compare_attached_generation, compare_generation,
    current_attachment, current_attachment_mut, observation,
};

impl<R: crate::BackingSurfaceRuntime> BackingSurfaceAdapter<R> {
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
                    return Err(
                        compare_attached_generation(attachment.generation, generation).into(),
                    );
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
}
