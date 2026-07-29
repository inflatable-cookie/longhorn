use longhorn_transfer::{
    ClientEpoch, DropZone, LeaseGeneration, LeasePublication, TransferCapability, TransferClientId,
    TransferCoordinator, TransferDuration, TransferErrorCode, TransferLimits,
};

use super::support::{FakeClock, bind, client, limits, panel_zone, publication, rect, window};

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

    let target = panel_zone("zone:template", rect(10, 10, 10, 10), None)
        .target()
        .clone();
    let invalid_replacements = vec![
        vec![
            panel_zone("zone:duplicate", rect(10, 10, 20, 20), None),
            panel_zone("zone:duplicate", rect(40, 10, 20, 20), None),
        ],
        vec![panel_zone("zone:outside", rect(390, 290, 20, 20), None)],
        vec![panel_zone("zone:empty", rect(10, 10, 0, 20), None)],
        vec![panel_zone("zone:overflow", rect(i32::MAX, 0, 1, 1), None)],
        vec![panel_zone("zone:insertion", rect(10, 10, 20, 20), Some(6))],
        vec![DropZone::new(
            longhorn_transfer::DropZoneId::new("zone:capability").unwrap(),
            rect(10, 10, 20, 20),
            None,
            TransferCapability::MoveSurface,
            target,
        )],
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
}
