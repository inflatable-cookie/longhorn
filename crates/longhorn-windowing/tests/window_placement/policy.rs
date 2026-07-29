use longhorn_core::{GeometryError, ScreenSize};
use longhorn_windowing::{
    PlacementPolicy, PlacementReason, PlacementResolutionError, WindowRole,
    resolve_window_placement,
};

use super::support::*;

#[test]
fn soundcheck_minimum_and_work_area_are_explicit_policy_inputs() {
    let home = display_with_work_area(
        "display:main",
        rect(0, 0, 1440, 900),
        rect(0, 24, 1440, 876),
        true,
    );
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:main")))
        .with_normal_placement(display_id("display:main"), placement(2500, -200, 100, 80));

    let resolved = resolved(
        resolve_window_placement(
            &config,
            &inventory(&[home]),
            PlacementPolicy::new(ScreenSize::new(320, 240), ScreenSize::new(1, 1)),
        )
        .unwrap(),
    );

    assert_eq!(resolved.normal_placement(), placement(1120, 24, 320, 240));
    assert!(
        resolved
            .target_work_area()
            .contains_rect(&rect(1120, 24, 320, 240))
    );
}

#[test]
fn maximized_state_preserves_fitted_normal_geometry() {
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:home")))
        .with_normal_placement(display_id("display:home"), placement(1800, -40, 1400, 1000))
        .with_maximized(true);
    let resolved = resolved(
        resolve_window_placement(
            &config,
            &inventory(&[display("display:home", 1600, 0, 1200, 900, true)]),
            PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(1, 1)),
        )
        .unwrap(),
    );

    assert!(resolved.is_maximized());
    assert_eq!(resolved.normal_placement(), placement(1600, 0, 1200, 900));
}

#[test]
fn minimum_visibility_controls_useful_intersection_before_main_fallback() {
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:gone")))
        .with_normal_placement(display_id("display:gone"), placement(990, 0, 500, 500));
    let displays = inventory(&[
        display("display:tiny-overlap", 0, 0, 1000, 900, false),
        display("display:main", 2000, 0, 1200, 900, true),
    ]);

    let permissive = resolved(
        resolve_window_placement(
            &config,
            &displays,
            PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(1, 1)),
        )
        .unwrap(),
    );
    assert_eq!(
        permissive.target_display_id(),
        &display_id("display:tiny-overlap")
    );
    assert_eq!(
        permissive.reason(),
        &PlacementReason::UsefulIntersection { area: 5_000 }
    );

    let strict = resolved(
        resolve_window_placement(
            &config,
            &displays,
            PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(64, 64)),
        )
        .unwrap(),
    );
    assert_eq!(strict.target_display_id(), &display_id("display:main"));
    assert_eq!(strict.reason(), &PlacementReason::MainDisplay);
}

#[test]
fn rearranged_home_keeps_identity_and_refits_saved_geometry() {
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:home")))
        .with_normal_placement(display_id("display:home"), placement(100, 100, 1000, 700));
    let rearranged = display("display:home", -1600, -200, 1200, 800, true);

    let resolved = resolved(
        resolve_window_placement(
            &config,
            &inventory(&[rearranged]),
            PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(1, 1)),
        )
        .unwrap(),
    );

    assert_eq!(resolved.reason(), &PlacementReason::ConfiguredHome);
    assert_eq!(
        resolved.normal_placement(),
        placement(-1400, -100, 1000, 700)
    );
}

#[test]
fn no_surface_and_surface_shaped_consumers_get_the_same_window_result() {
    struct HostedSurfaceFixture {
        placement: longhorn_windowing::WindowPlacementConfig,
        surface_ids: Vec<&'static str>,
    }

    let plain =
        config(WindowRole::RequiredPrimary).with_home_display(Some(display_id("display:home")));
    let hosted = HostedSurfaceFixture {
        placement: plain.clone(),
        surface_ids: vec!["surface:mix", "surface:edit"],
    };
    let displays = inventory(&[display("display:home", 0, 0, 1600, 900, true)]);
    let policy = PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(1, 1));

    assert_eq!(
        resolve_window_placement(&plain, &displays, policy).unwrap(),
        resolve_window_placement(&hosted.placement, &displays, policy).unwrap()
    );
    assert_eq!(hosted.surface_ids.len(), 2);
}

#[test]
fn empty_selected_work_area_fails_typed() {
    let config =
        config(WindowRole::RequiredPrimary).with_home_display(Some(display_id("display:empty")));
    let empty = display_with_work_area(
        "display:empty",
        rect(0, 0, 1000, 800),
        rect(0, 0, 0, 800),
        true,
    );

    assert_eq!(
        resolve_window_placement(
            &config,
            &inventory(&[empty]),
            PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(1, 1)),
        ),
        Err(PlacementResolutionError::Geometry {
            window_id: window_id("window:main"),
            display_id: display_id("display:empty"),
            source: GeometryError::EmptyBounds,
        })
    );
}

#[test]
fn public_config_and_outcome_serde_round_trip() {
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:home")))
        .with_fallback_displays([display_id("display:fallback")])
        .with_normal_placement(display_id("display:home"), placement(10, 20, 800, 600))
        .with_maximized(true);
    let config_wire = serde_json::to_string(&config).unwrap();
    assert_eq!(
        serde_json::from_str::<longhorn_windowing::WindowPlacementConfig>(&config_wire).unwrap(),
        config
    );

    let outcome = resolve_window_placement(
        &config,
        &inventory(&[display("display:home", 0, 0, 1600, 900, true)]),
        PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(1, 1)),
    )
    .unwrap();
    let outcome_wire = serde_json::to_string(&outcome).unwrap();
    assert_eq!(
        serde_json::from_str::<longhorn_windowing::WindowPlacementResolution>(&outcome_wire)
            .unwrap(),
        outcome
    );
}
