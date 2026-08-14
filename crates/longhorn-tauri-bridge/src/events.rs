use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

use crate::{BridgeHostError, BridgeHostErrorCode};

/// Authoritative domain update event.
pub const BRIDGE_DOMAIN_EVENT: &str = "longhorn://bridge/domain";
/// Optional request-correlated progress event.
pub const BRIDGE_PROGRESS_EVENT: &str = "longhorn://bridge/progress";
/// Optional request-correlated terminal event.
pub const BRIDGE_TERMINAL_EVENT: &str = "longhorn://bridge/terminal";

/// Injected event publication edge shared by real and mock hosts.
///
/// Delivery is targeted: an event goes to the window that owns the session
/// it was published under, not to every webview in the application. The
/// client already drops foreign-session cursors, so app-wide broadcast was
/// delivery without a consumer — and a read-authority hole beside the rest
/// of the per-caller model.
pub trait BridgeEventSink: Send + Sync {
    /// Publishes one checked generic bridge payload to `target`.
    fn emit(
        &self,
        target: &str,
        event: &'static str,
        payload: Value,
    ) -> Result<(), BridgeHostError>;
}

impl<F> BridgeEventSink for F
where
    F: Fn(&str, &'static str, Value) -> Result<(), BridgeHostError> + Send + Sync,
{
    fn emit(
        &self,
        target: &str,
        event: &'static str,
        payload: Value,
    ) -> Result<(), BridgeHostError> {
        self(target, event, payload)
    }
}

/// Real Tauri event edge over an application handle.
pub struct TauriBridgeEventSink<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriBridgeEventSink<R> {
    /// Binds bridge event publication to one Tauri application.
    #[must_use]
    pub const fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> BridgeEventSink for TauriBridgeEventSink<R> {
    fn emit(
        &self,
        target: &str,
        event: &'static str,
        payload: Value,
    ) -> Result<(), BridgeHostError> {
        self.app.emit_to(target, event, payload).map_err(|error| {
            BridgeHostError::new(
                BridgeHostErrorCode::EventPublication,
                error.to_string(),
                true,
            )
        })
    }
}
