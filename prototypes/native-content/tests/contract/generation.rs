use longhorn_native_content_prototype::{
    AttachGeneration, AttachmentLifecycle, CoordinationError, DesiredPresence, DesiredVisibility,
    EffectiveFocus, EffectiveVisibility, FocusIntent, InputRoutingMode, NativeContentMechanism,
    NativeContentRevision, ObservationUpdate, ObservedGeometry, ObservedReadiness,
};

use super::support::{attached_observation, coordinator, desired_update, viewport};

#[test]
fn stale_and_future_observations_leave_exact_state_unchanged() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                2,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Request,
                InputRoutingMode::NativeDirect,
                2000,
                viewport(),
            ),
        )
        .unwrap();
    let before = coordinator.clone();

    let stale = coordinator.admit_observation(
        NativeContentRevision::INITIAL,
        attached_observation(
            NativeContentMechanism::ChildView,
            1,
            InputRoutingMode::NativeDirect,
        ),
    );
    assert_eq!(
        stale,
        Err(CoordinationError::StaleGeneration {
            current: AttachGeneration::new(2),
            supplied: AttachGeneration::new(1),
        })
    );
    assert_eq!(coordinator, before);

    let future = coordinator.admit_observation(
        NativeContentRevision::INITIAL,
        attached_observation(
            NativeContentMechanism::ChildView,
            3,
            InputRoutingMode::NativeDirect,
        ),
    );
    assert_eq!(
        future,
        Err(CoordinationError::FutureGeneration {
            current: AttachGeneration::new(2),
            supplied: AttachGeneration::new(3),
        })
    );
    assert_eq!(coordinator, before);
}

#[test]
fn stale_desired_revision_and_generation_gap_are_atomic() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    let first = desired_update(
        1,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Unchanged,
        InputRoutingMode::NativeDirect,
        2000,
        viewport(),
    );
    coordinator
        .update_desired(NativeContentRevision::INITIAL, first)
        .unwrap();
    let before = coordinator.clone();

    let stale = coordinator.update_desired(
        NativeContentRevision::INITIAL,
        desired_update(
            1,
            DesiredPresence::Absent,
            DesiredVisibility::Visible,
            FocusIntent::Unchanged,
            InputRoutingMode::Disabled,
            2000,
            viewport(),
        ),
    );
    assert!(matches!(
        stale,
        Err(CoordinationError::StaleRevision { .. })
    ));
    assert_eq!(coordinator, before);

    let gap = coordinator.update_desired(
        NativeContentRevision::new(1),
        desired_update(
            3,
            DesiredPresence::Present,
            DesiredVisibility::Visible,
            FocusIntent::Unchanged,
            InputRoutingMode::NativeDirect,
            2000,
            viewport(),
        ),
    );
    assert!(matches!(gap, Err(CoordinationError::GenerationGap { .. })));
    assert_eq!(coordinator, before);
}

#[test]
fn illegal_lifecycle_and_unobservable_platform_claims_reject() {
    let mut backing = coordinator(NativeContentMechanism::BackingSurface);
    let before = backing.clone();
    let false_visibility = ObservationUpdate::new(
        AttachGeneration::INITIAL,
        AttachmentLifecycle::Attached,
        ObservedReadiness::Ready,
        EffectiveVisibility::Visible,
        EffectiveFocus::Unknown,
        ObservedGeometry::Unknown,
        Some(InputRoutingMode::RendererForwarded),
    );
    assert_eq!(
        backing.admit_observation(NativeContentRevision::INITIAL, false_visibility),
        Err(CoordinationError::UnsupportedVisibilityObservation)
    );
    assert_eq!(backing, before);

    let mut child = coordinator(NativeContentMechanism::ChildView);
    child
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(
                NativeContentMechanism::ChildView,
                1,
                InputRoutingMode::NativeDirect,
            ),
        )
        .unwrap();
    let before = child.clone();
    let illegal = ObservationUpdate::new(
        AttachGeneration::INITIAL,
        AttachmentLifecycle::Attaching,
        ObservedReadiness::NotReady,
        EffectiveVisibility::Unknown,
        EffectiveFocus::Unknown,
        ObservedGeometry::Unknown,
        None,
    );
    assert!(matches!(
        child.admit_observation(NativeContentRevision::new(1), illegal),
        Err(CoordinationError::IllegalLifecycleTransition { .. })
    ));
    assert_eq!(child, before);
}

#[test]
fn detach_is_explicit_and_converges_idempotently() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(
                NativeContentMechanism::ChildView,
                1,
                InputRoutingMode::NativeDirect,
            ),
        )
        .unwrap();
    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                1,
                DesiredPresence::Absent,
                DesiredVisibility::Hidden {
                    reason: "shutdown".parse().unwrap(),
                },
                FocusIntent::ReleaseIfOwned,
                InputRoutingMode::Disabled,
                2000,
                viewport(),
            ),
        )
        .unwrap();
    let plan = coordinator.plan().unwrap();
    assert_eq!(plan.operations().len(), 1);
    assert!(matches!(
        plan.operations()[0].operation(),
        longhorn_native_content_prototype::NativeContentOperation::Detach { .. }
    ));

    coordinator
        .admit_observation(
            NativeContentRevision::new(1),
            ObservationUpdate::new(
                AttachGeneration::INITIAL,
                AttachmentLifecycle::Detaching,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Hidden,
                EffectiveFocus::Unfocused,
                ObservedGeometry::Unknown,
                None,
            ),
        )
        .unwrap();
    coordinator
        .admit_observation(
            NativeContentRevision::new(2),
            ObservationUpdate::new(
                AttachGeneration::INITIAL,
                AttachmentLifecycle::Absent,
                ObservedReadiness::Unknown,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::Unknown,
                None,
            ),
        )
        .unwrap();
    assert!(coordinator.plan().unwrap().is_empty());
}

#[test]
fn failed_generation_requires_exact_next_generation() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            ObservationUpdate::new(
                AttachGeneration::INITIAL,
                AttachmentLifecycle::Failed,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::Unknown,
                None,
            ),
        )
        .unwrap();
    assert_eq!(
        coordinator.plan(),
        Err(CoordinationError::TerminalGeneration(
            AttachGeneration::INITIAL
        ))
    );

    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                2,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Request,
                InputRoutingMode::NativeDirect,
                2000,
                viewport(),
            ),
        )
        .unwrap();
    assert!(matches!(
        coordinator.plan().unwrap().operations()[0].operation(),
        longhorn_native_content_prototype::NativeContentOperation::Attach { .. }
    ));
}

#[test]
fn live_native_content_blocks_generation_advance() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(
                NativeContentMechanism::ChildView,
                1,
                InputRoutingMode::NativeDirect,
            ),
        )
        .unwrap();
    let before = coordinator.clone();
    assert_eq!(
        coordinator.update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                2,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Request,
                InputRoutingMode::NativeDirect,
                2000,
                viewport(),
            ),
        ),
        Err(CoordinationError::GenerationStillAttached(
            AttachmentLifecycle::Attached
        ))
    );
    assert_eq!(coordinator, before);
}
