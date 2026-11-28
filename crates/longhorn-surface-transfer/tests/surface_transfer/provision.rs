use longhorn_core::{DisplayId, ScreenPoint, ScreenRect, ScreenSize, WindowPlacement};
use longhorn_surface_transfer::{
    EmptyDisplayProvisionPolicy, EmptyDisplayProvisionTarget, SurfaceTransferPolicy,
};
use longhorn_surfaces::EmptyWindowPolicy;

use super::support::window_id;

#[test]
fn enabled_provisioning_requires_at_least_one_target() {
    let error = SurfaceTransferPolicy::new(
        [window_id("window:a")],
        EmptyWindowPolicy::Allow,
        EmptyDisplayProvisionPolicy::Enabled(vec![]),
    )
    .unwrap_err();
    assert!(error.detail().contains("at least one"));
}

#[test]
fn display_policy_rejects_overlapping_bounds_and_accepts_shared_edges() {
    let policy = |second_x| {
        let target = |display, window, x| {
            EmptyDisplayProvisionTarget::new(
                DisplayId::new(display).unwrap(),
                ScreenRect::new(ScreenPoint::new(x, 0), ScreenSize::new(100, 100)),
                window_id(window),
                WindowPlacement::new(ScreenPoint::new(x, 0), ScreenSize::new(80, 80)),
                None,
            )
        };
        SurfaceTransferPolicy::new(
            [window_id("window:a"), window_id("window:b")],
            EmptyWindowPolicy::Allow,
            EmptyDisplayProvisionPolicy::Enabled(vec![
                target("display:a", "window:a", 0),
                target("display:b", "window:b", second_x),
            ]),
        )
    };
    let error = policy(99).unwrap_err();
    assert!(error.detail().contains("display:a"));
    assert!(error.detail().contains("display:b"));
    assert!(error.detail().contains("overlap"));
    assert!(policy(100).is_ok());
}

#[test]
fn policy_rejects_zero_geometry_and_duplicate_display_or_window_targets() {
    let valid = |display: &str, window: &str| {
        EmptyDisplayProvisionTarget::new(
            DisplayId::new(display).unwrap(),
            ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(100, 100)),
            window_id(window),
            WindowPlacement::new(ScreenPoint::new(0, 0), ScreenSize::new(80, 80)),
            None,
        )
    };
    assert!(
        SurfaceTransferPolicy::new(
            [window_id("window:a"), window_id("window:b")],
            EmptyWindowPolicy::Allow,
            EmptyDisplayProvisionPolicy::Enabled(vec![
                valid("display:a", "window:a"),
                valid("display:a", "window:b"),
            ]),
        )
        .is_err()
    );
    assert!(
        SurfaceTransferPolicy::new(
            [window_id("window:a")],
            EmptyWindowPolicy::Allow,
            EmptyDisplayProvisionPolicy::Enabled(vec![
                valid("display:a", "window:a"),
                valid("display:b", "window:a"),
            ]),
        )
        .is_err()
    );
    let disallowed = SurfaceTransferPolicy::new(
        [window_id("window:allowed")],
        EmptyWindowPolicy::Allow,
        EmptyDisplayProvisionPolicy::Enabled(vec![valid("display:a", "window:not-allowed")]),
    )
    .unwrap_err();
    assert!(disallowed.detail().contains("allowed_target_windows"));
}
