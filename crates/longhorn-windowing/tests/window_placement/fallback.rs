use longhorn_core::ScreenSize;
use longhorn_windowing::{
    PlacementPolicy, PlacementReason, UnavailablePlacementReason, WindowPlacementResolution,
    WindowRole, resolve_window_placement,
};

use super::support::*;

fn policy() -> PlacementPolicy {
    PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(1, 1))
}

#[test]
fn nucleus_saved_display_wins_before_all_fallbacks() {
    let home = display("display:home", 0, 0, 1600, 900, false);
    let main = display("display:main", 1600, 0, 1200, 900, true);
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:home")))
        .with_fallback_displays([display_id("display:main")])
        .with_normal_placement(display_id("display:home"), placement(120, 80, 900, 700));

    let resolved =
        resolved(resolve_window_placement(&config, &inventory(&[main, home]), policy()).unwrap());

    assert_eq!(resolved.target_display_id(), &display_id("display:home"));
    assert_eq!(resolved.reason(), &PlacementReason::ConfiguredHome);
    assert_eq!(resolved.normal_placement(), placement(120, 80, 900, 700));
    assert!(!resolved.is_temporary_fallback());
}

#[test]
fn loophole_uses_first_available_configured_fallback_and_its_memory() {
    let first_missing = display_id("display:missing-first");
    let second = display_id("display:second");
    let third = display_id("display:third");
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:home")))
        .with_fallback_displays([first_missing, second.clone(), third.clone()])
        .with_normal_placement(second.clone(), placement(1700, 40, 700, 500))
        .with_normal_placement(third, placement(2900, 40, 600, 400));

    let resolved = resolved(
        resolve_window_placement(
            &config,
            &inventory(&[
                display("display:third", 2800, 0, 1200, 900, false),
                display("display:second", 1600, 0, 1200, 900, false),
            ]),
            policy(),
        )
        .unwrap(),
    );

    assert_eq!(resolved.target_display_id(), &second);
    assert_eq!(resolved.reason(), &PlacementReason::ConfiguredFallback);
    assert_eq!(
        resolved.configured_home_display_id(),
        Some(&display_id("display:home"))
    );
    assert_eq!(resolved.normal_placement(), placement(1700, 40, 700, 500));
    assert!(resolved.is_temporary_fallback());
    assert_eq!(config.home_display_id(), Some(&display_id("display:home")));
}

#[test]
fn nucleus_missing_home_uses_largest_useful_intersection() {
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:gone")))
        .with_normal_placement(display_id("display:gone"), placement(1700, 50, 1000, 700));

    let resolved = resolved(
        resolve_window_placement(
            &config,
            &inventory(&[
                display("display:main", 0, 0, 1600, 900, true),
                display("display:side", 1600, 0, 1200, 900, false),
            ]),
            policy(),
        )
        .unwrap(),
    );

    assert_eq!(resolved.target_display_id(), &display_id("display:side"));
    assert_eq!(
        resolved.reason(),
        &PlacementReason::UsefulIntersection { area: 700_000 }
    );
    assert_eq!(resolved.normal_placement(), placement(1700, 50, 1000, 700));
}

#[test]
fn equal_intersection_ties_use_display_id_under_input_permutation() {
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:gone")))
        .with_normal_placement(display_id("display:gone"), placement(500, 0, 1000, 600));
    let left = display("display:a", 0, 0, 1000, 900, false);
    let right = display("display:b", 1000, 0, 1000, 900, false);

    let forward = resolve_window_placement(
        &config,
        &inventory(&[left.clone(), right.clone()]),
        policy(),
    )
    .unwrap();
    let reverse = resolve_window_placement(&config, &inventory(&[right, left]), policy()).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(
        resolved(forward).target_display_id(),
        &display_id("display:a")
    );
}

#[test]
fn nucleus_uses_main_then_canonical_first_when_no_intersection_exists() {
    let config = config(WindowRole::RequiredPrimary)
        .with_home_display(Some(display_id("display:gone")))
        .with_normal_placement(display_id("display:gone"), placement(9000, 9000, 800, 600));

    let main = resolved(
        resolve_window_placement(
            &config,
            &inventory(&[
                display("display:a", -1200, 0, 1200, 900, false),
                display("display:z", 0, 0, 1600, 900, true),
            ]),
            policy(),
        )
        .unwrap(),
    );
    assert_eq!(main.target_display_id(), &display_id("display:z"));
    assert_eq!(main.reason(), &PlacementReason::MainDisplay);

    let first = resolved(
        resolve_window_placement(
            &config,
            &inventory(&[
                display("display:z", 0, 0, 1600, 900, false),
                display("display:a", -1200, 0, 1200, 900, false),
            ]),
            policy(),
        )
        .unwrap(),
    );
    assert_eq!(first.target_display_id(), &display_id("display:a"));
    assert_eq!(first.reason(), &PlacementReason::DeterministicFallback);
}

#[test]
fn unavailable_and_disabled_outcomes_never_fabricate_geometry() {
    let required = config(WindowRole::RequiredPrimary);
    assert!(matches!(
        resolve_window_placement(&required, &inventory(&[]), policy()).unwrap(),
        WindowPlacementResolution::Unavailable(unavailable)
            if unavailable.reason() == UnavailablePlacementReason::NoAvailableDisplays
    ));

    let optional =
        config(WindowRole::Optional).with_home_display(Some(display_id("display:missing")));
    assert!(matches!(
        resolve_window_placement(
            &optional,
            &inventory(&[display("display:available", 0, 0, 1000, 800, true)]),
            policy(),
        )
        .unwrap(),
        WindowPlacementResolution::Unavailable(unavailable)
            if unavailable.reason()
                == UnavailablePlacementReason::NoConfiguredDisplayAvailable
    ));

    let disabled = config(WindowRole::RequiredPrimary).with_enabled(false);
    assert!(matches!(
        resolve_window_placement(
            &disabled,
            &inventory(&[display("display:available", 0, 0, 1000, 800, true)]),
            policy(),
        )
        .unwrap(),
        WindowPlacementResolution::Disabled { .. }
    ));
}
