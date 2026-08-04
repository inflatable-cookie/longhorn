use longhorn_core::WindowId;

use super::persistence::{
    force_flush, handle_capture_completed, handle_close, handle_flush_deadline,
};
use super::{
    CaptureGeneration, CaptureReason, FlushReason, GeometryEvent, IgnoreReason, MonotonicMillis,
    PendingCapture, WindowLifecycleDirective, WindowLifecycleError, WindowLifecycleEvent,
    WindowLifecyclePolicy, WindowState,
};
use crate::{ApplyGeneration, WindowOperationKind};

pub(super) fn handle_event(
    policy: WindowLifecyclePolicy,
    state: &mut WindowState,
    at: MonotonicMillis,
    window_id: WindowId,
    event: WindowLifecycleEvent,
) -> Result<Vec<WindowLifecycleDirective>, WindowLifecycleError> {
    match event {
        WindowLifecycleEvent::Moved { outer_origin, .. } => handle_geometry(
            policy,
            state,
            at,
            window_id,
            GeometryEvent::Moved(outer_origin),
        ),
        WindowLifecycleEvent::Resized { inner_size, .. } => handle_geometry(
            policy,
            state,
            at,
            window_id,
            GeometryEvent::Resized(inner_size),
        ),
        WindowLifecycleEvent::ScaleChanged { .. } => {
            handle_geometry(policy, state, at, window_id, GeometryEvent::ScaleChanged)
        }
        WindowLifecycleEvent::Blurred { .. } => {
            begin_immediate_capture(policy, state, at, window_id, CaptureReason::Blur)
        }
        WindowLifecycleEvent::CloseRequested { .. } => {
            Ok(handle_close(policy, state, at, window_id))
        }
        WindowLifecycleEvent::CaptureDeadline { generation, .. } => {
            Ok(handle_capture_deadline(state, at, window_id, generation))
        }
        WindowLifecycleEvent::CaptureCompleted { generation, .. } => {
            handle_capture_completed(policy, state, at, window_id, generation)
        }
        WindowLifecycleEvent::CaptureFailed { generation, .. } => {
            Ok(handle_capture_failed(state, window_id, generation))
        }
        WindowLifecycleEvent::FlushDeadline { generation, .. } => Ok(handle_flush_deadline(
            policy, state, at, window_id, generation,
        )),
        WindowLifecycleEvent::FlushRequested { .. } => Ok(force_flush(
            policy,
            state,
            window_id,
            FlushReason::Explicit,
            CaptureReason::Flush,
        )),
        WindowLifecycleEvent::Destroyed { .. } => unreachable!(),
    }
}

fn handle_capture_failed(
    state: &mut WindowState,
    window_id: WindowId,
    generation: CaptureGeneration,
) -> Vec<WindowLifecycleDirective> {
    let Some(pending) = state.pending.as_mut() else {
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
    pending.capture_requested = false;
    pending.captured = false;
    pending.flush_due_at = None;
    vec![ignore(window_id, IgnoreReason::CaptureFailedReset)]
}

fn handle_geometry(
    policy: WindowLifecyclePolicy,
    state: &mut WindowState,
    at: MonotonicMillis,
    window_id: WindowId,
    event: GeometryEvent,
) -> Result<Vec<WindowLifecycleDirective>, WindowLifecycleError> {
    expire_apply(state, at);
    let user_active = state.user_until.is_some_and(|until| at < until);
    if !user_active && let Some((generation, operation)) = matching_apply(state, at, event) {
        return Ok(vec![ignore(
            window_id,
            IgnoreReason::ProgrammaticApply {
                generation,
                operation,
            },
        )]);
    }

    let user_until = at.checked_add(policy.user_precedence())?;
    let due_at = at.checked_add(policy.settle_delay())?;
    let generation = next_capture_generation(state)?;
    state.user_until = Some(user_until);
    state.pending = Some(PendingCapture {
        generation,
        capture_due_at: due_at,
        capture_requested: false,
        captured: false,
        flush_due_at: None,
        force_flush: None,
    });
    Ok(vec![WindowLifecycleDirective::ScheduleCapture {
        window_id,
        generation,
        due_at,
    }])
}

fn begin_immediate_capture(
    policy: WindowLifecyclePolicy,
    state: &mut WindowState,
    at: MonotonicMillis,
    window_id: WindowId,
    reason: CaptureReason,
) -> Result<Vec<WindowLifecycleDirective>, WindowLifecycleError> {
    let user_until = at.checked_add(policy.user_precedence())?;
    let generation = next_capture_generation(state)?;
    state.user_until = Some(user_until);
    state.pending = Some(PendingCapture {
        generation,
        capture_due_at: at,
        capture_requested: true,
        captured: false,
        flush_due_at: None,
        force_flush: None,
    });
    Ok(vec![WindowLifecycleDirective::CaptureNow {
        window_id,
        generation,
        reason,
    }])
}

fn handle_capture_deadline(
    state: &mut WindowState,
    at: MonotonicMillis,
    window_id: WindowId,
    generation: CaptureGeneration,
) -> Vec<WindowLifecycleDirective> {
    let Some(pending) = state.pending.as_mut() else {
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
    if at < pending.capture_due_at {
        return vec![ignore(
            window_id,
            IgnoreReason::EarlyDeadline {
                due_at: pending.capture_due_at,
            },
        )];
    }
    if pending.capture_requested {
        return vec![ignore(window_id, IgnoreReason::CaptureAlreadyRequested)];
    }
    pending.capture_requested = true;
    vec![WindowLifecycleDirective::CaptureNow {
        window_id,
        generation,
        reason: CaptureReason::Settled,
    }]
}

fn matching_apply(
    state: &WindowState,
    at: MonotonicMillis,
    event: GeometryEvent,
) -> Option<(ApplyGeneration, WindowOperationKind)> {
    let apply = state.apply.as_ref()?;
    if at >= apply.expires_at {
        return None;
    }
    apply.effects.iter().find_map(|effect| {
        if effect.matches_geometry(event) {
            Some((apply.generation, effect.operation()?))
        } else {
            None
        }
    })
}

pub(super) fn expire_apply(state: &mut WindowState, at: MonotonicMillis) {
    if state
        .apply
        .as_ref()
        .is_some_and(|apply| at >= apply.expires_at)
    {
        state.apply = None;
    }
}

fn next_capture_generation(
    state: &mut WindowState,
) -> Result<CaptureGeneration, WindowLifecycleError> {
    let generation = state
        .capture_generation
        .checked_add(1)
        .ok_or(WindowLifecycleError::CaptureGenerationExhausted)?;
    state.capture_generation = generation;
    Ok(CaptureGeneration::new(generation))
}

pub(super) fn ignore(window_id: WindowId, reason: IgnoreReason) -> WindowLifecycleDirective {
    WindowLifecycleDirective::Ignore { window_id, reason }
}
