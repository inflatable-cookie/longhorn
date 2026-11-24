use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime};

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
        let webview = self.app.get_webview_window(target).ok_or_else(|| {
            publication_error("bridge event's target webview window is unavailable")
        })?;
        // Tauri's filtered events can still evaluate the payload in other
        // webviews. Deliver directly to the owning webview instead.
        let event = serde_json::to_string(event).map_err(publication_error)?;
        let payload = serde_json::to_string(&payload.to_string()).map_err(publication_error)?;
        webview
            .eval(format!(
                "window.dispatchEvent(new CustomEvent({event}, {{ detail: JSON.parse({payload}) }}))"
            ))
            .map_err(publication_error)
    }
}

fn publication_error(error: impl ToString) -> BridgeHostError {
    BridgeHostError::new(
        BridgeHostErrorCode::EventPublication,
        error.to_string(),
        true,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use serde_json::json;
    use tauri::{Listener, WebviewUrl, WebviewWindowBuilder};

    use super::*;

    #[test]
    fn publishes_only_to_a_live_webview_and_skips_the_tauri_event_bus() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let _window = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
            .build()
            .unwrap();
        let received = Arc::new(AtomicBool::new(false));
        let listened = Arc::clone(&received);
        let _listener = app.listen_any(BRIDGE_DOMAIN_EVENT, move |_| {
            listened.store(true, Ordering::Relaxed);
        });
        let sink = TauriBridgeEventSink::new(app.handle().clone());

        sink.emit("main", BRIDGE_DOMAIN_EVENT, json!({"status": "ready"}))
            .unwrap();
        let error = sink
            .emit("missing", BRIDGE_DOMAIN_EVENT, json!(null))
            .unwrap_err();

        assert!(!received.load(Ordering::Relaxed));
        assert_eq!(error.code, BridgeHostErrorCode::EventPublication);
    }
}
