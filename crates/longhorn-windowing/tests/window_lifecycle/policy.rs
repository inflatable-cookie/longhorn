use longhorn_windowing::{
    WindowLifecycleCoordinator, WindowLifecycleError, WindowLifecycleEvent,
    WindowLifecycleEventKind, WindowLifecyclePolicy,
};

use super::support::{at, donor_policy, duration, id, move_resize, moved};

#[test]
fn recommended_policy_is_explicit_and_stable() {
    let policy = WindowLifecyclePolicy::recommended();

    assert_eq!(policy.programmatic_attribution(), duration(400));
    assert_eq!(policy.user_precedence(), duration(400));
    assert_eq!(policy.settle_delay(), duration(200));
    assert_eq!(policy.persistence_debounce(), duration(500));
    assert_eq!(policy.flush_timeout(), duration(2_000));
}

#[test]
fn timing_policy_has_no_hidden_defaults_and_round_trips_exactly() {
    let policy = donor_policy();

    assert_eq!(policy.programmatic_attribution(), duration(3_000));
    assert_eq!(policy.user_precedence(), duration(5_000));
    assert_eq!(policy.settle_delay(), duration(300));
    assert_eq!(policy.persistence_debounce(), duration(250));
    assert_eq!(policy.flush_timeout(), duration(1_000));
    assert_eq!(
        serde_json::from_str::<WindowLifecyclePolicy>(&serde_json::to_string(&policy).unwrap())
            .unwrap(),
        policy
    );
}

#[test]
fn event_categories_are_explicit_and_serializable() {
    let event = WindowLifecycleEvent::CloseRequested { window_id: id() };

    assert_eq!(event.kind(), WindowLifecycleEventKind::CloseRequested);
    assert_eq!(event.window_id(), &id());
    assert_eq!(
        serde_json::from_str::<WindowLifecycleEvent>(&serde_json::to_string(&event).unwrap())
            .unwrap(),
        event
    );
}

#[test]
fn deadline_overflow_fails_typed_instead_of_wrapping() {
    let policy = WindowLifecyclePolicy::new(
        duration(1),
        duration(1),
        duration(1),
        duration(1),
        duration(1),
    );
    let mut coordinator = WindowLifecycleCoordinator::new(policy);

    assert_eq!(
        coordinator
            .register_apply(
                at(u64::MAX),
                longhorn_windowing::ApplyGeneration::new(1),
                &move_resize(1, 2, 3, 4),
            )
            .unwrap_err(),
        WindowLifecycleError::DeadlineOverflow {
            at: at(u64::MAX),
            duration: duration(1),
        }
    );
    assert_eq!(
        coordinator.handle(at(u64::MAX), moved(1, 2)).unwrap_err(),
        WindowLifecycleError::DeadlineOverflow {
            at: at(u64::MAX),
            duration: duration(1),
        }
    );
}
