use longhorn_core::{PhysicalSize, ScreenSize};
use longhorn_display::DisplayBuiltinStatus;
use longhorn_tauri_windowing::{
    DesktopMappingError, DesktopObservationError, MappedDesktopGeometry, PhysicalDesktopSnapshot,
    UniformScaleMapper, project_desktop,
};

use super::support::{display, mapped_display, physical_rect, screen_rect, window};

#[path = "mapping/projection.rs"]
mod projection;

#[test]
fn loophole_uniform_desktop_preserves_full_work_and_frame_distinctions() {
    let snapshot = PhysicalDesktopSnapshot::new(
        [
            display(
                "probe:studio",
                "Studio Display",
                0,
                0,
                3840,
                2160,
                48,
                2112,
                2000,
                true,
                DisplayBuiltinStatus::External,
            ),
            display(
                "probe:side",
                "Side Display",
                3840,
                0,
                2560,
                1440,
                48,
                1392,
                2000,
                false,
                DisplayBuiltinStatus::External,
            ),
        ],
        [window(
            "window:main",
            "main",
            physical_rect(200, 300, 1640, 1260),
            PhysicalSize::new(1600, 1200),
            2000,
        )],
    );

    let observed = project_desktop(&snapshot, &UniformScaleMapper).unwrap();
    assert_eq!(
        observed.displays()[1].facts().full_bounds(),
        screen_rect(0, 0, 1920, 1080)
    );
    assert_eq!(
        observed.displays()[1].facts().work_area(),
        screen_rect(0, 24, 1920, 1056)
    );
    assert_eq!(
        observed.windows()[0].metrics().outer_bounds(),
        screen_rect(100, 150, 820, 630)
    );
    assert_eq!(
        observed.windows()[0].metrics().inner_size(),
        ScreenSize::new(800, 600)
    );
}

#[test]
fn nucleus_negative_origin_survives_checked_nearest_conversion() {
    let snapshot = PhysicalDesktopSnapshot::new(
        [display(
            "probe:left",
            "Left",
            -1920,
            -300,
            1920,
            1200,
            -270,
            1170,
            1500,
            true,
            DisplayBuiltinStatus::Unknown,
        )],
        [],
    );

    let observed = project_desktop(&snapshot, &UniformScaleMapper).unwrap();
    assert_eq!(
        observed.displays()[0].facts().full_bounds(),
        screen_rect(-1280, -200, 1280, 800)
    );
    assert_eq!(
        observed.displays()[0].facts().builtin_status(),
        DisplayBuiltinStatus::Unknown
    );
}

#[test]
fn mixed_scale_is_unavailable_without_whole_desktop_provider() {
    let snapshot = PhysicalDesktopSnapshot::new(
        [
            display(
                "probe:a",
                "A",
                0,
                0,
                1920,
                1080,
                0,
                1040,
                1000,
                true,
                DisplayBuiltinStatus::Unknown,
            ),
            display(
                "probe:b",
                "B",
                1920,
                0,
                2560,
                1440,
                0,
                1400,
                2000,
                false,
                DisplayBuiltinStatus::Unknown,
            ),
        ],
        [],
    );

    assert!(matches!(
        project_desktop(&snapshot, &UniformScaleMapper),
        Err(DesktopObservationError::Mapping(
            DesktopMappingError::MixedScaleUnavailable { .. }
        ))
    ));
}

#[test]
fn injected_mapper_can_establish_one_mixed_scale_plane() {
    let snapshot = PhysicalDesktopSnapshot::new(
        [
            display(
                "probe:a",
                "A",
                0,
                0,
                1920,
                1080,
                0,
                1040,
                1000,
                true,
                DisplayBuiltinStatus::Unknown,
            ),
            display(
                "probe:b",
                "B",
                1920,
                0,
                2560,
                1440,
                0,
                1400,
                2000,
                false,
                DisplayBuiltinStatus::External,
            ),
        ],
        [],
    );
    let mapper = |_: &PhysicalDesktopSnapshot| {
        Ok(MappedDesktopGeometry::new(
            [
                mapped_display(
                    "probe:a",
                    screen_rect(0, 0, 1920, 1080),
                    screen_rect(0, 0, 1920, 1040),
                ),
                mapped_display(
                    "probe:b",
                    screen_rect(1920, 0, 1280, 720),
                    screen_rect(1920, 0, 1280, 700),
                ),
            ],
            [],
        ))
    };

    let observed = project_desktop(&snapshot, &mapper).unwrap();
    assert_eq!(
        observed.displays()[1].facts().full_bounds(),
        screen_rect(1920, 0, 1280, 720)
    );
}
