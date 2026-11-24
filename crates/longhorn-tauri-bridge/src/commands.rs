use std::sync::Arc;

use longhorn_bridge::{
    BridgeCancellationReceipt, BridgeCancellationRequest, BridgeCommandEnvelope,
    BridgeCommandReply, BridgeHelloRequest, BridgeNegotiationReceipt, BridgeQueryEnvelope,
    BridgeQueryReply, BridgeSnapshotEnvelope,
};
use longhorn_core::{BridgeSessionId, DomainId};
use serde_json::Value;
use tauri::{Manager, RunEvent, Runtime, State, WebviewWindow, WindowEvent, plugin::TauriPlugin};

use crate::{
    BridgeAuthorityProvider, BridgeHandlerAssembly, BridgeHostError,
    handler::{HelloTicket, caller_torn_down},
};

/// Type-erased bridge commands installed once in Tauri managed state.
pub struct TauriBridgeState {
    assembly: Arc<BridgeHandlerAssembly<dyn BridgeAuthorityProvider>>,
}

impl TauriBridgeState {
    /// Wraps one explicitly injected real or mock host assembly.
    ///
    /// Prefer [`bridge_plugin`], which installs this state and wires
    /// [`Self::teardown_window`] onto Tauri's window-destroyed lifecycle hook
    /// automatically. Hosts calling `TauriBridgeState::new` directly must wire
    /// `teardown_window` from `.on_window_event`.
    #[must_use]
    pub fn new(assembly: Arc<BridgeHandlerAssembly<dyn BridgeAuthorityProvider>>) -> Self {
        Self { assembly }
    }

    /// Ends every session a destroyed window owned.
    pub fn teardown_window(&self, label: &str) -> Result<(), BridgeHostError> {
        self.assembly.teardown(label)
    }
}

/// Installs [`TauriBridgeState`] and wires window-destroyed session teardown
/// into Tauri's lifecycle hook by default.
#[must_use]
pub fn bridge_plugin<R: Runtime>(
    assembly: Arc<BridgeHandlerAssembly<dyn BridgeAuthorityProvider>>,
) -> TauriPlugin<R> {
    tauri::plugin::Builder::new("longhorn-bridge")
        .setup(move |app, _api| {
            app.manage(TauriBridgeState::new(assembly));
            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::Destroyed,
                ..
            } = event
                && let Some(state) = app.try_state::<TauriBridgeState>()
                && let Err(error) = state.teardown_window(label)
            {
                longhorn_core::report_best_effort_failure("bridge.window-teardown", error);
            }
        })
        .build()
}

struct OwnedHelloGuard {
    assembly: Arc<BridgeHandlerAssembly<dyn BridgeAuthorityProvider>>,
    ticket: Option<HelloTicket>,
}

impl OwnedHelloGuard {
    fn begin(
        assembly: Arc<BridgeHandlerAssembly<dyn BridgeAuthorityProvider>>,
        caller: &str,
    ) -> Result<Self, BridgeHostError> {
        let ticket = assembly.begin_hello(caller)?;
        Ok(Self {
            assembly,
            ticket: Some(ticket),
        })
    }

    fn take(&mut self) -> Result<HelloTicket, BridgeHostError> {
        self.ticket
            .take()
            .ok_or_else(BridgeHostError::state_unavailable)
    }
}

impl Drop for OwnedHelloGuard {
    fn drop(&mut self) {
        if let Some(ticket) = self.ticket.take() {
            self.assembly.abandon_hello(ticket);
        }
    }
}

async fn run_blocking_with<T: Send + 'static>(
    assembly: Arc<BridgeHandlerAssembly<dyn BridgeAuthorityProvider>>,
    action: impl FnOnce(
        &BridgeHandlerAssembly<dyn BridgeAuthorityProvider>,
    ) -> Result<T, BridgeHostError>
    + Send
    + 'static,
) -> Result<T, BridgeHostError> {
    tauri::async_runtime::spawn_blocking(move || action(&assembly))
        .await
        .map_err(|_| BridgeHostError::state_unavailable())?
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriBridgeState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &BridgeHandlerAssembly<dyn BridgeAuthorityProvider>,
        &str,
    ) -> Result<T, BridgeHostError>
    + Send
    + 'static,
) -> Result<T, BridgeHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    run_blocking_with(assembly, move |assembly| action(assembly, &label)).await
}

/// Negotiates one bridge session for the invoking window.
#[tauri::command]
pub async fn longhorn_bridge_hello<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    request: BridgeHelloRequest,
) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
    let label = window.label().to_owned();
    let assembly = Arc::clone(&state.assembly);
    let mut guard = OwnedHelloGuard::begin(Arc::clone(&assembly), &label)?;
    if window.app_handle().get_webview_window(&label).is_none() {
        return Err(caller_torn_down(&label));
    }
    run_blocking_with(assembly, move |assembly| {
        let ticket = guard.take()?;
        assembly.hello_with_ticket(&label, request, ticket)
    })
    .await
}

/// Refreshes capability and authority facts for the invoking window.
#[tauri::command]
pub async fn longhorn_bridge_authority<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    session_id: BridgeSessionId,
) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.authority(label, &session_id)
    })
    .await
}

/// Dispatches one registered query route.
#[tauri::command]
pub async fn longhorn_bridge_query<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    route: String,
    request: BridgeQueryEnvelope<Value>,
) -> Result<BridgeQueryReply<Value, Value>, BridgeHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.query(label, &route, request)
    })
    .await
}

/// Dispatches one registered authoritative command route.
#[tauri::command]
pub async fn longhorn_bridge_command<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    route: String,
    request: BridgeCommandEnvelope<Value>,
) -> Result<BridgeCommandReply<Value, Value>, BridgeHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.command(label, &route, request)
    })
    .await
}

/// Dispatches one registered cancellation route.
#[tauri::command]
pub async fn longhorn_bridge_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    route: String,
    request: BridgeCancellationRequest,
) -> Result<BridgeCancellationReceipt<Value>, BridgeHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.cancel(label, &route, request)
    })
    .await
}

/// Loads one registered authoritative domain snapshot.
#[tauri::command]
pub async fn longhorn_bridge_resync<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    session_id: BridgeSessionId,
    domain_id: DomainId,
) -> Result<BridgeSnapshotEnvelope<Value>, BridgeHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.resync(label, &session_id, &domain_id)
    })
    .await
}
