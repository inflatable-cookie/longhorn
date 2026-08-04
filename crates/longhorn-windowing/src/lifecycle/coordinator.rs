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
        if let Some(latest) = state.last_input_at
            && at < latest
        {
            return Ok(ApplyRegistrationOutcome::StaleTimestamp { latest });
        }
        if let Some(current) = state.apply.as_ref().map(|apply| apply.generation)
            && generation < current
        {
            return Ok(ApplyRegistrationOutcome::StaleGeneration { current });
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
            && at < latest
        {
            return Ok(vec![ignore(
                window_id,
                IgnoreReason::StaleTimestamp { latest },
            )]);
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

    /// Moves one window's complete state — pending capture, debounce,
    /// generation, and apply expectation — to a new identity. State already
    /// registered under the new identity (an apply expectation installed
    /// before the retag) is merged, preferring the previous capture lineage
    /// and the newest apply evidence. Returns re-schedule directives for
    /// pending deadlines so the host can deliver them under the new
    /// identity; wakes still queued under the previous identity fail as
    /// unknown and must be treated as superseded.
    pub fn retag(
        &mut self,
        previous: &WindowId,
        next: &WindowId,
    ) -> Result<Vec<WindowLifecycleDirective>, WindowLifecycleError> {
        if previous == next {
            return Ok(Vec::new());
        }
        let Some(mut state) = self.windows.remove(previous) else {
            return Ok(Vec::new());
        };
        if let Some(existing) = self.windows.remove(next) {
            state.apply = match (state.apply.take(), existing.apply) {
                (Some(previous_apply), Some(next_apply)) => {
                    Some(if next_apply.generation >= previous_apply.generation {
                        next_apply
                    } else {
                        previous_apply
                    })
                }
                (previous_apply, next_apply) => next_apply.or(previous_apply),
            };
            state.last_input_at = state.last_input_at.max(existing.last_input_at);
            state.user_until = state.user_until.max(existing.user_until);
            state.capture_generation = state.capture_generation.max(existing.capture_generation);
            state.pending = state.pending.or(existing.pending);
        }
        let mut directives = Vec::new();
        if let Some(pending) = &state.pending {
            if !pending.captured {
                directives.push(WindowLifecycleDirective::ScheduleCapture {
                    window_id: next.clone(),
                    generation: pending.generation,
                    due_at: pending.capture_due_at,
                });
            }
            if let Some(due_at) = pending.flush_due_at {
                directives.push(WindowLifecycleDirective::ScheduleFlush {
                    window_id: next.clone(),
                    generation: pending.generation,
                    due_at,
                });
            }
        }
        self.windows.insert(next.clone(), state);
        Ok(directives)
    }

    /// Releases one window's coordinator state without emitting directives.
    /// Used by hosts to undo state recreated by an event that lost a race
    /// with destruction. Returns whether state existed.
    pub fn release(&mut self, window_id: &WindowId) -> bool {
        self.windows.remove(window_id).is_some()
    }
}
