use longhorn_core::{ScaleFactor, ScreenSize};
use longhorn_windowing::{
    PlacementPolicy, PlacementReason, SavedDisplayAssociation, SavedDisplayEvidence,
    SavedWindowPlacement, WindowRole, restore_window_placement,
};

use super::support::*;

fn policy() -> PlacementPolicy {
    PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(80, 80))
}

#[test]
fn canonical_saved_display_restores_before_intersection() {
    let saved = SavedWindowPlacement::new(
        window_id("window:main"),
        placement(100, 100, 800, 600),
        true,
        SavedDisplayAssociation::new(Some(display_id("display:side")), None),
    );
    let restored = resolved(
        restore_window_placement(
            &saved,
            &inventory(&[
                display("display:main", 0, 0, 1600, 900, true),
                display("display:side", 1600, 0, 1200, 900, false),
            ]),
            WindowRole::RequiredPrimary,
            policy(),
        )
        .unwrap(),
    );

    assert_eq!(restored.target_display_id(), &display_id("display:side"));
    assert_eq!(restored.reason(), &PlacementReason::ConfiguredHome);
    assert!(restored.is_maximized());
}

#[test]
fn exact_saved_evidence_recovers_a_reallocated_display_id() {
    let side_bounds = rect(1600, 0, 1200, 900);
    let saved = SavedWindowPlacement::new(
        window_id("window:main"),
        placement(1700, 40, 800, 600),
        false,
        SavedDisplayAssociation::new(
            Some(display_id("display:old-side")),
            Some(SavedDisplayEvidence::new(
                side_bounds,
                side_bounds,
                ScaleFactor::from_thousandths(1000).unwrap(),
            )),
        ),
    );
    let restored = resolved(
        restore_window_placement(
            &saved,
            &inventory(&[
                display("display:main", 0, 0, 1600, 900, true),
                display("display:new-side", 1600, 0, 1200, 900, false),
            ]),
            WindowRole::RequiredPrimary,
            policy(),
        )
        .unwrap(),
    );

    assert_eq!(
        restored.target_display_id(),
        &display_id("display:new-side")
    );
    assert_eq!(restored.reason(), &PlacementReason::ConfiguredHome);
}

#[test]
fn missing_saved_display_uses_intersection_then_main() {
    let saved = SavedWindowPlacement::new(
        window_id("window:main"),
        placement(1700, 40, 800, 600),
        false,
        SavedDisplayAssociation::new(Some(display_id("display:gone")), None),
    );
    let overlapping = resolved(
        restore_window_placement(
            &saved,
            &inventory(&[
                display("display:main", 0, 0, 1600, 900, true),
                display("display:side", 1600, 0, 1200, 900, false),
            ]),
            WindowRole::RequiredPrimary,
            policy(),
        )
        .unwrap(),
    );
    assert_eq!(overlapping.target_display_id(), &display_id("display:side"));
    assert!(matches!(
        overlapping.reason(),
        PlacementReason::UsefulIntersection { .. }
    ));

    let main = resolved(
        restore_window_placement(
            &SavedWindowPlacement::new(
                window_id("window:main"),
                placement(9000, 9000, 800, 600),
                false,
                SavedDisplayAssociation::unresolved(),
            ),
            &inventory(&[display("display:main", 0, 0, 1600, 900, true)]),
            WindowRole::RequiredPrimary,
            policy(),
        )
        .unwrap(),
    );
    assert_eq!(main.reason(), &PlacementReason::MainDisplay);
    assert!(
        main.target_work_area()
            .contains_rect(&rect(800, 300, 800, 600))
    );
    assert_eq!(main.normal_placement(), placement(800, 300, 800, 600));
}

#[test]
fn saved_record_round_trips_with_display_evidence() {
    let saved = SavedWindowPlacement::new(
        window_id("window:main"),
        placement(-1200, 40, 1000, 700),
        false,
        SavedDisplayAssociation::new(
            Some(display_id("display:left")),
            Some(SavedDisplayEvidence::new(
                rect(-1200, 0, 1200, 900),
                rect(-1200, 0, 1200, 860),
                ScaleFactor::from_thousandths(1250).unwrap(),
            )),
        ),
    );

    let encoded = serde_json::to_string(&saved).unwrap();
    let decoded: SavedWindowPlacement = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, saved);
}

#[test]
fn poisoned_double_scale_size_is_clamped_to_the_work_area() {
    let saved = SavedWindowPlacement::new(
        window_id("window:main"),
        placement(0, 0, 3_160, 2_026),
        false,
        SavedDisplayAssociation::new(Some(display_id("display:main")), None),
    );
    let restored = resolved(
        restore_window_placement(
            &saved,
            &inventory(&[display("display:main", 0, 0, 1_580, 1_013, true)]),
            WindowRole::RequiredPrimary,
            policy(),
        )
        .unwrap(),
    );

    assert_eq!(restored.normal_placement(), placement(0, 0, 1_580, 1_013));
}

#[test]
fn reachable_edge_placement_keeps_its_saved_origin() {
    let saved = SavedWindowPlacement::new(
        window_id("window:main"),
        placement(1_400, 100, 1_200, 800),
        false,
        SavedDisplayAssociation::new(Some(display_id("display:main")), None),
    );
    let restored = resolved(
        restore_window_placement(
            &saved,
            &inventory(&[display("display:main", 0, 0, 1_512, 982, true)]),
            WindowRole::RequiredPrimary,
            policy(),
        )
        .unwrap(),
    );

    assert_eq!(
        restored.normal_placement(),
        placement(1_400, 100, 1_200, 800)
    );
}
