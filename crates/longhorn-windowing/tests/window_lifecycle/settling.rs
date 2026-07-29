use longhorn_windowing::{
    CaptureReason, FlushReason, IgnoreReason, WindowLifecycleDirective, WindowLifecycleEvent,
};

use super::support::{at, coordinator, id, moved, only};

#[test]
fn repeated_user_geometry_coalesces_by_generation_and_deadline() {
    let mut coordinator = coordinator();
    let first = only(coordinator.handle(at(100), moved(10, 20)).unwrap());
    let second = only(coordinator.handle(at(200), moved(11, 20)).unwrap());
    let (first_generation, second_generation) = match (&first, &second) {
        (
            WindowLifecycleDirective::ScheduleCapture {
                generation: first,
                due_at: first_due,
                ..
            },
            WindowLifecycleDirective::ScheduleCapture {
                generation: second,
                due_at: second_due,
                ..
            },
        ) => {
            assert_eq!(*first_due, at(400));
            assert_eq!(*second_due, at(500));
            (*first, *second)
        }
        value => panic!("unexpected directives: {value:?}"),
    };

    assert_eq!(
        only(
            coordinator
                .handle(
                    at(400),
                    WindowLifecycleEvent::CaptureDeadline {
                        window_id: id(),
                        generation: first_generation,
                    },
                )
                .unwrap(),
        ),
        WindowLifecycleDirective::Ignore {
            window_id: id(),
            reason: IgnoreReason::StaleCaptureGeneration {
                current: Some(second_generation),
            },
        }
    );
    assert!(matches!(
        only(
            coordinator
                .handle(
                    at(499),
                    WindowLifecycleEvent::CaptureDeadline {
                        window_id: id(),
                        generation: second_generation,
                    },
                )
                .unwrap(),
        ),
        WindowLifecycleDirective::Ignore {
            reason: IgnoreReason::EarlyDeadline { due_at },
            ..
        } if due_at == at(500)
    ));
    assert_eq!(
        only(
            coordinator
                .handle(
                    at(500),
                    WindowLifecycleEvent::CaptureDeadline {
                        window_id: id(),
                        generation: second_generation,
                    },
                )
                .unwrap(),
        ),
        WindowLifecycleDirective::CaptureNow {
            window_id: id(),
            generation: second_generation,
            reason: CaptureReason::Settled,
        }
    );
}

#[test]
fn completed_capture_debounces_then_emits_bounded_flush() {
    let mut coordinator = coordinator();
    let generation = match only(coordinator.handle(at(100), moved(10, 20)).unwrap()) {
        WindowLifecycleDirective::ScheduleCapture { generation, .. } => generation,
        directive => panic!("unexpected directive: {directive:?}"),
    };
    coordinator
        .handle(
            at(400),
            WindowLifecycleEvent::CaptureDeadline {
                window_id: id(),
                generation,
            },
        )
        .unwrap();

    assert_eq!(
        only(
            coordinator
                .handle(
                    at(410),
                    WindowLifecycleEvent::CaptureCompleted {
                        window_id: id(),
                        generation,
                    },
                )
                .unwrap(),
        ),
        WindowLifecycleDirective::ScheduleFlush {
            window_id: id(),
            generation,
            due_at: at(660),
        }
    );
    assert_eq!(
        only(
            coordinator
                .handle(
                    at(411),
                    WindowLifecycleEvent::CaptureCompleted {
                        window_id: id(),
                        generation,
                    },
                )
                .unwrap(),
        ),
        WindowLifecycleDirective::Ignore {
            window_id: id(),
            reason: IgnoreReason::CaptureAlreadyCompleted,
        }
    );
    assert_eq!(
        only(
            coordinator
                .handle(
                    at(660),
                    WindowLifecycleEvent::FlushDeadline {
                        window_id: id(),
                        generation,
                    },
                )
                .unwrap(),
        ),
        WindowLifecycleDirective::Flush {
            window_id: id(),
            generation: Some(generation),
            timeout: super::support::duration(1_000),
            reason: FlushReason::Debounce,
        }
    );
}

#[test]
fn blur_requests_immediate_complete_capture() {
    let mut coordinator = coordinator();

    assert!(matches!(
        only(
            coordinator
                .handle(at(50), WindowLifecycleEvent::Blurred { window_id: id() },)
                .unwrap(),
        ),
        WindowLifecycleDirective::CaptureNow {
            reason: CaptureReason::Blur,
            ..
        }
    ));
}

#[test]
fn reordered_events_are_ignored_without_changing_pending_work() {
    let mut coordinator = coordinator();
    coordinator.handle(at(100), moved(10, 20)).unwrap();

    assert_eq!(
        only(coordinator.handle(at(99), moved(11, 20)).unwrap()),
        WindowLifecycleDirective::Ignore {
            window_id: id(),
            reason: IgnoreReason::StaleTimestamp { latest: at(100) },
        }
    );
}
