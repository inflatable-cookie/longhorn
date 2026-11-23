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
    TransferHostError,
};

/// Window-local event carrying newly issued renderer transfer authority.
pub const TRANSFER_CLIENT_CHANGED_EVENT: &str = "longhorn://transfer/client-changed";

#[cfg(feature = "surface-transfer")]
mod surface;

#[cfg(feature = "surface-transfer")]
pub use surface::{
    AssembledSurfaceTransferCommands, TauriSurfaceTransferState, longhorn_transfer_commit_surface,
    longhorn_transfer_start_surface,
};

trait ErasedTransferCommands: Send + Sync {
    fn snapshot(
        &self,
        caller: &HostWindowHandle,
    ) -> Result<TransferClientSnapshot, TransferHandlerError>;

    fn start_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelSessionStartRequest,
    ) -> Result<TransferSessionResponse, TransferHandlerError>;

    fn publish_lease(
        &self,
        caller: &HostWindowHandle,
        request: TransferLeaseRequest,
    ) -> Result<TransferLeaseResponse, TransferHandlerError>;

    fn commit_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelTransferCommand,
    ) -> Result<PanelTransferResponse, TransferHandlerError>;

    fn cancel(
        &self,
        caller: &HostWindowHandle,
        request: TransferCancelRequest,
    ) -> Result<TransferCancelResponse, TransferHandlerError>;
}

/// One command assembly combining shared handler state and one panel adapter.
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

impl<R, C, P> AssembledTransferCommands<R, C, P>
where
    R: ManagedTransferRuntime,
    C: MonotonicClock,
    P: PanelTransferAdapter,
{
    /// Registers one fresh caller renderer epoch.
    pub fn snapshot(
        &self,
        caller: &HostWindowHandle,
    ) -> Result<TransferClientSnapshot, TransferHandlerError> {
        self.handler.snapshot(caller)
    }

    /// Admits one current panel transfer session.
    pub fn start_panel(
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

    /// Publishes one complete caller-window zone replacement.
    pub fn publish_lease(
        &self,
        caller: &HostWindowHandle,
        request: TransferLeaseRequest,
    ) -> Result<TransferLeaseResponse, TransferHandlerError> {
        self.handler.publish_lease(caller, request)
    }

    /// Commits one terminal panel transfer.
    pub fn commit_panel(
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

    /// Cancels one bounded transfer session.
    pub fn cancel(
        &self,
        caller: &HostWindowHandle,
        request: TransferCancelRequest,
    ) -> Result<TransferCancelResponse, TransferHandlerError> {
        self.handler.cancel(caller, request)
    }
}

impl<R, C, P> ErasedTransferCommands for AssembledTransferCommands<R, C, P>
where
    R: ManagedTransferRuntime + 'static,
    C: MonotonicClock + Send + Sync + 'static,
    P: PanelTransferAdapter + Send + 'static,
{
    fn snapshot(
        &self,
        caller: &HostWindowHandle,
    ) -> Result<TransferClientSnapshot, TransferHandlerError> {
        Self::snapshot(self, caller)
    }

    fn start_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelSessionStartRequest,
    ) -> Result<TransferSessionResponse, TransferHandlerError> {
        Self::start_panel(self, caller, request)
    }

    fn publish_lease(
        &self,
        caller: &HostWindowHandle,
        request: TransferLeaseRequest,
    ) -> Result<TransferLeaseResponse, TransferHandlerError> {
        Self::publish_lease(self, caller, request)
    }

    fn commit_panel(
        &self,
        caller: &HostWindowHandle,
        request: PanelTransferCommand,
    ) -> Result<PanelTransferResponse, TransferHandlerError> {
        Self::commit_panel(self, caller, request)
    }

    fn cancel(
        &self,
        caller: &HostWindowHandle,
        request: TransferCancelRequest,
    ) -> Result<TransferCancelResponse, TransferHandlerError> {
        Self::cancel(self, caller, request)
    }
}

/// Type-erased transfer commands installed once in Tauri managed state.
pub struct TauriTransferState {
    commands: Arc<dyn ErasedTransferCommands>,
    // Serializes snapshot acquisition with client-changed emission so two
    // concurrent snapshots cannot deliver their events out of epoch order.
    snapshot_emit: Arc<Mutex<()>>,
}

impl TauriTransferState {
    /// Wraps one real or mock-runtime command assembly.
    #[must_use]
    pub fn new<R, C, P>(commands: Arc<AssembledTransferCommands<R, C, P>>) -> Self
    where
        R: ManagedTransferRuntime + 'static,
        C: MonotonicClock + Send + Sync + 'static,
        P: PanelTransferAdapter + Send + 'static,
    {
        Self {
            commands,
            snapshot_emit: Arc::new(Mutex::new(())),
        }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriTransferState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &dyn ErasedTransferCommands,
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

/// Returns one fresh caller transfer-client snapshot.
#[tauri::command]
pub async fn longhorn_transfer_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
) -> Result<TransferClientSnapshot, TransferHostError> {
    let caller = caller_handle(&window)?;
    let commands = Arc::clone(&state.commands);
    let snapshot_emit = Arc::clone(&state.snapshot_emit);
    let emit_window = window.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _ordered = snapshot_emit
            .lock()
            .map_err(|_| TransferHostError::state_unavailable())?;
        let snapshot = commands
            .snapshot(&caller)
            .map_err(TransferHostError::from)?;
        // The invoke result is the authoritative delivery; the event is advisory
        // to other listeners. The epoch has already advanced, so an emit failure
        // must not hide the new authority behind an error.
        if let Err(error) = emit_window.emit(TRANSFER_CLIENT_CHANGED_EVENT, &snapshot) {
            longhorn_core::report_best_effort_failure("transfer.client-changed-emit", error);
        }
        Ok(snapshot)
    })
    .await
    .map_err(|_| TransferHostError::state_unavailable())?
}

/// Admits one current panel transfer session.
#[tauri::command]
pub async fn longhorn_transfer_start_panel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: PanelSessionStartRequest,
) -> Result<TransferSessionResponse, TransferHostError> {
    run_blocking(state, &window, move |commands, caller| {
        commands.start_panel(caller, request)
    })
    .await
}

/// Publishes one complete caller-window drop-zone lease.
#[tauri::command]
pub async fn longhorn_transfer_publish_lease<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: TransferLeaseRequest,
) -> Result<TransferLeaseResponse, TransferHostError> {
    run_blocking(state, &window, move |commands, caller| {
        commands.publish_lease(caller, request)
    })
    .await
}

/// Commits one terminal panel transfer.
#[tauri::command]
pub async fn longhorn_transfer_commit_panel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: PanelTransferCommand,
) -> Result<PanelTransferResponse, TransferHostError> {
    run_blocking(state, &window, move |commands, caller| {
        commands.commit_panel(caller, request)
    })
    .await
}

/// Cancels one bounded transfer session.
#[tauri::command]
pub async fn longhorn_transfer_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriTransferState>,
    request: TransferCancelRequest,
) -> Result<TransferCancelResponse, TransferHostError> {
    run_blocking(state, &window, move |commands, caller| {
        commands.cancel(caller, request)
    })
    .await
}

pub(super) fn caller_handle<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<HostWindowHandle, TransferHostError> {
    HostWindowHandle::new(window.label())
        .map_err(|error| TransferHostError::invalid_caller(error.to_string()))
}
