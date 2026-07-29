use std::collections::BTreeMap;

use longhorn_core::WindowId;

use super::{
    ApplyExpectation, ApplyGeneration, ApplyRegistrationOutcome, ExpectedEffect, FlushReason,
    IgnoreReason, MonotonicMillis, WindowLifecycleDirective, WindowLifecycleError,
    WindowLifecycleEvent, WindowLifecyclePolicy, WindowOperation, WindowState, handle_event,
    ignore,
};

/// Pure per-window apply attribution, settling, debounce, and close coordinator.
pub struct WindowLifecycleCoordinator {
    policy: WindowLifecyclePolicy,
    windows: BTreeMap<WindowId, WindowState>,
}

impl WindowLifecycleCoordinator {
    /// Constructs an empty coordinator with exact caller timing policy.
    #[must_use]
    pub const fn new(policy: WindowLifecyclePolicy) -> Self {
        Self {
            policy,
            windows: BTreeMap::new(),
        }
    }

    /// Registers one expected operation before its native mutation.
    pub fn register_apply(
        &mut self,
        at: MonotonicMillis,
        generation: ApplyGeneration,
        operation: &WindowOperation,
    ) -> Result<ApplyRegistrationOutcome, WindowLifecycleError> {
        let state = self
            .windows
            .entry(operation.window_id().clone())
            .or_default();
        if let Some(latest) = state.last_input_at {
            if at < latest {
                return Ok(ApplyRegistrationOutcome::StaleTimestamp { latest });
            }
        }
        if let Some(current) = state.apply.as_ref().map(|apply| apply.generation) {
            if generation < current {
                return Ok(ApplyRegistrationOutcome::StaleGeneration { current });
            }
        }

        let expires_at = at.checked_add(self.policy.programmatic_attribution())?;
        let effects = ExpectedEffect::from_operation(operation);
        let outcome = match state.apply.as_mut() {
            Some(apply) if apply.generation == generation => {
                apply.expires_at = apply.expires_at.max(expires_at);
                for effect in effects {
                    if !apply.effects.contains(&effect) {
                        apply.effects.push(effect);
                    }
                }
                ApplyRegistrationOutcome::Extended
            }
            _ => {
                state.apply = Some(ApplyExpectation {
                    generation,
                    expires_at,
                    effects,
                });
                ApplyRegistrationOutcome::Registered
            }
        };
        state.last_input_at = Some(at);
        Ok(outcome)
    }

    /// Processes one caller-timestamped event into ordered host directives.
    pub fn handle(
        &mut self,
        at: MonotonicMillis,
        event: WindowLifecycleEvent,
    ) -> Result<Vec<WindowLifecycleDirective>, WindowLifecycleError> {
        let window_id = event.window_id().clone();

        // Destruction is terminal native evidence. It must release state even
        // when callback delivery is reordered behind a newer timestamp.
        if matches!(event, WindowLifecycleEvent::Destroyed { .. }) {
            let generation = self
                .windows
                .remove(&window_id)
                .and_then(|state| state.pending.map(|pending| pending.generation));
            return Ok(vec![
                WindowLifecycleDirective::Flush {
                    window_id: window_id.clone(),
                    generation,
                    timeout: self.policy.flush_timeout(),
                    reason: FlushReason::Destroy,
                },
                WindowLifecycleDirective::Forget { window_id },
            ]);
        }

        if let Some(latest) = self
            .windows
            .get(&window_id)
            .and_then(|state| state.last_input_at)
        {
            if at < latest {
                return Ok(vec![ignore(
                    window_id,
                    IgnoreReason::StaleTimestamp { latest },
                )]);
            }
        }

        let directives = {
            let state = self.windows.entry(window_id.clone()).or_default();
            handle_event(self.policy, state, at, window_id.clone(), event)?
        };
        if let Some(state) = self.windows.get_mut(&window_id) {
            state.last_input_at = Some(at);
        }
        Ok(directives)
    }

    /// Returns whether one logical window currently has coordinator state.
    #[must_use]
    pub fn is_tracking(&self, window_id: &WindowId) -> bool {
        self.windows.contains_key(window_id)
    }
}
