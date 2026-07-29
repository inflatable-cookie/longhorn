use longhorn_windowing::{
    CaptureReason, FlushReason, IgnoreReason, WindowLifecycleDirective, WindowLifecycleEvent,
};

use super::support::{at, close, coordinator, generation, id, moved};

#[test]
fn current_programmatic_close_is_consumed_but_expired_evidence_is_not() {
    let mut coordinator = coordinator();
    coordinator
        .register_apply(at(100), generation(1), &close())
        .unwrap();
    assert_eq!(
        coordinator
            .handle(
                at(200),
                WindowLifecycleEvent::CloseRequested { window_id: id() },
            )
            .unwrap(),
        vec![WindowLifecycleDirective::Ignore {
            window_id: id(),
            reason: IgnoreReason::ProgrammaticClose {
                generation: generation(1),
            },
        }]
    );

    coordinator
        .register_apply(at(300), generation(2), &close())
        .unwrap();
    assert_eq!(
        coordinator
            .handle(
                at(3_300),
                WindowLifecycleEvent::CloseRequested { window_id: id() },
            )
            .unwrap(),
        vec![
            WindowLifecycleDirective::Flush {
                window_id: id(),
                generation: None,
                timeout: super::support::duration(1_000),
                reason: FlushReason::UserClose,
            },
            WindowLifecycleDirective::UserClose { window_id: id() },
        ]
    );
}

#[test]
fn user_close_captures_pending_geometry_then_flushes_without_mutating_policy() {
    let mut coordinator = coordinator();
    let generation = match &coordinator.handle(at(100), moved(10, 20)).unwrap()[0] {
        WindowLifecycleDirective::ScheduleCapture { generation, .. } => *generation,
        directive => panic!("unexpected directive: {directive:?}"),
    };

    assert_eq!(
        coordinator
            .handle(
                at(200),
                WindowLifecycleEvent::CloseRequested { window_id: id() },
            )
            .unwrap(),
        vec![
            WindowLifecycleDirective::CaptureNow {
                window_id: id(),
                generation,
                reason: CaptureReason::UserClose,
            },
            WindowLifecycleDirective::UserClose { window_id: id() },
        ]
    );
    assert_eq!(
        coordinator
            .handle(
                at(210),
                WindowLifecycleEvent::CaptureCompleted {
                    window_id: id(),
                    generation,
                },
            )
            .unwrap(),
        vec![WindowLifecycleDirective::Flush {
            window_id: id(),
            generation: Some(generation),
            timeout: super::support::duration(1_000),
            reason: FlushReason::UserClose,
        }]
    );
}

#[test]
fn explicit_flush_and_destroy_are_bounded_and_destroy_forgets_state() {
    let mut coordinator = coordinator();
    assert_eq!(
        coordinator
            .handle(
                at(10),
                WindowLifecycleEvent::FlushRequested { window_id: id() },
            )
            .unwrap(),
        vec![WindowLifecycleDirective::Flush {
            window_id: id(),
            generation: None,
            timeout: super::support::duration(1_000),
            reason: FlushReason::Explicit,
        }]
    );
    assert!(coordinator.is_tracking(&id()));

    assert_eq!(
        coordinator
            .handle(at(20), WindowLifecycleEvent::Destroyed { window_id: id() },)
            .unwrap(),
        vec![
            WindowLifecycleDirective::Flush {
                window_id: id(),
                generation: None,
                timeout: super::support::duration(1_000),
                reason: FlushReason::Destroy,
            },
            WindowLifecycleDirective::Forget { window_id: id() },
        ]
    );
    assert!(!coordinator.is_tracking(&id()));
}

#[test]
fn terminal_destroy_forgets_state_despite_reordered_delivery() {
    let mut coordinator = coordinator();
    coordinator.handle(at(100), moved(10, 20)).unwrap();

    assert!(matches!(
        coordinator
            .handle(at(99), WindowLifecycleEvent::Destroyed { window_id: id() },)
            .unwrap()
            .as_slice(),
        [
            WindowLifecycleDirective::Flush {
                reason: FlushReason::Destroy,
                ..
            },
            WindowLifecycleDirective::Forget { .. },
        ]
    ));
    assert!(!coordinator.is_tracking(&id()));
}

#[test]
fn failed_forced_capture_can_retry_on_a_later_flush() {
    let mut coordinator = coordinator();
    let generation = match &coordinator.handle(at(100), moved(10, 20)).unwrap()[0] {
        WindowLifecycleDirective::ScheduleCapture { generation, .. } => *generation,
        directive => panic!("unexpected directive: {directive:?}"),
    };
    coordinator
        .handle(
            at(200),
            WindowLifecycleEvent::FlushRequested { window_id: id() },
        )
        .unwrap();
    assert!(matches!(
        coordinator
            .handle(
                at(210),
                WindowLifecycleEvent::CaptureFailed {
                    window_id: id(),
                    generation,
                },
            )
            .unwrap()
            .as_slice(),
        [WindowLifecycleDirective::Ignore {
            reason: IgnoreReason::CaptureFailedReset,
            ..
        }]
    ));
    assert!(matches!(
        coordinator
            .handle(
                at(220),
                WindowLifecycleEvent::FlushRequested {
                    window_id: id(),
                },
            )
            .unwrap()
            .as_slice(),
        [WindowLifecycleDirective::CaptureNow {
            generation: retried,
            reason: CaptureReason::Flush,
            ..
        }] if *retried == generation
    ));
}
