use longhorn_core::ScreenPoint;
use longhorn_transfer::{
    ClientEpoch, DropZoneId, LeaseGeneration, LeasePublication, LiveTransferWindow, TargetSelector,
    TransferClientId, TransferCoordinator, TransferDuration, TransferErrorCode, TransferLimits,
    TransferSessionRequest,
};

use super::support::{
    FakeClock, SequenceAllocator, bind, client, limits, panel_source, panel_zone, publication,
    rect, window,
};

#[test]
fn invalid_complete_replacements_preserve_the_current_generation() {
    let clock = FakeClock::new(0);
    let mut coordinator = TransferCoordinator::new(limits(4, 2, 2, 5));
    bind(
        &mut coordinator,
        &clock,
        "window:target",
        "client:target",
        1,
    );
    let bounds = rect(0, 0, 400, 300);
    coordinator
        .publish_lease(
            &clock,
            publication(
                "window:target",
                "client:target",
                1,
                1,
                bounds,
                vec![panel_zone("zone:one", rect(10, 10, 100, 80), Some(1))],
            ),
        )
        .unwrap();

    let invalid_replacements = vec![
        vec![
            panel_zone("zone:duplicate", rect(10, 10, 20, 20), None),
            panel_zone("zone:duplicate", rect(40, 10, 20, 20), None),
        ],
        vec![panel_zone("zone:outside", rect(390, 290, 20, 20), None)],
        vec![panel_zone("zone:empty", rect(10, 10, 0, 20), None)],
        vec![panel_zone("zone:overflow", rect(i32::MAX, 0, 1, 1), None)],
        vec![panel_zone("zone:insertion", rect(10, 10, 20, 20), Some(6))],
        vec![
            panel_zone("zone:a", rect(10, 10, 20, 20), None),
            panel_zone("zone:b", rect(40, 10, 20, 20), None),
            panel_zone("zone:c", rect(70, 10, 20, 20), None),
        ],
    ];

    for zones in invalid_replacements {
        let error = coordinator
            .publish_lease(
                &clock,
                publication("window:target", "client:target", 1, 2, bounds, zones),
            )
            .unwrap_err();
        assert_eq!(error.code(), TransferErrorCode::InvalidLease);
        assert_eq!(
            coordinator.current_lease_generation(&window("window:target")),
            Some(LeaseGeneration::new(1))
        );
    }
}

#[test]
fn replacement_generation_and_client_epoch_transitions_are_exact() {
    let clock = FakeClock::new(0);
    let mut coordinator = TransferCoordinator::new(limits(4, 2, 2, 5));
    bind(
        &mut coordinator,
        &clock,
        "window:target",
        "client:target",
        1,
    );
    let bounds = rect(0, 0, 400, 300);
    let first = publication(
        "window:target",
        "client:target",
        1,
        4,
        bounds,
        vec![panel_zone("zone:one", rect(10, 10, 20, 20), None)],
    );
    coordinator.publish_lease(&clock, first.clone()).unwrap();
    assert_eq!(
        coordinator.publish_lease(&clock, first).unwrap_err().code(),
        TransferErrorCode::StaleLeaseGeneration
    );

    let replacement = coordinator
        .publish_lease(
            &clock,
            publication("window:target", "client:target", 1, 5, bounds, Vec::new()),
        )
        .unwrap();
    assert_eq!(replacement.zone_count(), 0);
    assert_eq!(replacement.generation(), LeaseGeneration::new(5));

    assert_eq!(
        coordinator
            .bind_client_epoch(
                &clock,
                window("window:target"),
                client("client:target"),
                ClientEpoch::new(2),
            )
            .unwrap(),
        longhorn_transfer::ClientEpochBindingStatus::Advanced
    );
    assert_eq!(
        coordinator.current_lease_generation(&window("window:target")),
        None
    );
    assert_eq!(
        coordinator
            .publish_lease(
                &clock,
                publication("window:target", "client:target", 1, 6, bounds, Vec::new(),),
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::StaleClientEpoch
    );
    assert_eq!(
        coordinator
            .publish_lease(
                &clock,
                publication("window:target", "client:target", 2, 0, bounds, Vec::new(),),
            )
            .unwrap()
            .generation(),
        LeaseGeneration::new(0)
    );
}

#[test]
fn client_capacity_and_lease_lifetime_fail_before_publication() {
    let clock = FakeClock::new(0);
    let mut coordinator = TransferCoordinator::new(limits(2, 1, 2, 5));
    bind(&mut coordinator, &clock, "window:one", "client:one", 1);
    assert_eq!(
        coordinator
            .bind_client_epoch(
                &clock,
                window("window:two"),
                client("client:two"),
                ClientEpoch::new(1),
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::ClientWindowCapacity
    );

    let invalid = LeasePublication::new(
        window("window:one"),
        TransferClientId::new("client:one").unwrap(),
        ClientEpoch::new(1),
        LeaseGeneration::new(1),
        TransferDuration::new(51),
        rect(0, 0, 100, 100),
        Vec::new(),
    );
    assert_eq!(
        coordinator
            .publish_lease(&clock, invalid)
            .unwrap_err()
            .code(),
        TransferErrorCode::InvalidLifetime
    );
    assert_eq!(coordinator.lease_count(), 0);

    let mut lease_limited = TransferCoordinator::new(
        TransferLimits::new(
            2,
            2,
            1,
            2,
            5,
            TransferDuration::new(20),
            TransferDuration::new(40),
        )
        .unwrap(),
    );
    bind(&mut lease_limited, &clock, "window:one", "client:one", 1);
    bind(&mut lease_limited, &clock, "window:two", "client:two", 1);
    lease_limited
        .publish_lease(
            &clock,
            publication(
                "window:one",
                "client:one",
                1,
                1,
                rect(0, 0, 100, 100),
                Vec::new(),
            ),
        )
        .unwrap();
    assert_eq!(
        lease_limited
            .publish_lease(
                &clock,
                publication(
                    "window:two",
                    "client:two",
                    1,
                    1,
                    rect(200, 0, 100, 100),
                    Vec::new(),
                ),
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::LeaseCapacity
    );
    assert_eq!(lease_limited.lease_count(), 1);

    clock.set(40);
    let receipt = lease_limited
        .publish_lease(
            &clock,
            publication(
                "window:two",
                "client:two",
                1,
                1,
                rect(200, 0, 100, 100),
                Vec::new(),
            ),
        )
        .unwrap();
    assert_eq!(receipt.generation(), LeaseGeneration::new(1));
    assert_eq!(lease_limited.lease_count(), 1);
    assert_eq!(
        lease_limited.current_lease_generation(&window("window:one")),
        None
    );
    assert_eq!(
        lease_limited.current_lease_generation(&window("window:two")),
        Some(LeaseGeneration::new(1))
    );
}

#[test]
fn same_epoch_generation_monotonicity_persists_across_expiry_and_resets_on_epoch_change() {
    let clock = FakeClock::new(0);
    let mut coordinator = TransferCoordinator::new(limits(4, 2, 2, 5));
    bind(
        &mut coordinator,
        &clock,
        "window:target",
        "client:target",
        1,
    );
    let bounds = rect(0, 0, 400, 300);
    coordinator
        .publish_lease(
            &clock,
            publication(
                "window:target",
                "client:target",
                1,
                4,
                bounds,
                vec![panel_zone("zone:one", rect(10, 10, 40, 40), None)],
            ),
        )
        .unwrap();

    // Advance to the exact expiry boundary (publication helper uses lifetime 30).
    clock.set(30);
    for stale_generation in [4, 3] {
        let error = coordinator
            .publish_lease(
                &clock,
                publication(
                    "window:target",
                    "client:target",
                    1,
                    stale_generation,
                    bounds,
                    vec![panel_zone("zone:one", rect(10, 10, 40, 40), None)],
                ),
            )
            .unwrap_err();
        assert_eq!(error.code(), TransferErrorCode::StaleLeaseGeneration);
    }

    let advanced = coordinator
        .publish_lease(
            &clock,
            publication("window:target", "client:target", 1, 5, bounds, Vec::new()),
        )
        .unwrap();
    assert_eq!(advanced.generation(), LeaseGeneration::new(5));

    coordinator
        .bind_client_epoch(
            &clock,
            window("window:target"),
            client("client:target"),
            ClientEpoch::new(2),
        )
        .unwrap();
    let reset = coordinator
        .publish_lease(
            &clock,
            publication("window:target", "client:target", 2, 0, bounds, Vec::new()),
        )
        .unwrap();
    assert_eq!(reset.generation(), LeaseGeneration::new(0));
}

#[test]
fn terminal_attempt_reports_no_target_after_unrelated_publication_reclaims_expired_lease() {
    let clock = FakeClock::new(0);
    let mut allocator = SequenceAllocator::new([[1; 16], [2; 16], [3; 16]]);
    let mut coordinator = TransferCoordinator::new(
        TransferLimits::new(
            4,
            4,
            2,
            2,
            5,
            TransferDuration::new(60),
            TransferDuration::new(40),
        )
        .unwrap(),
    );
    bind(
        &mut coordinator,
        &clock,
        "window:source",
        "client:source",
        1,
    );
    bind(
        &mut coordinator,
        &clock,
        "window:target",
        "client:target",
        1,
    );
    bind(&mut coordinator, &clock, "window:other", "client:other", 1);

    let target_bounds = rect(0, 0, 400, 300);
    coordinator
        .publish_lease(
            &clock,
            publication(
                "window:target",
                "client:target",
                1,
                1,
                target_bounds,
                vec![panel_zone("zone:one", rect(10, 10, 80, 80), None)],
            ),
        )
        .unwrap();

    let session_before = coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(
                panel_source("window:source", "client:source", 1),
                TransferDuration::new(50),
            ),
        )
        .unwrap()
        .payload()
        .session_id();
    let session_after_zone = coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(
                panel_source("window:source", "client:source", 1),
                TransferDuration::new(50),
            ),
        )
        .unwrap()
        .payload()
        .session_id();
    let session_after_point = coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(
                panel_source("window:source", "client:source", 1),
                TransferDuration::new(50),
            ),
        )
        .unwrap()
        .payload()
        .session_id();

    let live_windows = [LiveTransferWindow::new(
        window("window:target"),
        target_bounds,
    )];

    // At expiry, before reclamation, resolution still reports LeaseExpired.
    clock.set(30);
    let before = coordinator
        .attempt_target_resolution(
            &clock,
            session_before,
            TargetSelector::ExplicitZone(DropZoneId::new("zone:one").unwrap()),
            &live_windows,
        )
        .unwrap_err();
    assert_eq!(before.code(), TransferErrorCode::LeaseExpired);
    assert!(before.session_consumed());

    // An unrelated lease publication sweeps expired leases.
    coordinator
        .publish_lease(
            &clock,
            publication(
                "window:other",
                "client:other",
                1,
                1,
                rect(500, 0, 200, 200),
                Vec::new(),
            ),
        )
        .unwrap();

    // After reclamation, no expired lease record is retained, so both paths report NoTarget.
    let after_zone = coordinator
        .attempt_target_resolution(
            &clock,
            session_after_zone,
            TargetSelector::ExplicitZone(DropZoneId::new("zone:one").unwrap()),
            &live_windows,
        )
        .unwrap_err();
    assert_eq!(after_zone.code(), TransferErrorCode::NoTarget);
    assert!(after_zone.session_consumed());

    let after_point = coordinator
        .attempt_target_resolution(
            &clock,
            session_after_point,
            TargetSelector::ScreenPoint(ScreenPoint::new(20, 20)),
            &live_windows,
        )
        .unwrap_err();
    assert_eq!(after_point.code(), TransferErrorCode::NoTarget);
    assert!(after_point.session_consumed());
}
