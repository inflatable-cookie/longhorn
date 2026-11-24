//! Public adapter operations.

use longhorn_core::WindowId;
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, EffectiveFocus,
    EffectiveVisibility, NativeContentCoordinator, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, execute_adapter_plan, gate_attached,
};
use tauri::Url;

use crate::{
    ChildViewAdapterEvent, ChildViewError, ChildViewRuntimeEvent, ChildViewRuntimeEventKind,
};

use super::{
    ChildViewAdapter, ChildViewHostDestroyOutcome, ChildViewHostDestroyReceipt,
    ChildViewNavigationOutcome, ChildViewNavigationReceipt, ChildViewTeardownOutcome,
    ChildViewTeardownReceipt, compare_attached_generation, compare_generation,
};

impl<R: crate::ChildViewRuntime> ChildViewAdapter<R> {
    /// Executes a current immutable plan and returns coordinator-validated evidence.
    pub fn apply(
        &self,
        authority: &NativeContentCoordinator,
        plan: &ApplyPlan,
    ) -> Result<ApplyReceipt, ChildViewError> {
        self.validate_plan(plan)?;
        execute_adapter_plan(
            authority,
            plan,
            || Ok(()),
            |planned| self.execute(plan.generation(), planned.operation()),
            ChildViewError::failure_code,
        )
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
                Some(attachment) => {
                    compare_attached_generation(attachment.generation, generation)?;
                    Some((
                        attachment.handle.clone(),
                        attachment.ready,
                        attachment.detaching,
                        attachment.input_routing,
                    ))
                }
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
        (self.observer)(ChildViewAdapterEvent::RendererUnmounted { generation });
        Ok(())
    }

    /// Requests native focus once for the current child without claiming effective focus.
    pub fn request_focus(&self, generation: AttachGeneration) -> Result<(), ChildViewError> {
        let handle = self.handle(generation)?;
        self.runtime.focus(&handle)
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
                compare_attached_generation(attachment.generation, generation)?;
                state.attachment = None;
                state.retired_generation = Some(generation);
                state.invalidated_generation = Some(generation);
                ChildViewHostDestroyOutcome::Invalidated
            }
        };
        if outcome == ChildViewHostDestroyOutcome::Invalidated {
            (self.observer)(ChildViewAdapterEvent::HostInvalidated { generation });
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
        if event.island_id != *self.spec.island_id() {
            return Err(ChildViewError::ForeignIsland {
                expected: self.spec.island_id().clone(),
                supplied: event.island_id,
            });
        }
        if event.child_label != *self.spec.child_label() {
            return Err(ChildViewError::NotAttached);
        }
        {
            let mut state = self.state.lock().map_err(|_| ChildViewError::Poisoned)?;
            let latest = state.latest_generation;
            let retired = state.retired_generation;
            let attachment = gate_attached(
                latest,
                retired,
                state.attachment.as_mut(),
                event.generation,
                |attachment| attachment.generation,
            )?;
            match event.kind {
                ChildViewRuntimeEventKind::PageLoadStarted => attachment.ready = false,
                ChildViewRuntimeEventKind::PageLoadFinished => attachment.ready = true,
            }
        }
        (self.observer)(ChildViewAdapterEvent::Runtime {
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
}
