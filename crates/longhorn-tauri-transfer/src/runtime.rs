use std::{collections::BTreeMap, sync::Arc};

use longhorn_core::{PhysicalPoint, PhysicalPx, RoundingMode, ScreenPoint, ScreenRect};
use longhorn_tauri_windowing::{
    DesktopCoordinateMapper, ManagedWebviewWindow, MappedWindowGeometry, PhysicalDesktopSnapshot,
    PhysicalLiveWindowObservation, TauriWindowHost, probe_managed_windows,
};
use longhorn_windowing::HostWindowHandle;
use tauri::Runtime;

use crate::{
    ManagedTransferSnapshot, ManagedTransferWindow, TransferHostOperation, TransferRuntimeError,
};

/// Runtime seam used by the shared transfer handler assembly and its mocks.
pub trait ManagedTransferRuntime: Send + Sync {
    /// Reads one coherent current managed-window snapshot for the caller.
    fn snapshot(
        &self,
        caller_handle: &HostWindowHandle,
    ) -> Result<ManagedTransferSnapshot, TransferRuntimeError>;
}

impl<F> ManagedTransferRuntime for F
where
    F: Fn(&HostWindowHandle) -> Result<ManagedTransferSnapshot, TransferRuntimeError> + Send + Sync,
{
    fn snapshot(
        &self,
        caller_handle: &HostWindowHandle,
    ) -> Result<ManagedTransferSnapshot, TransferRuntimeError> {
        self(caller_handle)
    }
}

/// Real Tauri readback backed by the shared managed-window host and mapper.
pub struct TauriTransferRuntime<R: Runtime, M> {
    window_host: Arc<TauriWindowHost<R>>,
    mapper: M,
}

impl<R: Runtime, M> TauriTransferRuntime<R, M> {
    /// Binds transfer readback to the existing managed-window authority.
    #[must_use]
    pub const fn new(window_host: Arc<TauriWindowHost<R>>, mapper: M) -> Self {
        Self {
            window_host,
            mapper,
        }
    }
}

impl<R, M> ManagedTransferRuntime for TauriTransferRuntime<R, M>
where
    R: Runtime,
    M: DesktopCoordinateMapper + Send + Sync,
{
    fn snapshot(
        &self,
        caller_handle: &HostWindowHandle,
    ) -> Result<ManagedTransferSnapshot, TransferRuntimeError> {
        let managed = self
            .window_host
            .managed_windows()
            .map_err(|error| TransferRuntimeError::WindowHost(error.to_string()))?;
        let inner_positions = probe_inner_positions(&managed)?;
        let physical = probe_managed_windows(&managed)
            .map_err(|error| TransferRuntimeError::Probe(error.to_string()))?;
        map_transfer_snapshot(
            caller_handle,
            &self.mapper,
            &PhysicalDesktopSnapshot::new([], physical),
            &inner_positions,
        )
    }
}

fn map_transfer_snapshot(
    caller_handle: &HostWindowHandle,
    mapper: &impl DesktopCoordinateMapper,
    physical: &PhysicalDesktopSnapshot,
    inner_positions: &BTreeMap<HostWindowHandle, PhysicalPoint>,
) -> Result<ManagedTransferSnapshot, TransferRuntimeError> {
    let geometry = mapper
        .map_desktop(physical)
        .map_err(|error| TransferRuntimeError::Mapping(error.to_string()))?;
    let mapped = geometry
        .windows()
        .iter()
        .map(|window| (window.transport_handle(), window))
        .collect::<BTreeMap<_, _>>();

    let mut windows = Vec::with_capacity(physical.windows().len());
    for raw in physical.windows() {
        let handle = raw.transport_handle();
        let window_id = raw
            .window_id()
            .cloned()
            .ok_or_else(|| TransferRuntimeError::MissingWindowId(handle.clone()))?;
        let mapped = mapped
            .get(handle)
            .ok_or_else(|| TransferRuntimeError::MissingMappedWindow(handle.clone()))?;
        let inner_position = inner_positions
            .get(handle)
            .expect("inner position exists for every probed managed window");
        let content_origin = project_content_origin(raw, mapped, *inner_position)?;
        windows.push(ManagedTransferWindow::new(
            window_id,
            handle.clone(),
            mapped.outer_bounds(),
            ScreenRect::new(content_origin, mapped.inner_size()),
        ));
    }
    ManagedTransferSnapshot::new(caller_handle, windows)
}

fn probe_inner_positions<R: Runtime>(
    managed: &[ManagedWebviewWindow<R>],
) -> Result<BTreeMap<HostWindowHandle, PhysicalPoint>, TransferRuntimeError> {
    managed
        .iter()
        .map(|managed| {
            let window = managed.webview_window();
            let handle = HostWindowHandle::new(window.label())
                .expect("managed-window host already validated Tauri labels");
            window
                .inner_position()
                .map(|position| (handle.clone(), PhysicalPoint::new(position.x, position.y)))
                .map_err(|error| TransferRuntimeError::HostCall {
                    handle,
                    operation: TransferHostOperation::InnerPosition,
                    detail: error.to_string(),
                })
        })
        .collect()
}

fn project_content_origin(
    raw: &PhysicalLiveWindowObservation,
    mapped: &MappedWindowGeometry,
    inner_position: PhysicalPoint,
) -> Result<ScreenPoint, TransferRuntimeError> {
    let overflow = || TransferRuntimeError::ContentOriginOverflow(raw.transport_handle().clone());
    let outer = raw.outer_bounds().origin();
    let dx = inner_position
        .x()
        .get()
        .checked_sub(outer.x().get())
        .ok_or_else(overflow)?;
    let dy = inner_position
        .y()
        .get()
        .checked_sub(outer.y().get())
        .ok_or_else(overflow)?;
    let dx = raw
        .scale()
        .physical_to_screen(PhysicalPx::new(dx), RoundingMode::Nearest)
        .map_err(|_| overflow())?;
    let dy = raw
        .scale()
        .physical_to_screen(PhysicalPx::new(dy), RoundingMode::Nearest)
        .map_err(|_| overflow())?;
    let outer = mapped.outer_bounds().origin();
    let x = outer.x().get().checked_add(dx.get()).ok_or_else(overflow)?;
    let y = outer.y().get().checked_add(dy.get()).ok_or_else(overflow)?;
    Ok(ScreenPoint::new(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use longhorn_core::{
        ClientPoint, PhysicalRect, PhysicalSize, ScaleFactor, ScreenSize, WindowId,
    };
    use longhorn_tauri_windowing::LogicalLayoutMapper;

    fn observation(
        handle: &HostWindowHandle,
        outer_origin: PhysicalPoint,
    ) -> PhysicalLiveWindowObservation {
        PhysicalLiveWindowObservation::new(
            Some(WindowId::new("window:desktop").unwrap()),
            handle.clone(),
            PhysicalRect::new(outer_origin, PhysicalSize::new(900, 750)),
            PhysicalSize::new(870, 705),
            ScaleFactor::from_thousandths(1500).unwrap(),
            false,
            true,
            false,
        )
    }

    #[test]
    fn native_geometry_maps_outer_frame_and_scales_the_content_offset() {
        let handle = HostWindowHandle::new("desktop").unwrap();
        let raw = observation(&handle, PhysicalPoint::new(-600, 301));
        let physical = PhysicalDesktopSnapshot::new([], [raw]);
        let inner_positions = BTreeMap::from([(handle.clone(), PhysicalPoint::new(-593, 314))]);
        let snapshot =
            map_transfer_snapshot(&handle, &LogicalLayoutMapper, &physical, &inner_positions)
                .unwrap();
        let window = snapshot.caller();

        assert_eq!(
            window.outer_bounds(),
            ScreenRect::new(ScreenPoint::new(-400, 201), ScreenSize::new(600, 500))
        );
        // The 7px and 13px offsets round to 5 and 9 DIP relative to the mapped frame.
        assert_eq!(
            window.content_bounds(),
            ScreenRect::new(ScreenPoint::new(-395, 210), ScreenSize::new(580, 470))
        );
        assert_eq!(
            crate::project_client_point(window, ClientPoint::new(10.0, 12.0).unwrap()),
            Ok(ScreenPoint::new(-385, 222))
        );
    }

    #[test]
    fn native_geometry_rejects_content_offset_overflow() {
        let handle = HostWindowHandle::new("desktop").unwrap();
        let raw = observation(&handle, PhysicalPoint::new(i32::MIN, 0));
        let physical = PhysicalDesktopSnapshot::new([], [raw]);
        let inner_positions = BTreeMap::from([(handle.clone(), PhysicalPoint::new(i32::MAX, 0))]);

        assert_eq!(
            map_transfer_snapshot(&handle, &LogicalLayoutMapper, &physical, &inner_positions),
            Err(TransferRuntimeError::ContentOriginOverflow(handle))
        );
    }
}
