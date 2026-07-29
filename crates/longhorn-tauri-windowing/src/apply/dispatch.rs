use std::{error::Error, fmt};

use longhorn_windowing::WindowDiffInput;
use tauri::{AppHandle, Runtime};

use crate::{
    ManagedDesktopReadback, ManagedWindowRegistry, TauriApplyError, TauriApplyOutcome,
    TauriWindowFactory, WindowMutationBackend, execute_tauri_window_apply,
};

/// Schedules a complete apply and readback on Tauri's main thread.
pub fn dispatch_tauri_window_apply<R, F, B, Q, C>(
    app: &AppHandle<R>,
    input: WindowDiffInput,
    registry: ManagedWindowRegistry<R>,
    factory: F,
    backend: B,
    readback: Q,
    completion: C,
) -> Result<(), TauriDispatchError>
where
    R: Runtime,
    F: TauriWindowFactory<R> + 'static,
    B: WindowMutationBackend<R> + 'static,
    Q: ManagedDesktopReadback<R> + 'static,
    C: FnOnce(Result<TauriApplyOutcome<R>, TauriApplyError>) + Send + 'static,
{
    let apply_app = app.clone();
    app.run_on_main_thread(move || {
        completion(execute_tauri_window_apply(
            &apply_app, input, registry, factory, backend, readback,
        ));
    })
    .map_err(|source| TauriDispatchError {
        detail: source.to_string(),
    })
}

/// Main-thread scheduling failure before execution starts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriDispatchError {
    detail: String,
}

impl TauriDispatchError {
    /// Returns the Tauri scheduling diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for TauriDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "main-thread window apply dispatch failed: {}",
            self.detail
        )
    }
}

impl Error for TauriDispatchError {}
