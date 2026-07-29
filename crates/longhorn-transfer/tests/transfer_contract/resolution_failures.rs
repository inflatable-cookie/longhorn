use longhorn_core::ScreenPoint;
use longhorn_transfer::{
    DropZoneId, TargetSelector, TransferDuration, TransferErrorCode, TransferSessionRequest,
};

use super::support::{
    FakeClock, SequenceAllocator, bind, coordinator, live, panel_source, panel_zone, publication,
    rect, surface_source, surface_zone, window,
};

#[test]
fn duplicate_ids_expiry_stale_geometry_and_target_destroy_are_typed() {
    let clock = FakeClock::new(0);
    let mut coordinator = coordinator();
    bind(
        &mut coordinator,
        &clock,
        "window:source",
        "client:source",
        1,
    );
    bind(&mut coordinator, &clock, "window:a", "client:a", 1);
    bind(&mut coordinator, &clock, "window:b", "client:b", 1);
    let a_bounds = rect(0, 0, 200, 200);
    let b_bounds = rect(300, 0, 200, 200);
    for (window_id, client_id, bounds) in [
        ("window:a", "client:a", a_bounds),
        ("window:b", "client:b", b_bounds),
    ] {
        coordinator
            .publish_lease(
                &clock,
                publication(
                    window_id,
                    client_id,
                    1,
                    1,
                    bounds,
                    vec![panel_zone(
                        "zone:shared",
                        rect(bounds.origin().x().get() + 10, 10, 50, 50),
                        None,
                    )],
                ),
            )
            .unwrap();
    }
    let mut allocator = SequenceAllocator::new([[6; 16], [7; 16], [8; 16], [9; 16]]);
    let duplicate = create_panel_session(&mut coordinator, &clock, &mut allocator);
    assert_eq!(
        coordinator
            .attempt_target_resolution(
                &clock,
                duplicate,
                TargetSelector::ExplicitZone(DropZoneId::new("zone:shared").unwrap()),
                &[live("window:a", a_bounds), live("window:b", b_bounds)],
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::AmbiguousZone
    );

    let stale = create_panel_session(&mut coordinator, &clock, &mut allocator);
    assert_eq!(
        coordinator
            .attempt_target_resolution(
                &clock,
                stale,
                TargetSelector::ScreenPoint(ScreenPoint::new(20, 20)),
                &[live("window:a", rect(0, 0, 210, 200))],
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::StaleWindowGeometry
    );

    coordinator.destroy_window(&window("window:b"));
    let missing = create_panel_session(&mut coordinator, &clock, &mut allocator);
    assert_eq!(
        coordinator
            .attempt_target_resolution(
                &clock,
                missing,
                TargetSelector::ScreenPoint(ScreenPoint::new(320, 20)),
                &[live("window:b", b_bounds)],
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::NoTarget
    );

    clock.set(30);
    let expired = create_panel_session(&mut coordinator, &clock, &mut allocator);
    assert_eq!(
        coordinator
            .attempt_target_resolution(
                &clock,
                expired,
                TargetSelector::ScreenPoint(ScreenPoint::new(20, 20)),
                &[live("window:a", a_bounds)],
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::LeaseExpired
    );
}

#[test]
fn hosted_surface_targets_use_the_same_core_without_surface_types() {
    let clock = FakeClock::new(0);
    let mut coordinator = coordinator();
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
    let bounds = rect(0, 0, 300, 200);
    coordinator
        .publish_lease(
            &clock,
            publication(
                "window:target",
                "client:target",
                1,
                1,
                bounds,
                vec![surface_zone("zone:surface", rect(10, 10, 100, 100))],
            ),
        )
        .unwrap();
    let mut allocator = SequenceAllocator::new([[10; 16]]);
    let session = coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(
                surface_source("window:source", "client:source", 1),
                TransferDuration::new(20),
            ),
        )
        .unwrap()
        .payload()
        .session_id();
    let attempt = coordinator
        .attempt_target_resolution(
            &clock,
            session,
            TargetSelector::ScreenPoint(ScreenPoint::new(20, 20)),
            &[live("window:target", bounds)],
        )
        .unwrap();
    assert_eq!(
        attempt.source().subject_kind(),
        longhorn_transfer::TransferSubjectKind::Surface
    );
}

fn create_panel_session(
    coordinator: &mut longhorn_transfer::TransferCoordinator,
    clock: &FakeClock,
    allocator: &mut SequenceAllocator,
) -> longhorn_transfer::DragSessionId {
    coordinator
        .create_session(
            clock,
            allocator,
            TransferSessionRequest::new(
                panel_source("window:source", "client:source", 1),
                TransferDuration::new(40),
            ),
        )
        .unwrap()
        .payload()
        .session_id()
}
