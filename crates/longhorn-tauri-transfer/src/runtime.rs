use std::{collections::BTreeMap, sync::Arc};

use longhorn_core::{PhysicalPx, RoundingMode, ScreenPoint, ScreenRect};
use longhorn_tauri_windowing::{
    DesktopCoordinateMapper, ManagedWebviewWindow, MappedWindowGeometry, PhysicalDesktopSnapshot,
    TauriWindowHost, probe_managed_windows,
};
use longhorn_windowing::HostWindowHandle;
use tauri::Runtime;

use crate::{ManagedTransferSnapshot, ManagedTransferWindow, TransferRuntimeError};

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
            .map_err(|error| TransferRuntimeError::WindowHost(format!("{error:?}")))?;
        let inner_positions = probe_inner_positions(&managed)?;
        let physical = probe_managed_windows(&managed)
            .map_err(|error| TransferRuntimeError::Probe(error.to_string()))?;
        let raw = PhysicalDesktopSnapshot::new([], physical.iter().cloned());
        let mapped = self
            .mapper
            .map_desktop(&raw)
            .map_err(|error| TransferRuntimeError::Mapping(error.to_string()))?;
        let mapped = mapped
            .windows()
            .iter()
            .map(|window| (window.transport_handle().clone(), window.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut windows = Vec::with_capacity(physical.len());
        for (managed, raw) in managed.iter().zip(physical) {
            let handle = raw.transport_handle().clone();
            let window_id = managed
                .window_id()
                .cloned()
                .ok_or_else(|| TransferRuntimeError::MissingWindowId(handle.clone()))?;
            let mapped = mapped
                .get(&handle)
                .ok_or_else(|| TransferRuntimeError::MissingMappedWindow(handle.clone()))?;
            let inner_position = inner_positions
                .get(&handle)
                .expect("inner position exists for every probed managed window");
            let content_origin = project_content_origin(&handle, &raw, mapped, *inner_position)?;
            windows.push(ManagedTransferWindow::new(
                window_id,
                handle,
                mapped.outer_bounds(),
                ScreenRect::new(content_origin, mapped.inner_size()),
            ));
        }
        ManagedTransferSnapshot::new(caller_handle, windows)
    }
}

fn probe_inner_positions<R: Runtime>(
    managed: &[ManagedWebviewWindow<R>],
) -> Result<BTreeMap<HostWindowHandle, (i32, i32)>, TransferRuntimeError> {
    managed
        .iter()
        .map(|managed| {
            let window = managed.webview_window();
            let handle = HostWindowHandle::new(window.label())
                .expect("managed-window host already validated Tauri labels");
            window
                .inner_position()
                .map(|position| (handle.clone(), (position.x, position.y)))
                .map_err(|error| TransferRuntimeError::HostCall {
                    handle,
                    operation: "inner_position",
                    detail: error.to_string(),
                })
        })
        .collect()
}

fn project_content_origin(
    handle: &HostWindowHandle,
    raw: &longhorn_tauri_windowing::PhysicalLiveWindowObservation,
    mapped: &MappedWindowGeometry,
    inner_position: (i32, i32),
) -> Result<ScreenPoint, TransferRuntimeError> {
    let outer = raw.outer_bounds().origin();
    let dx = inner_position
        .0
        .checked_sub(outer.x().get())
        .ok_or_else(|| TransferRuntimeError::ContentOriginOverflow(handle.clone()))?;
    let dy = inner_position
        .1
        .checked_sub(outer.y().get())
        .ok_or_else(|| TransferRuntimeError::ContentOriginOverflow(handle.clone()))?;
    let dx = raw
        .scale()
        .physical_to_screen(PhysicalPx::new(dx), RoundingMode::Nearest)
        .map_err(|_| TransferRuntimeError::ContentOriginOverflow(handle.clone()))?;
    let dy = raw
        .scale()
        .physical_to_screen(PhysicalPx::new(dy), RoundingMode::Nearest)
        .map_err(|_| TransferRuntimeError::ContentOriginOverflow(handle.clone()))?;
    let outer = mapped.outer_bounds().origin();
    let x = outer
        .x()
        .get()
        .checked_add(dx.get())
        .ok_or_else(|| TransferRuntimeError::ContentOriginOverflow(handle.clone()))?;
    let y = outer
        .y()
        .get()
        .checked_add(dy.get())
        .ok_or_else(|| TransferRuntimeError::ContentOriginOverflow(handle.clone()))?;
    Ok(ScreenPoint::new(x, y))
}
