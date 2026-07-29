use longhorn_core::{
    PhysicalPoint, PhysicalRect, PhysicalSize, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize,
    WindowId,
};
use longhorn_display::{DisplayBuiltinStatus, DisplayEvidence, ObservationId};
use longhorn_tauri_windowing::{
    DisplayObservationMetadata, MappedDisplayGeometry, MappedWindowGeometry,
    PhysicalDisplayObservation, PhysicalLiveWindowObservation, PhysicalMonitorFacts,
};
use longhorn_windowing::HostWindowHandle;

pub(super) fn physical_rect(x: i32, y: i32, width: u32, height: u32) -> PhysicalRect {
    PhysicalRect::new(PhysicalPoint::new(x, y), PhysicalSize::new(width, height))
}

pub(super) fn screen_rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn display(
    id: &str,
    label: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    work_y: i32,
    work_height: u32,
    scale: u32,
    main: bool,
    builtin: DisplayBuiltinStatus,
) -> PhysicalDisplayObservation {
    PhysicalDisplayObservation::new(
        PhysicalMonitorFacts::new(
            Some(label.to_string()),
            main,
            physical_rect(x, y, width, height),
            physical_rect(x, work_y, width, work_height),
            ScaleFactor::from_thousandths(scale).unwrap(),
        ),
        DisplayObservationMetadata::new(
            ObservationId::new(id).unwrap(),
            builtin,
            DisplayEvidence::new(),
        ),
    )
}

pub(super) fn window(
    id: &str,
    handle: &str,
    outer: PhysicalRect,
    inner: PhysicalSize,
    scale: u32,
) -> PhysicalLiveWindowObservation {
    PhysicalLiveWindowObservation::new(
        Some(WindowId::new(id).unwrap()),
        HostWindowHandle::new(handle).unwrap(),
        outer,
        inner,
        ScaleFactor::from_thousandths(scale).unwrap(),
        false,
        true,
        false,
    )
}

pub(super) fn mapped_display(
    id: &str,
    full: ScreenRect,
    work: ScreenRect,
) -> MappedDisplayGeometry {
    MappedDisplayGeometry::new(ObservationId::new(id).unwrap(), full, work)
}

pub(super) fn mapped_window(
    handle: &str,
    outer: ScreenRect,
    inner: ScreenSize,
) -> MappedWindowGeometry {
    MappedWindowGeometry::new(HostWindowHandle::new(handle).unwrap(), outer, inner)
}
