//! Contract fixtures for Longhorn's typed coordinate and geometry foundation.

use longhorn_core::{
    ClientPoint, DisplayId, LiveWindowMetrics, PhysicalPoint, PhysicalPx, PhysicalRect,
    PhysicalSize, RoundingMode, ScaleConversionError, ScaleFactor, ScreenPoint, ScreenRect,
    ScreenSize, ScreenVector, WindowId, WindowPlacement,
};
use proptest::prelude::*;

#[test]
fn public_serde_shape_is_stable_and_explicit() {
    let display_id = DisplayId::new("display:0198f97e").unwrap();
    let window_id = WindowId::new("window:primary").unwrap();
    let scale = ScaleFactor::from_thousandths(1500).unwrap();
    let point = ScreenPoint::new(-1920, 40);
    let size = ScreenSize::new(1280, 720);
    let rect = ScreenRect::new(point, size);
    let client = ClientPoint::new(12.5, -3.25).unwrap();

    assert_eq!(
        serde_json::to_string(&display_id).unwrap(),
        r#""display:0198f97e""#
    );
    assert_eq!(
        serde_json::to_string(&window_id).unwrap(),
        r#""window:primary""#
    );
    assert_eq!(serde_json::to_string(&scale).unwrap(), "1500");
    assert_eq!(
        serde_json::to_string(&point).unwrap(),
        r#"{"x":-1920,"y":40}"#
    );
    assert_eq!(
        serde_json::to_string(&size).unwrap(),
        r#"{"width":1280,"height":720}"#
    );
    assert_eq!(
        serde_json::to_string(&rect).unwrap(),
        r#"{"origin":{"x":-1920,"y":40},"size":{"width":1280,"height":720}}"#
    );
    assert_eq!(
        serde_json::to_string(&client).unwrap(),
        r#"{"x":12.5,"y":-3.25}"#
    );

    assert_eq!(
        serde_json::from_str::<ScreenRect>(&serde_json::to_string(&rect).unwrap()).unwrap(),
        rect
    );
}

#[test]
fn identity_scale_is_exact_and_conversion_overflow_is_typed() {
    let identity = ScaleFactor::from_thousandths(1000).unwrap();
    let point = PhysicalPoint::new(i32::MIN, i32::MAX);
    let size = PhysicalSize::new(u32::MAX, 42);

    assert_eq!(
        identity
            .physical_point_to_screen(point, RoundingMode::Nearest)
            .unwrap(),
        ScreenPoint::new(i32::MIN, i32::MAX)
    );
    assert_eq!(
        identity
            .physical_size_to_screen(size, RoundingMode::Nearest)
            .unwrap(),
        ScreenSize::new(u32::MAX, 42)
    );

    let tiny_scale = ScaleFactor::from_thousandths(1).unwrap();
    assert_eq!(
        tiny_scale.physical_to_screen(PhysicalPx::new(i32::MAX), RoundingMode::Nearest),
        Err(ScaleConversionError::Overflow)
    );
}

#[test]
fn nucleus_negative_origin_and_oversized_window_fit() {
    let work_area = ScreenRect::new(ScreenPoint::new(-1920, 0), ScreenSize::new(1920, 1080));
    let oversized = ScreenRect::new(ScreenPoint::new(-2500, -100), ScreenSize::new(3000, 1600));

    let fitted = oversized
        .fit_within(&work_area, &ScreenSize::new(320, 240))
        .unwrap();

    assert_eq!(fitted, work_area);
    assert_eq!(fitted.origin(), ScreenPoint::new(-1920, 0));
}

#[test]
fn soundcheck_minimum_is_policy_input_not_a_core_constant() {
    let work_area = ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(1440, 900));
    let tiny = ScreenRect::new(ScreenPoint::new(1500, 950), ScreenSize::new(10, 20));
    let soundcheck_minimum = ScreenSize::new(320, 240);

    let fitted = tiny.fit_within(&work_area, &soundcheck_minimum).unwrap();

    assert_eq!(fitted.size(), soundcheck_minimum);
    assert_eq!(fitted.origin(), ScreenPoint::new(1120, 660));
    assert!(work_area.contains_rect(&fitted));
}

#[test]
fn minimum_visibility_moves_without_resizing() {
    let work_area = ScreenRect::new(ScreenPoint::new(-1280, 0), ScreenSize::new(1280, 800));
    let hidden = ScreenRect::new(ScreenPoint::new(-2000, -1000), ScreenSize::new(640, 480));

    let visible = hidden
        .ensure_minimum_visible(&work_area, &ScreenSize::new(64, 48))
        .unwrap();
    let intersection = visible.intersection(&work_area).unwrap();

    assert_eq!(visible.size(), hidden.size());
    assert!(intersection.size().width() >= 64);
    assert!(intersection.size().height() >= 48);
}

#[test]
fn contained_window_is_unchanged_by_visibility_clamp() {
    let work_area = ScreenRect::new(ScreenPoint::new(-1920, -200), ScreenSize::new(1920, 1080));
    let contained = ScreenRect::new(ScreenPoint::new(-1600, 0), ScreenSize::new(800, 600));

    assert_eq!(
        contained
            .ensure_minimum_visible(&work_area, &ScreenSize::new(64, 48))
            .unwrap(),
        contained
    );
}

#[test]
fn loophole_placement_and_live_metrics_preserve_frame_distinction() {
    let placement = WindowPlacement::new(ScreenPoint::new(40, 80), ScreenSize::new(1200, 700));
    let live = LiveWindowMetrics::new(
        ScreenRect::new(ScreenPoint::new(40, 80), ScreenSize::new(1216, 739)),
        ScreenSize::new(1200, 700),
    );

    assert_eq!(placement.outer_origin(), live.outer_bounds().origin());
    assert_eq!(placement.inner_size(), live.inner_size());
    assert_ne!(placement.inner_size(), live.outer_bounds().size());
}

#[test]
fn checked_translation_rejects_coordinate_overflow() {
    let rect = ScreenRect::new(ScreenPoint::new(i32::MAX, 0), ScreenSize::new(10, 10));

    assert!(rect.checked_translate(&ScreenVector::new(1, 0)).is_err());
}

proptest! {
    #[test]
    fn nearest_physical_round_trip_obeys_scale_quantization_bound(
        thousandths in 1_u32..=u32::MAX,
        value in -1_000_000_i32..=1_000_000,
    ) {
        let scale = ScaleFactor::from_thousandths(thousandths).unwrap();
        let physical = PhysicalPx::new(value);
        let screen = scale
            .physical_to_screen(physical, RoundingMode::Nearest)
            .unwrap();
        let round_trip = scale
            .screen_to_physical(screen, RoundingMode::Nearest)
            .unwrap();

        let maximum_error = i64::try_from(u64::from(thousandths).div_ceil(2000)).unwrap();
        prop_assert!(
            (i64::from(round_trip.get()) - i64::from(value)).abs() <= maximum_error
        );
    }

    #[test]
    fn intersection_is_commutative_and_bounded(
        ax in -10_000_i32..=10_000,
        ay in -10_000_i32..=10_000,
        aw in 0_u32..=20_000,
        ah in 0_u32..=20_000,
        bx in -10_000_i32..=10_000,
        by in -10_000_i32..=10_000,
        bw in 0_u32..=20_000,
        bh in 0_u32..=20_000,
    ) {
        let a = ScreenRect::new(ScreenPoint::new(ax, ay), ScreenSize::new(aw, ah));
        let b = ScreenRect::new(ScreenPoint::new(bx, by), ScreenSize::new(bw, bh));
        let intersection = a.intersection(&b);

        prop_assert_eq!(intersection, b.intersection(&a));
        if let Some(intersection) = intersection {
            prop_assert!(intersection.area() <= a.area());
            prop_assert!(intersection.area() <= b.area());
            prop_assert!(a.contains_rect(&intersection));
            prop_assert!(b.contains_rect(&intersection));
        }
    }

    #[test]
    fn contained_rectangles_are_unchanged_by_fit(
        work_x in -10_000_i32..=10_000,
        work_y in -10_000_i32..=10_000,
        work_width in 1_u32..=4000,
        work_height in 1_u32..=4000,
        width_fraction in 0_u32..=1000,
        height_fraction in 0_u32..=1000,
    ) {
        let width = u32::try_from(
            u64::from(work_width) * u64::from(width_fraction) / 1000
        ).unwrap();
        let height = u32::try_from(
            u64::from(work_height) * u64::from(height_fraction) / 1000
        ).unwrap();
        let work = ScreenRect::new(
            ScreenPoint::new(work_x, work_y),
            ScreenSize::new(work_width, work_height),
        );
        let rect = ScreenRect::new(
            ScreenPoint::new(work_x, work_y),
            ScreenSize::new(width, height),
        );
        let minimum = ScreenSize::new(width, height);

        prop_assert_eq!(rect.fit_within(&work, &minimum).unwrap(), rect);
    }

    #[test]
    fn arbitrary_windows_fit_inside_nonempty_bounds(
        work_x in -10_000_i32..=10_000,
        work_y in -10_000_i32..=10_000,
        work_width in 1_u32..=4000,
        work_height in 1_u32..=4000,
        window_x in -20_000_i32..=20_000,
        window_y in -20_000_i32..=20_000,
        window_width in 0_u32..=8000,
        window_height in 0_u32..=8000,
        minimum_width in 0_u32..=8000,
        minimum_height in 0_u32..=8000,
    ) {
        let work = ScreenRect::new(
            ScreenPoint::new(work_x, work_y),
            ScreenSize::new(work_width, work_height),
        );
        let window = ScreenRect::new(
            ScreenPoint::new(window_x, window_y),
            ScreenSize::new(window_width, window_height),
        );
        let minimum = ScreenSize::new(minimum_width, minimum_height);
        let fitted = window.fit_within(&work, &minimum).unwrap();

        prop_assert!(work.contains_rect(&fitted));
        prop_assert_eq!(
            fitted.size().width(),
            window_width.max(minimum_width).min(work_width)
        );
        prop_assert_eq!(
            fitted.size().height(),
            window_height.max(minimum_height).min(work_height)
        );
    }

    #[test]
    fn physical_rect_serde_round_trips(
        x in any::<i32>(),
        y in any::<i32>(),
        width in any::<u32>(),
        height in any::<u32>(),
    ) {
        let rect = PhysicalRect::new(
            PhysicalPoint::new(x, y),
            PhysicalSize::new(width, height),
        );
        let json = serde_json::to_string(&rect).unwrap();
        let decoded: PhysicalRect = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(decoded, rect);
    }

    #[test]
    fn translation_overflow_is_never_silent(
        x in any::<i32>(),
        y in any::<i32>(),
        dx in any::<i32>(),
        dy in any::<i32>(),
    ) {
        let rect = ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(10, 10));
        let result = rect.checked_translate(&ScreenVector::new(dx, dy));

        match (x.checked_add(dx), y.checked_add(dy)) {
            (Some(expected_x), Some(expected_y)) => {
                prop_assert_eq!(
                    result.unwrap().origin(),
                    ScreenPoint::new(expected_x, expected_y)
                );
            }
            _ => prop_assert!(result.is_err()),
        }
    }

    #[test]
    fn minimum_visibility_is_met_when_requested(
        work_x in -10_000_i32..=10_000,
        work_y in -10_000_i32..=10_000,
        work_width in 1_u32..=4000,
        work_height in 1_u32..=4000,
        window_x in -20_000_i32..=20_000,
        window_y in -20_000_i32..=20_000,
        window_width in 1_u32..=8000,
        window_height in 1_u32..=8000,
        requested_width in 0_u32..=8000,
        requested_height in 0_u32..=8000,
    ) {
        let work = ScreenRect::new(
            ScreenPoint::new(work_x, work_y),
            ScreenSize::new(work_width, work_height),
        );
        let window = ScreenRect::new(
            ScreenPoint::new(window_x, window_y),
            ScreenSize::new(window_width, window_height),
        );
        let requested = ScreenSize::new(requested_width, requested_height);
        let visible = window.ensure_minimum_visible(&work, &requested).unwrap();
        let intersection = visible.intersection(&work);
        let expected_width = requested_width.min(window_width).min(work_width);
        let expected_height = requested_height.min(window_height).min(work_height);

        prop_assert_eq!(visible.size(), window.size());
        if expected_width > 0 && expected_height > 0 {
            let intersection = intersection.unwrap();
            prop_assert!(intersection.size().width() >= expected_width);
            prop_assert!(intersection.size().height() >= expected_height);
        }
    }
}
