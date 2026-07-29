use longhorn_core::{
    LiveWindowMetrics, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement,
};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapabilities, HostWindowHandle, LiveWindow,
    WindowDiffInput, WindowDiffReceipt, WindowOperationKind, plan_window_diff,
};

pub(super) fn id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub(super) fn handle(value: &str) -> HostWindowHandle {
    HostWindowHandle::new(value).unwrap()
}

pub(super) fn placement(x: i32, y: i32, width: u32, height: u32) -> WindowPlacement {
    WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub(super) fn desired(
    window_id: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
    visible: bool,
) -> DesiredWindow {
    DesiredWindow::new(
        id(window_id),
        placement(x, y, width, height),
        maximized,
        visible,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live(
    window_id: Option<&str>,
    transport_handle: &str,
    x: i32,
    y: i32,
    outer_width: u32,
    outer_height: u32,
    inner_width: u32,
    inner_height: u32,
    maximized: bool,
    visible: bool,
    focused: bool,
) -> LiveWindow {
    LiveWindow::new(
        window_id.map(id),
        handle(transport_handle),
        LiveWindowMetrics::new(
            ScreenRect::new(
                ScreenPoint::new(x, y),
                ScreenSize::new(outer_width, outer_height),
            ),
            ScreenSize::new(inner_width, inner_height),
        ),
        maximized,
        visible,
        focused,
    )
}

pub(super) fn input(
    desired_windows: impl IntoIterator<Item = DesiredWindow>,
    live_windows: impl IntoIterator<Item = LiveWindow>,
) -> WindowDiffInput {
    WindowDiffInput::new(
        desired_windows,
        live_windows,
        HostCapabilities::all(),
        ApplyGeneration::new(42),
    )
}

pub(super) fn plan(input: &WindowDiffInput) -> WindowDiffReceipt {
    plan_window_diff(input).unwrap()
}

pub(super) fn kinds(receipt: &WindowDiffReceipt) -> Vec<WindowOperationKind> {
    receipt
        .operations()
        .iter()
        .map(|operation| operation.operation().kind())
        .collect()
}
