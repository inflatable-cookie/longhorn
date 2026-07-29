use std::sync::{Arc, Mutex};

use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferCommand,
    SurfaceTransferResponse,
};
use longhorn_transfer::MonotonicClock;
use longhorn_windowing::HostWindowHandle;
use tauri::{Runtime, State, WebviewWindow};

use super::caller_handle;
use crate::{
    ManagedTransferRuntime, SurfaceTransferAdapter, TransferHandlerAssembly, TransferHandlerError,
};

/// Object-safe optional whole-Surface command surface.
pub trait SurfaceTransferCommandService: Send + Sync {
    /// Admits one current whole-Surface transfer.
    fn start_surface(
        &self,
        caller: &HostWindowHandle,
        request: SurfaceSessionStartRequest,
    ) -> Result<SurfaceSessionResponse, TransferHandlerError>;

    /// Commits one terminal whole-Surface transfer.
    fn commit_surface(
        &self,
        caller: &HostWindowHandle,
        request: SurfaceTransferCommand,
    ) -> Result<SurfaceTransferResponse, TransferHandlerError>;
}

/// Optional Surface commands sharing the base handler's coordinator and epochs.
pub struct AssembledSurfaceTransferCommands<R, C, S> {
    handler: Arc<TransferHandlerAssembly<R, C>>,
    surface: Mutex<S>,
}

impl<R, C, S> AssembledSurfaceTransferCommands<R, C, S> {
    /// Assembles optional Surface commands over shared base handler state.
    #[must_use]
    pub const fn new(handler: Arc<TransferHandlerAssembly<R, C>>, surface: S) -> Self {
        Self {
            handler,
            surface: Mutex::new(surface),
        }
    }
}

impl<R, C, S> SurfaceTransferCommandService for AssembledSurfaceTransferCommands<R, C, S>
where
    R: ManagedTransferRuntime + 'static,
    C: MonotonicClock + Send + Sync + 'static,
    S: SurfaceTransferAdapter + Send + 'static,
{
    fn start_surface(
        &self,
        caller: &HostWindowHandle,
        request: SurfaceSessionStartRequest,
    ) -> Result<SurfaceSessionResponse, TransferHandlerError> {
        let mut surface = self
            .surface
            .lock()
            .map_err(|_| TransferHandlerError::StateUnavailable)?;
        self.handler.start_surface(caller, request, &mut *surface)
    }

    fn commit_surface(
        &self,
        caller: &HostWindowHandle,
        request: SurfaceTransferCommand,
    ) -> Result<SurfaceTransferResponse, TransferHandlerError> {
        let mut surface = self
            .surface
            .lock()
            .map_err(|_| TransferHandlerError::StateUnavailable)?;
        self.handler.commit_surface(caller, request, &mut *surface)
    }
}

/// Type-erased optional Surface commands installed in Tauri managed state.
pub struct TauriSurfaceTransferState {
    service: Arc<dyn SurfaceTransferCommandService>,
}

impl TauriSurfaceTransferState {
    /// Wraps optional Surface commands over shared handler authority.
    #[must_use]
    pub fn new(service: Arc<dyn SurfaceTransferCommandService>) -> Self {
        Self { service }
    }
}

/// Admits one current whole-Surface transfer session.
#[tauri::command]
pub fn longhorn_transfer_start_surface<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSurfaceTransferState>,
    request: SurfaceSessionStartRequest,
) -> Result<SurfaceSessionResponse, String> {
    state
        .service
        .start_surface(&caller_handle(&window)?, request)
        .map_err(|error| error.to_string())
}

/// Commits one terminal whole-Surface transfer.
#[tauri::command]
pub fn longhorn_transfer_commit_surface<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSurfaceTransferState>,
    request: SurfaceTransferCommand,
) -> Result<SurfaceTransferResponse, String> {
    state
        .service
        .commit_surface(&caller_handle(&window)?, request)
        .map_err(|error| error.to_string())
}
