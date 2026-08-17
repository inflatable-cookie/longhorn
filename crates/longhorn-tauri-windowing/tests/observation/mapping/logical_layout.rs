//! The mixed-scale mapper for hosts that derive physical facts from a logical
//! desktop layout.
//!
//! The arrangement under test is measured, not invented: a DELL U3415W at 1x
//! and a built-in Retina display at 2x, read from Core Graphics on 2026-08-17.
//! It is the arrangement that produced Figmatic's `MixedScaleUnavailable`
//! refusal, including the built-in display's negative x origin.
//!
//! ```text
//! DELL U3415W   (0, 0) 3440x1440 pt        backingScale 1.0
//! Built-in XDR  (-1577, 1440) 1800x1169 pt  backingScale 2.0
//! ```
//!
//! Tauri's physical facts for that desktop are those points times each object's
//! own scale, which is how the layout is reported on macOS and on Linux.

use longhorn_core::{PhysicalSize, ScreenSize};
use longhorn_display::DisplayBuiltinStatus;
use longhorn_tauri_windowing::{
    DesktopCoordinateMapper, DesktopMappingError, LogicalLayoutMapper, PhysicalDesktopSnapshot,
    UniformScaleMapper,
};

use super::super::support::{display, physical_rect, screen_rect, window};

/// The two real displays as Tauri reports them: logical points times own scale.
fn measured_snapshot() -> PhysicalDesktopSnapshot {
    PhysicalDesktopSnapshot::new(
        [
            display(
                "probe:dell",
                "DELL U3415W",
                0,
                0,
                3440,
                1440,
                30,
                1410,
                1000,
                true,
                DisplayBuiltinStatus::External,
            ),
            display(
                "probe:builtin",
                "Built-in Retina",
                -3154,
                2880,
                3600,
                2338,
                2956,
                2262,
                2000,
                false,
                DisplayBuiltinStatus::BuiltIn,
            ),
        ],
        [],
    )
}

/// The refusal this lane exists to lift, kept as the control so the
/// fail-closed contract cannot weaken unnoticed.
#[test]
fn the_uniform_mapper_still_refuses_the_arrangement_that_started_this() {
    let outcome = UniformScaleMapper.map_desktop(&measured_snapshot());

    assert!(
        matches!(
            outcome,
            Err(DesktopMappingError::MixedScaleUnavailable { ref scales })
                if scales.len() == 2
        ),
        "expected the mixed-scale refusal, found {outcome:?}"
    );
}

/// The whole point: both scales land in one plane, matching what the platform
/// itself reports.
#[test]
fn both_scales_map_into_one_plane() {
    let mapped = LogicalLayoutMapper
        .map_desktop(&measured_snapshot())
        .expect("the measured arrangement maps");

    let dell = &mapped.displays()[0];
    let builtin = &mapped.displays()[1];

    assert_eq!(dell.full_bounds(), screen_rect(0, 0, 3440, 1440));
    assert_eq!(dell.work_area(), screen_rect(0, 30, 3440, 1410));
    // The negative origin survives and the 2x display is not scaled twice.
    assert_eq!(builtin.full_bounds(), screen_rect(-1577, 1440, 1800, 1169));
    assert_eq!(builtin.work_area(), screen_rect(-1577, 1478, 1800, 1131));
}

#[test]
fn observation_identity_is_preserved() {
    let mapped = LogicalLayoutMapper
        .map_desktop(&measured_snapshot())
        .expect("maps");

    let ids = mapped
        .displays()
        .iter()
        .map(|display| display.observation_id().as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["probe:dell", "probe:builtin"]);
}

/// A 2x window lands in the same plane as the 1x display beside it — the thing
/// a single-scale mapper could not express at all.
#[test]
fn a_window_on_the_retina_display_lands_in_the_shared_plane() {
    let snapshot = PhysicalDesktopSnapshot::new(
        measured_snapshot().displays().to_vec(),
        [window(
            "win:studio",
            "studio",
            physical_rect(-2400, 3200, 1800, 1400),
            PhysicalSize::new(1800, 1344),
            2000,
        )],
    );

    let mapped = LogicalLayoutMapper.map_desktop(&snapshot).expect("maps");

    let window = &mapped.windows()[0];
    assert_eq!(window.outer_bounds(), screen_rect(-1200, 1600, 900, 700));
    assert_eq!(window.inner_size(), ScreenSize::new(900, 672));
    // And it sits within the built-in display's mapped bounds, which is what
    // having one plane is for.
    let builtin = mapped.displays()[1].full_bounds();
    assert!(window.outer_bounds().origin().x() >= builtin.origin().x());
    assert!(window.outer_bounds().origin().y() >= builtin.origin().y());
}

/// Windows and displays may be at different scales in the same snapshot; each
/// converts through its own.
#[test]
fn each_object_converts_through_its_own_scale() {
    let snapshot = PhysicalDesktopSnapshot::new(
        measured_snapshot().displays().to_vec(),
        [
            window(
                "win:on-dell",
                "on-dell",
                physical_rect(100, 200, 800, 600),
                PhysicalSize::new(800, 572),
                1000,
            ),
            window(
                "win:on-builtin",
                "on-builtin",
                physical_rect(-2400, 3200, 1800, 1400),
                PhysicalSize::new(1800, 1344),
                2000,
            ),
        ],
    );

    let mapped = LogicalLayoutMapper.map_desktop(&snapshot).expect("maps");

    assert_eq!(
        mapped.windows()[0].outer_bounds(),
        screen_rect(100, 200, 800, 600),
        "a 1x window is unchanged"
    );
    assert_eq!(
        mapped.windows()[1].outer_bounds(),
        screen_rect(-1200, 1600, 900, 700),
        "a 2x window halves"
    );
}

/// A uniform desktop must map identically through either mapper: adopting this
/// one cannot change behaviour on arrangements that already worked.
#[test]
fn a_uniform_desktop_maps_identically_through_both_mappers() {
    let snapshot = PhysicalDesktopSnapshot::new(
        [display(
            "probe:dell",
            "DELL U3415W",
            0,
            0,
            3440,
            1440,
            30,
            1410,
            1000,
            true,
            DisplayBuiltinStatus::External,
        )],
        [window(
            "win:only",
            "only",
            physical_rect(10, 20, 640, 480),
            PhysicalSize::new(640, 450),
            1000,
        )],
    );

    let through_uniform = UniformScaleMapper.map_desktop(&snapshot).expect("maps");
    let through_layout = LogicalLayoutMapper.map_desktop(&snapshot).expect("maps");

    assert_eq!(through_uniform, through_layout);
}
