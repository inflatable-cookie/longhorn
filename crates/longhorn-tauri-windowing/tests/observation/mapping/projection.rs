use longhorn_core::{PhysicalPoint, PhysicalRect, PhysicalSize, ScreenSize};
use longhorn_display::DisplayBuiltinStatus;
use longhorn_tauri_windowing::{
    DesktopMappingError, DesktopObservationError, MappedDesktopGeometry, PhysicalDesktopSnapshot,
    UniformScaleMapper, project_desktop,
};

use crate::support::{display, mapped_display, mapped_window, screen_rect, window};

#[test]
fn mapping_is_complete_permutation_invariant_and_serializable() {
    let a = display(
        "probe:a",
        "Soundcheck",
        0,
        0,
        1728,
        1117,
        25,
        1092,
        1000,
        true,
        DisplayBuiltinStatus::Unknown,
    );
    let b = display(
        "probe:b",
        "External",
        -1280,
        0,
        1280,
        720,
        0,
        700,
        1000,
        false,
        DisplayBuiltinStatus::External,
    );
    let forward = PhysicalDesktopSnapshot::new([a.clone(), b.clone()], []);
    let reverse = PhysicalDesktopSnapshot::new([b, a], []);

    let first = project_desktop(&forward, &UniformScaleMapper).unwrap();
    let second = project_desktop(&reverse, &UniformScaleMapper).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        serde_json::from_str::<PhysicalDesktopSnapshot>(&serde_json::to_string(&forward).unwrap())
            .unwrap(),
        forward
    );
    assert!(
        serde_json::to_string(&first)
            .unwrap()
            .contains("\"builtin_status\":\"unknown\"")
    );

    let incomplete = |_: &PhysicalDesktopSnapshot| {
        Ok(MappedDesktopGeometry::new(
            [mapped_display(
                "probe:a",
                screen_rect(0, 0, 1728, 1117),
                screen_rect(0, 25, 1728, 1092),
            )],
            [],
        ))
    };
    assert!(matches!(
        project_desktop(&forward, &incomplete),
        Err(DesktopObservationError::MissingDisplayMapping(_))
    ));
}

#[test]
fn checked_conversion_rejects_geometry_overflow() {
    let snapshot = PhysicalDesktopSnapshot::new(
        [display(
            "probe:overflow",
            "Overflow",
            i32::MAX,
            0,
            1,
            1,
            0,
            1,
            1,
            true,
            DisplayBuiltinStatus::Unknown,
        )],
        [],
    );

    assert!(matches!(
        project_desktop(&snapshot, &UniformScaleMapper),
        Err(DesktopObservationError::Mapping(
            DesktopMappingError::Conversion(_)
        ))
    ));
}

#[test]
fn missing_managed_window_mapping_fails_the_complete_projection() {
    let raw_window = window(
        "window:main",
        "main",
        PhysicalRect::new(PhysicalPoint::new(0, 0), PhysicalSize::new(820, 630)),
        PhysicalSize::new(800, 600),
        1000,
    );
    let snapshot = PhysicalDesktopSnapshot::new([], [raw_window]);
    let mapper = |_: &PhysicalDesktopSnapshot| {
        Ok(MappedDesktopGeometry::new(
            [],
            [mapped_window(
                "other",
                screen_rect(0, 0, 820, 630),
                ScreenSize::new(800, 600),
            )],
        ))
    };

    assert!(matches!(
        project_desktop(&snapshot, &mapper),
        Err(DesktopObservationError::UnexpectedWindowMapping(_))
    ));
}
