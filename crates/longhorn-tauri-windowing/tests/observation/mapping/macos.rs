//! The macOS mixed-scale mapper.
//!
//! The arrangement under test is measured, not invented: a DELL U3415W at 1x
//! and a built-in Retina display at 2x, read from Core Graphics on 2026-08-17.
//! It is the arrangement that produced Figmatic's `MixedScaleUnavailable`
//! refusal, including the built-in display's negative x origin.
//!
//! ```text
//! DELL U3415W   CGDisplayBounds (0, 0) 3440x1440 pt      backingScale 1.0
//! Built-in XDR  CGDisplayBounds (-1577, 1440) 1800x1169  backingScale 2.0
//! ```
//!
//! Tauri's physical facts for the same desktop are those points times each
//! object's own scale, which is how tao derives them on macOS.

use longhorn_core::{PhysicalSize, ScaleFactor, ScreenSize};
use longhorn_display::DisplayBuiltinStatus;
use longhorn_tauri_windowing::{
    DesktopCoordinateMapper, DesktopMappingError, MacOsDesktopMapper, NativeDesktopPlane,
    NativeDisplayGeometry, PhysicalDesktopSnapshot, UniformScaleMapper,
};

use super::super::support::{display, physical_rect, screen_rect, window};

/// The measured plane.
struct MeasuredPlane {
    displays: Vec<NativeDisplayGeometry>,
}

impl MeasuredPlane {
    /// The two real displays, in Core Graphics order.
    fn new() -> Self {
        Self {
            displays: vec![
                // DELL U3415W, main, 1x. Work area inset 30pt for the menu bar.
                NativeDisplayGeometry::new(
                    screen_rect(0, 0, 3440, 1440),
                    screen_rect(0, 30, 3440, 1410),
                    PhysicalSize::new(3440, 1440),
                    ScaleFactor::from_thousandths(1000).unwrap(),
                    true,
                ),
                // Built-in Retina, 2x, left of and below the main display.
                NativeDisplayGeometry::new(
                    screen_rect(-1577, 1440, 1800, 1169),
                    screen_rect(-1577, 1478, 1800, 1131),
                    PhysicalSize::new(3600, 2338),
                    ScaleFactor::from_thousandths(2000).unwrap(),
                    false,
                ),
            ],
        }
    }
}

impl NativeDesktopPlane for MeasuredPlane {
    fn displays(&self) -> Result<Vec<NativeDisplayGeometry>, String> {
        Ok(self.displays.clone())
    }
}

/// The Tauri-side view of the same two displays: points times own scale.
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

/// The refusal this whole lane exists to lift, kept as the control.
#[test]
fn the_uniform_mapper_still_refuses_the_arrangement_that_started_this() {
    let outcome = UniformScaleMapper.map_desktop(&measured_snapshot());

    assert!(
        matches!(
            outcome,
            Err(DesktopMappingError::MixedScaleUnavailable { ref scales })
                if scales.len() == 2
        ),
        "the uniform mapper's fail-closed contract must not weaken, found {outcome:?}"
    );
}

#[test]
fn the_macos_mapper_establishes_one_plane_across_both_scales() {
    let mapper = MacOsDesktopMapper::new(MeasuredPlane::new());

    let mapped = mapper
        .map_desktop(&measured_snapshot())
        .expect("the measured arrangement maps");

    let dell = &mapped.displays()[0];
    let builtin = &mapped.displays()[1];

    // Read from the platform plane, not divided out of the physical facts.
    assert_eq!(dell.full_bounds(), screen_rect(0, 0, 3440, 1440));
    assert_eq!(dell.work_area(), screen_rect(0, 30, 3440, 1410));
    // The negative origin survives, and the 2x display is not scaled twice.
    assert_eq!(builtin.full_bounds(), screen_rect(-1577, 1440, 1800, 1169));
    assert_eq!(builtin.work_area(), screen_rect(-1577, 1478, 1800, 1131));
}

#[test]
fn mapped_displays_keep_the_observation_identity_they_were_asked_about() {
    let mapper = MacOsDesktopMapper::new(MeasuredPlane::new());

    let mapped = mapper.map_desktop(&measured_snapshot()).expect("maps");

    let ids = mapped
        .displays()
        .iter()
        .map(|display| display.observation_id().as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["probe:dell", "probe:builtin"]);
}

/// Input order is the host's, not the platform's; the mapping may not depend
/// on the two agreeing.
#[test]
fn mapping_is_invariant_under_observation_order() {
    let mapper = MacOsDesktopMapper::new(MeasuredPlane::new());
    let forward = measured_snapshot();
    let mut reversed_displays = forward.displays().to_vec();
    reversed_displays.reverse();
    let reversed = PhysicalDesktopSnapshot::new(reversed_displays, []);

    let forward = mapper.map_desktop(&forward).expect("maps");
    let reversed = mapper.map_desktop(&reversed).expect("maps");

    assert_eq!(forward.displays()[0], reversed.displays()[1]);
    assert_eq!(forward.displays()[1], reversed.displays()[0]);
}

/// A 2x window lands in the same plane as the 1x displays around it — the
/// thing a single-scale mapper could not express at all.
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

    let mapped = MacOsDesktopMapper::new(MeasuredPlane::new())
        .map_desktop(&snapshot)
        .expect("maps");

    let window = &mapped.windows()[0];
    assert_eq!(window.outer_bounds(), screen_rect(-1200, 1600, 900, 700));
    assert_eq!(window.inner_size(), ScreenSize::new(900, 672));
    // And it sits inside the built-in display's mapped bounds, which is the
    // whole point of having one plane.
    let builtin = mapped.displays()[1].full_bounds();
    assert!(window.outer_bounds().origin().x() >= builtin.origin().x());
    assert!(window.outer_bounds().origin().y() >= builtin.origin().y());
}

/// The correlation is the load-bearing assumption. If the host's physical facts
/// stop agreeing with the platform plane, that must surface as a refusal rather
/// than as silently wrong geometry.
#[test]
fn an_observation_the_platform_does_not_recognise_fails_typed() {
    let snapshot = PhysicalDesktopSnapshot::new(
        [display(
            "probe:unknown",
            "Unknown",
            0,
            0,
            1234,
            5678,
            0,
            5678,
            1000,
            true,
            DisplayBuiltinStatus::External,
        )],
        [],
    );

    let outcome = MacOsDesktopMapper::new(MeasuredPlane::new()).map_desktop(&snapshot);

    assert!(
        matches!(outcome, Err(DesktopMappingError::Provider { .. })),
        "expected a typed refusal, found {outcome:?}"
    );
}

/// Two identical external displays cannot be told apart from size, scale, and
/// main status alone. Contract 009 already sets the precedent for this shape:
/// refuse rather than attribute arbitrarily.
#[test]
fn identical_displays_are_refused_rather_than_guessed() {
    let twin = NativeDisplayGeometry::new(
        screen_rect(0, 0, 2560, 1440),
        screen_rect(0, 30, 2560, 1410),
        PhysicalSize::new(2560, 1440),
        ScaleFactor::from_thousandths(1000).unwrap(),
        false,
    );
    let other = NativeDisplayGeometry::new(
        screen_rect(2560, 0, 2560, 1440),
        screen_rect(2560, 30, 2560, 1410),
        PhysicalSize::new(2560, 1440),
        ScaleFactor::from_thousandths(1000).unwrap(),
        false,
    );
    struct Twins(Vec<NativeDisplayGeometry>);
    impl NativeDesktopPlane for Twins {
        fn displays(&self) -> Result<Vec<NativeDisplayGeometry>, String> {
            Ok(self.0.clone())
        }
    }

    let snapshot = PhysicalDesktopSnapshot::new(
        [display(
            "probe:left",
            "Twin",
            0,
            0,
            2560,
            1440,
            30,
            1410,
            1000,
            false,
            DisplayBuiltinStatus::External,
        )],
        [],
    );

    let outcome = MacOsDesktopMapper::new(Twins(vec![twin, other])).map_desktop(&snapshot);

    assert!(
        matches!(outcome, Err(DesktopMappingError::Provider { .. })),
        "expected an ambiguity refusal, found {outcome:?}"
    );
}

/// A uniform desktop must map identically through either mapper: adopting the
/// macOS mapper cannot change behaviour on the arrangements that already worked.
#[test]
fn a_uniform_desktop_maps_identically_through_both_mappers() {
    let uniform_native = NativeDisplayGeometry::new(
        screen_rect(0, 0, 3440, 1440),
        screen_rect(0, 30, 3440, 1410),
        PhysicalSize::new(3440, 1440),
        ScaleFactor::from_thousandths(1000).unwrap(),
        true,
    );
    struct One(NativeDisplayGeometry);
    impl NativeDesktopPlane for One {
        fn displays(&self) -> Result<Vec<NativeDisplayGeometry>, String> {
            Ok(vec![self.0.clone()])
        }
    }

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
        [],
    );

    let through_uniform = UniformScaleMapper.map_desktop(&snapshot).expect("maps");
    let through_macos = MacOsDesktopMapper::new(One(uniform_native))
        .map_desktop(&snapshot)
        .expect("maps");

    assert_eq!(through_uniform, through_macos);
}
