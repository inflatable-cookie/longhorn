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
pub trait BridgeEventSink: Send + Sync {
    /// Publishes one checked generic bridge payload.
    fn emit(&self, event: &'static str, payload: Value) -> Result<(), BridgeHostError>;
}

impl<F> BridgeEventSink for F
where
    F: Fn(&'static str, Value) -> Result<(), BridgeHostError> + Send + Sync,
{
    fn emit(&self, event: &'static str, payload: Value) -> Result<(), BridgeHostError> {
        self(event, payload)
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
    fn emit(&self, event: &'static str, payload: Value) -> Result<(), BridgeHostError> {
        self.app.emit(event, payload).map_err(|error| {
            BridgeHostError::new(
                BridgeHostErrorCode::EventPublication,
                error.to_string(),
                true,
            )
        })
    }
}
