use longhorn_core::{ScreenPoint, ScreenSize, WindowPlacement};
use longhorn_surface_windowing::{SurfaceWindowCompositionErrorCode, compose_surface_window_plan};
use longhorn_windowing::{DesiredWindow, WindowPlacementResolution, WindowRole};

use crate::support::{display, document, inventory, limits, resolve, surface_id, window_id};

#[test]
fn missing_and_returning_preferred_window_do_not_rewrite_surface_state() {
    let source = document();
    let main_only = inventory(&[display("display:main", 0, 1600, true)]);
    let fallback = compose_surface_window_plan(
        limits(),
        &source,
        [surface_id("surface:mix"), surface_id("surface:edit")],
        &[
            resolve(
                "window:main",
                WindowRole::RequiredPrimary,
                "display:main",
                &main_only,
            ),
            resolve(
                "window:preferred",
                WindowRole::Optional,
                "display:right",
                &main_only,
            ),
        ],
        |_| true,
    )
    .unwrap();
    let main = &fallback.windows()[0];
    assert_eq!(main.surfaces().window_id(), &window_id("window:main"));
    assert_eq!(main.surfaces().surfaces().len(), 2);
    assert_eq!(main.surfaces().surfaces()[1].host_preference_index(), 1);

    let both = inventory(&[
        display("display:main", 0, 1600, true),
        display("display:right", 1600, 1200, false),
    ]);
    let returned = compose_surface_window_plan(
        limits(),
        &source,
        [surface_id("surface:mix"), surface_id("surface:edit")],
        &[
            resolve(
                "window:main",
                WindowRole::RequiredPrimary,
                "display:main",
                &both,
            ),
            resolve(
                "window:preferred",
                WindowRole::Optional,
                "display:right",
                &both,
            ),
        ],
        |_| true,
    )
    .unwrap();
    assert_eq!(returned.windows().len(), 2);
    assert_eq!(
        returned.windows()[1].surfaces().surfaces()[0].surface_id(),
        &surface_id("surface:mix")
    );
    assert_eq!(
        returned.windows()[1].surfaces().surfaces()[0].host_preference_index(),
        0
    );
    assert_eq!(source, document());
    assert_eq!(fallback.surface_revision(), returned.surface_revision());
}

#[test]
fn direct_window_outcomes_are_ignored_and_visibility_stays_caller_owned() {
    let displays = inventory(&[display("display:main", 0, 1600, true)]);
    let direct = WindowPlacementResolution::Resolved(
        match resolve(
            "window:direct",
            WindowRole::RequiredPrimary,
            "display:main",
            &displays,
        ) {
            WindowPlacementResolution::Resolved(value) => value,
            other => panic!("expected resolved direct window, got {other:?}"),
        },
    );
    let plan = compose_surface_window_plan(
        limits(),
        &document(),
        [surface_id("surface:edit")],
        &[
            direct,
            resolve(
                "window:main",
                WindowRole::RequiredPrimary,
                "display:main",
                &displays,
            ),
        ],
        |id| id != &window_id("window:main"),
    )
    .unwrap();
    assert_eq!(plan.windows().len(), 1);
    assert!(!plan.windows()[0].desired_window().is_visible());
}

#[test]
fn temporary_display_fallback_remains_window_evidence_not_surface_adoption() {
    let source = document();
    let displays = inventory(&[display("display:main", 0, 1600, true)]);
    let plan = compose_surface_window_plan(
        limits(),
        &source,
        [surface_id("surface:edit")],
        &[resolve(
            "window:main",
            WindowRole::RequiredPrimary,
            "display:missing",
            &displays,
        )],
        |_| true,
    )
    .unwrap();
    let placement = plan.windows()[0].placement();
    assert!(placement.is_temporary_fallback());
    assert_eq!(
        placement.configured_home_display_id().unwrap().as_str(),
        "display:missing"
    );
    assert_eq!(source, document());
}

#[test]
fn duplicate_participating_placement_is_typed() {
    let displays = inventory(&[display("display:main", 0, 1600, true)]);
    let placement = resolve(
        "window:main",
        WindowRole::RequiredPrimary,
        "display:main",
        &displays,
    );
    let error = compose_surface_window_plan(
        limits(),
        &document(),
        [surface_id("surface:edit")],
        &[placement.clone(), placement],
        |_| true,
    )
    .unwrap_err();
    assert_eq!(
        error.code(),
        SurfaceWindowCompositionErrorCode::DuplicatePlacementOutcome
    );
}

#[test]
fn desired_windows_remain_plain_windowing_inputs() {
    let displays = inventory(&[display("display:main", 0, 1600, true)]);
    let plan = compose_surface_window_plan(
        limits(),
        &document(),
        [surface_id("surface:edit")],
        &[resolve(
            "window:main",
            WindowRole::RequiredPrimary,
            "display:main",
            &displays,
        )],
        |_| true,
    )
    .unwrap();
    let desired = plan
        .desired_windows()
        .cloned()
        .collect::<Vec<DesiredWindow>>();
    assert_eq!(desired.len(), 1);
    assert_eq!(desired[0].window_id(), &window_id("window:main"));
    assert_ne!(
        desired[0].placement(),
        WindowPlacement::new(ScreenPoint::new(0, 0), ScreenSize::new(1, 1))
    );
}
