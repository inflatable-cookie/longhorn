use longhorn_core::ScaleFactor;
use longhorn_windowing::{
    ApplyRegistrationOutcome, IgnoreReason, WindowLifecycleCoordinator, WindowLifecycleDirective,
    WindowLifecycleEvent, WindowLifecyclePolicy, WindowOperation, WindowOperationKind,
};

use super::support::{
    at, coordinator, duration, generation, id, move_to, moved, only, resize_to, resized,
};

#[test]
fn exact_apply_effects_are_attributed_but_mismatches_are_user_input() {
    // Placement is two operations now, so a caller that applies both registers
    // both into one generation. Each native event is attributed to the axis
    // that asked for it rather than to a compound neither operation names.
    let mut coordinator = coordinator();
    assert_eq!(
        coordinator
            .register_apply(at(100), generation(7), &move_to(40, 50))
            .unwrap(),
        ApplyRegistrationOutcome::Registered
    );
    assert_eq!(
        coordinator
            .register_apply(at(100), generation(7), &resize_to(800, 600))
            .unwrap(),
        ApplyRegistrationOutcome::Extended
    );

    for (time, event, expected) in [
        (110, moved(40, 50), WindowOperationKind::Move),
        (120, resized(800, 600), WindowOperationKind::Resize),
        (
            130,
            WindowLifecycleEvent::ScaleChanged {
                window_id: id(),
                scale: ScaleFactor::from_thousandths(2_000).unwrap(),
            },
            // A scale change follows whichever axis moved first; both
            // operations expect one, and the earlier expectation matches.
            WindowOperationKind::Move,
        ),
    ] {
        assert_eq!(
            only(coordinator.handle(at(time), event).unwrap()),
            WindowLifecycleDirective::Ignore {
                window_id: id(),
                reason: IgnoreReason::ProgrammaticApply {
                    generation: generation(7),
                    operation: expected,
                },
            }
        );
    }

    assert!(matches!(
        only(coordinator.handle(at(140), moved(41, 50)).unwrap()),
        WindowLifecycleDirective::ScheduleCapture {
            window_id,
            generation,
            due_at,
        } if window_id == id() && generation.get() == 1 && due_at == at(440)
    ));
}

#[test]
fn user_precedence_outlives_and_outranks_later_apply_evidence() {
    let policy = WindowLifecyclePolicy::new(
        duration(10_000),
        duration(5_000),
        duration(300),
        duration(250),
        duration(1_000),
    );
    let mut coordinator = WindowLifecycleCoordinator::new(policy);
    coordinator
        .register_apply(at(0), generation(1), &move_to(10, 20))
        .unwrap();
    coordinator.handle(at(100), moved(11, 20)).unwrap();
    coordinator
        .register_apply(at(200), generation(2), &move_to(10, 20))
        .unwrap();

    assert!(matches!(
        only(coordinator.handle(at(300), moved(10, 20)).unwrap()),
        WindowLifecycleDirective::ScheduleCapture {
            generation,
            due_at,
            ..
        } if generation.get() == 2 && due_at == at(600)
    ));
}

#[test]
fn nucleus_and_soundcheck_can_select_no_suppression_restore_policy() {
    let policy = WindowLifecyclePolicy::new(
        duration(0),
        duration(0),
        duration(0),
        duration(0),
        duration(1),
    );
    let mut coordinator = WindowLifecycleCoordinator::new(policy);
    coordinator
        .register_apply(at(10), generation(1), &move_to(40, 50))
        .unwrap();

    assert!(matches!(
        only(coordinator.handle(at(10), moved(40, 50)).unwrap()),
        WindowLifecycleDirective::ScheduleCapture { due_at, .. } if due_at == at(10)
    ));
}

#[test]
fn one_apply_generation_accumulates_transition_evidence() {
    let mut coordinator = coordinator();
    coordinator
        .register_apply(at(10), generation(4), &move_to(40, 50))
        .unwrap();
    assert_eq!(
        coordinator
            .register_apply(
                at(11),
                generation(4),
                &WindowOperation::Maximize {
                    window_id: id(),
                    transport_handle: None,
                },
            )
            .unwrap(),
        ApplyRegistrationOutcome::Extended
    );

    assert!(matches!(
        only(coordinator.handle(at(12), resized(1_920, 1_080)).unwrap()),
        WindowLifecycleDirective::Ignore {
            reason: IgnoreReason::ProgrammaticApply {
                generation: apply_generation,
                operation: WindowOperationKind::Maximize,
            },
            ..
        } if apply_generation == generation(4)
    ));
}

#[test]
fn stale_apply_generations_and_timestamps_do_not_replace_current_evidence() {
    let mut coordinator = coordinator();
    coordinator
        .register_apply(at(100), generation(5), &move_to(40, 50))
        .unwrap();

    assert_eq!(
        coordinator
            .register_apply(at(101), generation(4), &move_to(1, 2))
            .unwrap(),
        ApplyRegistrationOutcome::StaleGeneration {
            current: generation(5),
        }
    );
    assert_eq!(
        coordinator
            .register_apply(at(99), generation(6), &move_to(1, 2))
            .unwrap(),
        ApplyRegistrationOutcome::StaleTimestamp { latest: at(100) }
    );
}
