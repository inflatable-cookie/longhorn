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
    TransferHostError,
};

trait ErasedSurfaceTransferCommands: Send + Sync {
    fn start_surface(
        &self,
        caller: &HostWindowHandle,
        request: SurfaceSessionStartRequest,
    ) -> Result<SurfaceSessionResponse, TransferHandlerError>;

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

impl<R, C, S> AssembledSurfaceTransferCommands<R, C, S>
where
    R: ManagedTransferRuntime,
    C: MonotonicClock,
    S: SurfaceTransferAdapter,
{
    /// Admits one current whole-Surface transfer.
    pub fn start_surface(
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

    /// Commits one terminal whole-Surface transfer.
    pub fn commit_surface(
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

impl<R, C, S> ErasedSurfaceTransferCommands for AssembledSurfaceTransferCommands<R, C, S>
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
        Self::start_surface(self, caller, request)
    }

    fn commit_surface(
        &self,
        caller: &HostWindowHandle,
        request: SurfaceTransferCommand,
    ) -> Result<SurfaceTransferResponse, TransferHandlerError> {
        Self::commit_surface(self, caller, request)
    }
}

/// Type-erased optional Surface commands installed in Tauri managed state.
pub struct TauriSurfaceTransferState {
    commands: Arc<dyn ErasedSurfaceTransferCommands>,
}

impl TauriSurfaceTransferState {
    /// Wraps optional Surface commands over shared handler authority.
    #[must_use]
    pub fn new<R, C, S>(commands: Arc<AssembledSurfaceTransferCommands<R, C, S>>) -> Self
    where
        R: ManagedTransferRuntime + 'static,
        C: MonotonicClock + Send + Sync + 'static,
        S: SurfaceTransferAdapter + Send + 'static,
    {
        Self { commands }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriSurfaceTransferState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &dyn ErasedSurfaceTransferCommands,
        &HostWindowHandle,
    ) -> Result<T, TransferHandlerError>
    + Send
    + 'static,
) -> Result<T, TransferHostError> {
    let caller = caller_handle(window)?;
    let commands = Arc::clone(&state.commands);
    tauri::async_runtime::spawn_blocking(move || action(commands.as_ref(), &caller))
        .await
        .map_err(|_| TransferHostError::state_unavailable())?
        .map_err(TransferHostError::from)
}

/// Admits one current whole-Surface transfer session.
#[tauri::command]
pub async fn longhorn_transfer_start_surface<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSurfaceTransferState>,
    request: SurfaceSessionStartRequest,
) -> Result<SurfaceSessionResponse, TransferHostError> {
    run_blocking(state, &window, move |commands, caller| {
        commands.start_surface(caller, request)
    })
    .await
}

/// Commits one terminal whole-Surface transfer.
#[tauri::command]
pub async fn longhorn_transfer_commit_surface<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSurfaceTransferState>,
    request: SurfaceTransferCommand,
) -> Result<SurfaceTransferResponse, TransferHostError> {
    run_blocking(state, &window, move |commands, caller| {
        commands.commit_surface(caller, request)
    })
    .await
}
