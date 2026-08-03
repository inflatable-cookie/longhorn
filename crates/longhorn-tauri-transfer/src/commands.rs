use std::sync::{Arc, Mutex};

use longhorn_transfer::{
    MonotonicClock, PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse,
    TransferCancelRequest, TransferCancelResponse, TransferClientSnapshot, TransferLeaseRequest,
    TransferLeaseResponse, TransferSessionResponse,
};
use longhorn_windowing::HostWindowHandle;
use tauri::{Emitter, Runtime, State, WebviewWindow};

use crate::{
    ManagedTransferRuntime, PanelTransferAdapter, TransferHandlerAssembly, TransferHandlerError,
};

/// Window-local event carrying newly issued renderer transfer authority.
pub const TRANSFER_CLIENT_CHANGED_EVENT: &str = "longhorn://transfer/client-changed";

#[cfg(feature = "surface-transfer")]
mod surface;

#[cfg(feature = "surface-transfer")]
pub use surface::{
    AssembledSurfaceTransferCommands, SurfaceTransferCommandService, TauriSurfaceTransferState,
    longhorn_transfer_commit_surface, longhorn_transfer_start_surface,
};

/// Object-safe command surface retained in Tauri managed state.
pub trait TransferCommandService: Send + Sync {
    /// Registers one fresh caller renderer epoch.
    fn snapshot(
        &self,
        caller: &HostWindowHandle,
    ) -> Result<TransferClientSnapshot, TransferHandlerError>;

    /// Admits one current panel transfer session.
    fn start_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelSessionStartRequest,
    ) -> Result<TransferSessionResponse, TransferHandlerError>;

    /// Publishes one complete caller-window zone replacement.
    fn publish_lease(
        &self,
        caller: &HostWindowHandle,
        request: TransferLeaseRequest,
    ) -> Result<TransferLeaseResponse, TransferHandlerError>;

    /// Commits one terminal panel transfer.
    fn commit_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelTransferCommand,
    ) -> Result<PanelTransferResponse, TransferHandlerError>;

    /// Cancels one bounded transfer session.
    fn cancel(
        &self,
        caller: &HostWindowHandle,
        request: TransferCancelRequest,
    ) -> Result<TransferCancelResponse, TransferHandlerError>;
}

/// One command service combining shared handler state and one panel adapter.
pub struct AssembledTransferCommands<R, C, P> {
    handler: Arc<TransferHandlerAssembly<R, C>>,
    panel: Mutex<P>,
}

impl<R, C, P> AssembledTransferCommands<R, C, P> {
    /// Assembles one command path for real or mock managed-window runtime.
    #[must_use]
    pub fn new(handler: TransferHandlerAssembly<R, C>, panel: P) -> Self {
        Self {
            handler: Arc::new(handler),
            panel: Mutex::new(panel),
        }
    }

    /// Assembles panel commands over existing shared handler state.
    #[must_use]
    pub const fn from_shared(handler: Arc<TransferHandlerAssembly<R, C>>, panel: P) -> Self {
        Self {
            handler,
            panel: Mutex::new(panel),
        }
    }

    /// Returns the shared handler core for trusted lifecycle integration.
    #[must_use]
    pub fn handler(&self) -> &TransferHandlerAssembly<R, C> {
        &self.handler
    }

    /// Clones shared handler state for optional peer command adapters.
    #[must_use]
    pub fn shared_handler(&self) -> Arc<TransferHandlerAssembly<R, C>> {
        self.handler.clone()
    }
}

impl<R, C, P> TransferCommandService for AssembledTransferCommands<R, C, P>
where
    R: ManagedTransferRuntime + 'static,
    C: MonotonicClock + Send + Sync + 'static,
    P: PanelTransferAdapter + Send + 'static,
{
    fn snapshot(
        &self,
        caller: &HostWindowHandle,
    ) -> Result<TransferClientSnapshot, TransferHandlerError> {
        self.handler.snapshot(caller)
    }

    fn start_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelSessionStartRequest,
    ) -> Result<TransferSessionResponse, TransferHandlerError> {
        let mut panel = self
            .panel
            .lock()
            .map_err(|_| TransferHandlerError::StateUnavailable)?;
        self.handler.start_panel(caller, request, &mut *panel)
    }

    fn publish_lease(
        &self,
        caller: &HostWindowHandle,
        request: TransferLeaseRequest,
    ) -> Result<TransferLeaseResponse, TransferHandlerError> {
        self.handler.publish_lease(caller, request)
    }

    fn commit_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelTransferCommand,
    ) -> Result<PanelTransferResponse, TransferHandlerError> {
        let mut panel = self
            .panel
            .lock()
            .map_err(|_| TransferHandlerError::StateUnavailable)?;
        self.handler.commit_panel(caller, request, &mut *panel)
    }

    fn cancel(
        &self,
        caller: &HostWindowHandle,
        request: TransferCancelRequest,
    ) -> Result<TransferCancelResponse, TransferHandlerError> {
        self.handler.cancel(caller, request)
    }
}

/// Type-erased transfer commands installed once in Tauri managed state.
pub struct TauriTransferState {
    service: Arc<dyn TransferCommandService>,
    // Serializes snapshot acquisition with client-changed emission so two
    // concurrent snapshots cannot deliver their events out of epoch order.
    snapshot_emit: std::sync::Mutex<()>,
}

impl TauriTransferState {
    /// Wraps one real or mock-runtime command assembly.
    #[must_use]
    pub fn new(service: Arc<dyn TransferCommandService>) -> Self {
        Self {
            service,
            snapshot_emit: std::sync::Mutex::new(()),
        }
    }
}

/// Returns one fresh caller transfer-client snapshot.
#[tauri::command]
pub fn longhorn_transfer_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
) -> Result<TransferClientSnapshot, String> {
    let _ordered = state
        .snapshot_emit
        .lock()
        .map_err(|_| "transfer snapshot ordering lock is poisoned".to_string())?;
    let snapshot = state
        .service
        .snapshot(&caller_handle(&window)?)
        .map_err(|error| error.to_string())?;
    // The invoke result is the authoritative delivery; the event is advisory
    // to other listeners. The epoch has already advanced, so an emit failure
    // must not hide the new authority behind an error.
    if let Err(error) = window.emit(TRANSFER_CLIENT_CHANGED_EVENT, &snapshot) {
        longhorn_core::report_best_effort_failure("transfer.client-changed-emit", error);
    }
    Ok(snapshot)
}

/// Admits one current panel transfer session.
#[tauri::command]
pub fn longhorn_transfer_start_panel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: PanelSessionStartRequest,
) -> Result<TransferSessionResponse, String> {
    state
        .service
        .start_panel(&caller_handle(&window)?, request)
        .map_err(|error| error.to_string())
}

/// Publishes one complete caller-window drop-zone lease.
#[tauri::command]
pub fn longhorn_transfer_publish_lease<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: TransferLeaseRequest,
) -> Result<TransferLeaseResponse, String> {
    state
        .service
        .publish_lease(&caller_handle(&window)?, request)
        .map_err(|error| error.to_string())
}

/// Commits one terminal panel transfer.
#[tauri::command]
pub fn longhorn_transfer_commit_panel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: PanelTransferCommand,
) -> Result<PanelTransferResponse, String> {
    state
        .service
        .commit_panel(&caller_handle(&window)?, request)
        .map_err(|error| error.to_string())
}

/// Cancels one bounded transfer session.
#[tauri::command]
pub fn longhorn_transfer_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: TransferCancelRequest,
) -> Result<TransferCancelResponse, String> {
    state
        .service
        .cancel(&caller_handle(&window)?, request)
        .map_err(|error| error.to_string())
}

fn caller_handle<R: Runtime>(window: &WebviewWindow<R>) -> Result<HostWindowHandle, String> {
    HostWindowHandle::new(window.label()).map_err(|error| error.to_string())
}
