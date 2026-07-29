use longhorn_core::ScreenPoint;
use longhorn_transfer::{
    DropZoneId, TargetResolutionPath, TargetSelector, TransferDuration, TransferErrorCode,
    TransferSessionRequest,
};

use super::support::{
    FakeClock, SequenceAllocator, bind, coordinator, live, panel_source, panel_zone, publication,
    rect,
};

#[test]
fn explicit_zone_and_screen_point_resolve_the_same_direct_container_target() {
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
    let bounds = rect(100, 100, 400, 300);
    coordinator
        .publish_lease(
            &clock,
            publication(
                "window:target",
                "client:target",
                1,
                1,
                bounds,
                vec![panel_zone("zone:main", rect(150, 140, 120, 80), Some(2))],
            ),
        )
        .unwrap();
    let mut allocator = SequenceAllocator::new([[1; 16], [2; 16]]);
    let explicit_id = create_panel_session(&mut coordinator, &clock, &mut allocator);
    let point_id = create_panel_session(&mut coordinator, &clock, &mut allocator);
    let live = [live("window:target", bounds)];

    let explicit = coordinator
        .attempt_target_resolution(
            &clock,
            explicit_id,
            TargetSelector::ExplicitZone(DropZoneId::new("zone:main").unwrap()),
            &live,
        )
        .unwrap();
    let point = coordinator
        .attempt_target_resolution(
            &clock,
            point_id,
            TargetSelector::ScreenPoint(ScreenPoint::new(160, 150)),
            &live,
        )
        .unwrap();
    assert_eq!(explicit.target().path(), TargetResolutionPath::ExplicitZone);
    assert_eq!(point.target().path(), TargetResolutionPath::ScreenPoint);
    assert_eq!(explicit.target().window_id(), point.target().window_id());
    assert_eq!(explicit.target().zone(), point.target().zone());
    assert!(explicit.source().panel_placement().is_some());
}

#[test]
fn overlapping_windows_and_zones_reject_without_enumeration_order() {
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
    let bounds = rect(0, 0, 300, 300);
    coordinator
        .publish_lease(
            &clock,
            publication(
                "window:a",
                "client:a",
                1,
                1,
                bounds,
                vec![
                    panel_zone("zone:a", rect(20, 20, 100, 100), None),
                    panel_zone("zone:b", rect(40, 40, 100, 100), None),
                ],
            ),
        )
        .unwrap();
    let mut allocator = SequenceAllocator::new([[3; 16], [4; 16], [5; 16]]);
    let overlap_zones = create_panel_session(&mut coordinator, &clock, &mut allocator);
    let error = coordinator
        .attempt_target_resolution(
            &clock,
            overlap_zones,
            TargetSelector::ScreenPoint(ScreenPoint::new(50, 50)),
            &[live("window:a", bounds)],
        )
        .unwrap_err();
    assert_eq!(error.code(), TransferErrorCode::AmbiguousZone);
    assert!(error.session_consumed());
    assert_eq!(
        coordinator
            .attempt_target_resolution(
                &clock,
                overlap_zones,
                TargetSelector::ScreenPoint(ScreenPoint::new(50, 50)),
                &[live("window:a", bounds)],
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::SessionReplayed
    );

    let forward = [
        live("window:a", bounds),
        live("window:b", rect(30, 30, 200, 200)),
    ];
    for windows in [
        vec![forward[0].clone(), forward[1].clone()],
        vec![forward[1].clone(), forward[0].clone()],
    ] {
        let session = create_panel_session(&mut coordinator, &clock, &mut allocator);
        assert_eq!(
            coordinator
                .attempt_target_resolution(
                    &clock,
                    session,
                    TargetSelector::ScreenPoint(ScreenPoint::new(50, 50)),
                    &windows,
                )
                .unwrap_err()
                .code(),
            TransferErrorCode::AmbiguousWindow
        );
    }
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
