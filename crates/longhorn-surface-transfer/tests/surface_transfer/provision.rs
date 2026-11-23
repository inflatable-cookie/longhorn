use longhorn_core::{DisplayId, ScreenPoint, ScreenRect, ScreenSize, WindowPlacement};
use longhorn_surface_transfer::{
    EmptyDisplayProvisionPolicy, EmptyDisplayProvisionTarget, SurfaceTransferPolicy,
};
use longhorn_surfaces::EmptyWindowPolicy;

use super::support::window_id;

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
