use std::sync::Arc;

use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferCommand,
    SurfaceTransferResponse,
};
use longhorn_transfer::MonotonicClock;
use longhorn_windowing::HostWindowHandle;
use tauri::{Runtime, State, WebviewWindow};

use super::run_blocking;
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
    surface: S,
}

impl<R, C, S> AssembledSurfaceTransferCommands<R, C, S> {
    /// Assembles optional Surface commands over shared base handler state.
    #[must_use]
    pub const fn new(handler: Arc<TransferHandlerAssembly<R, C>>, surface: S) -> Self {
        Self { handler, surface }
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
        self.handler.start_surface(caller, request, &self.surface)
    }

    /// Commits one terminal whole-Surface transfer.
    pub fn commit_surface(
        &self,
        caller: &HostWindowHandle,
        request: SurfaceTransferCommand,
    ) -> Result<SurfaceTransferResponse, TransferHandlerError> {
        self.handler.commit_surface(caller, request, &self.surface)
    }
}

impl<R, C, S> ErasedSurfaceTransferCommands for AssembledSurfaceTransferCommands<R, C, S>
where
    R: ManagedTransferRuntime + 'static,
    C: MonotonicClock + Send + Sync + 'static,
    S: SurfaceTransferAdapter + Send + Sync + 'static,
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
        S: SurfaceTransferAdapter + Send + Sync + 'static,
    {
        Self { commands }
    }
}

/// Admits one current whole-Surface transfer session.
#[tauri::command]
pub async fn longhorn_transfer_start_surface<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSurfaceTransferState>,
    request: SurfaceSessionStartRequest,
) -> Result<SurfaceSessionResponse, TransferHostError> {
    run_blocking(&state.commands, &window, move |commands, caller| {
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
    run_blocking(&state.commands, &window, move |commands, caller| {
        commands.commit_surface(caller, request)
    })
    .await
}
