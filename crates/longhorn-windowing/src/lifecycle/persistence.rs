use longhorn_core::WindowId;

use super::transitions::{expire_apply, ignore};
use super::{
    CaptureGeneration, CaptureReason, ExpectedEffect, FlushReason, IgnoreReason, MonotonicMillis,
    WindowLifecycleDirective, WindowLifecycleError, WindowLifecyclePolicy, WindowState,
};

pub(super) fn handle_capture_completed(
    policy: WindowLifecyclePolicy,
    state: &mut WindowState,
    at: MonotonicMillis,
    window_id: WindowId,
    generation: CaptureGeneration,
) -> Result<Vec<WindowLifecycleDirective>, WindowLifecycleError> {
    let Some(pending) = state.pending.as_mut() else {
        return Ok(vec![ignore(window_id, IgnoreReason::NoPendingCapture)]);
    };
    if pending.generation != generation {
        return Ok(vec![ignore(
            window_id,
            IgnoreReason::StaleCaptureGeneration {
                current: Some(pending.generation),
            },
        )]);
    }
    if !pending.capture_requested {
        return Ok(vec![ignore(window_id, IgnoreReason::CaptureNotRequested)]);
    }
    if pending.captured {
        return Ok(vec![ignore(
            window_id,
            IgnoreReason::CaptureAlreadyCompleted,
        )]);
    }
    if let Some(reason) = pending.force_flush {
        state.pending = None;
        return Ok(vec![WindowLifecycleDirective::Flush {
            window_id,
            generation: Some(generation),
            timeout: policy.flush_timeout(),
            reason,
        }]);
    }

    let due_at = at.checked_add(policy.persistence_debounce())?;
    pending.captured = true;
    pending.flush_due_at = Some(due_at);
    Ok(vec![WindowLifecycleDirective::ScheduleFlush {
        window_id,
        generation,
        due_at,
    }])
}

pub(super) fn handle_flush_deadline(
    policy: WindowLifecyclePolicy,
    state: &mut WindowState,
    at: MonotonicMillis,
    window_id: WindowId,
    generation: CaptureGeneration,
) -> Vec<WindowLifecycleDirective> {
    let Some(pending) = state.pending.as_ref() else {
        return vec![ignore(window_id, IgnoreReason::NoPendingCapture)];
    };
    if pending.generation != generation {
        return vec![ignore(
            window_id,
            IgnoreReason::StaleCaptureGeneration {
                current: Some(pending.generation),
            },
        )];
    }
    let Some(due_at) = pending.flush_due_at else {
        return vec![ignore(window_id, IgnoreReason::CaptureNotRequested)];
    };
    if at < due_at {
        return vec![ignore(window_id, IgnoreReason::EarlyDeadline { due_at })];
    }
    state.pending = None;
    vec![WindowLifecycleDirective::Flush {
        window_id,
        generation: Some(generation),
        timeout: policy.flush_timeout(),
        reason: FlushReason::Debounce,
    }]
}

pub(super) fn handle_close(
    policy: WindowLifecyclePolicy,
    state: &mut WindowState,
    at: MonotonicMillis,
    window_id: WindowId,
) -> Vec<WindowLifecycleDirective> {
    expire_apply(state, at);

    if let Some(apply) = state.apply.as_mut()
        && let Some(index) = apply
            .effects
            .iter()
            .position(|effect| *effect == ExpectedEffect::Close)
    {
        apply.effects.remove(index);
        return vec![ignore(
            window_id,
            IgnoreReason::ProgrammaticClose {
                generation: apply.generation,
            },
        )];
    }

    let mut directives = force_flush(
        policy,
        state,
        window_id.clone(),
        FlushReason::UserClose,
        CaptureReason::UserClose,
    );
    directives.push(WindowLifecycleDirective::UserClose { window_id });
    directives
}

pub(super) fn force_flush(
    policy: WindowLifecyclePolicy,
    state: &mut WindowState,
    window_id: WindowId,
    reason: FlushReason,
    capture_reason: CaptureReason,
) -> Vec<WindowLifecycleDirective> {
    let Some(pending) = state.pending.as_mut() else {
        return vec![WindowLifecycleDirective::Flush {
            window_id,
            generation: None,
            timeout: policy.flush_timeout(),
            reason,
        }];
    };
    if pending.captured {
        let generation = pending.generation;
        state.pending = None;
        return vec![WindowLifecycleDirective::Flush {
            window_id,
            generation: Some(generation),
            timeout: policy.flush_timeout(),
            reason,
        }];
    }
    pending.force_flush = Some(reason);
    if pending.capture_requested {
        vec![ignore(window_id, IgnoreReason::CaptureAlreadyRequested)]
    } else {
        pending.capture_requested = true;
        vec![WindowLifecycleDirective::CaptureNow {
            window_id,
            generation: pending.generation,
            reason: capture_reason,
        }]
    }
}
