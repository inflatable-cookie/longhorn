use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::{ApplyGeneration, CapturedWindowPlacement, WindowOperation};
use tauri::{Runtime, WebviewWindow};

use super::super::WindowLifecycleReport;

/// Complete live capture seam.
pub trait WindowCaptureBackend<R: Runtime>: Send + Sync {
    /// Captures one window without persistence or product policy.
    fn capture(
        &self,
        window_id: &WindowId,
        window: &WebviewWindow<R>,
        retained_normal: Option<WindowPlacement>,
    ) -> Result<CapturedWindowPlacement, super::super::TauriWindowCaptureError>;
}

/// Async listener receipt observer.
pub trait WindowLifecycleReporter: Send + Sync {
    /// Records one complete event result.
    fn report(&self, report: WindowLifecycleReport);
}

/// Explicit no-op asynchronous lifecycle reporter.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopWindowLifecycleReporter;

impl WindowLifecycleReporter for NoopWindowLifecycleReporter {
    fn report(&self, _report: WindowLifecycleReport) {}
}

impl<F> WindowLifecycleReporter for F
where
    F: Fn(WindowLifecycleReport) + Send + Sync,
{
    fn report(&self, report: WindowLifecycleReport) {
        self(report);
    }
}

/// Native reveal seam.
pub trait WindowRevealBackend<R: Runtime>: Send + Sync {
    /// Shows one placement-ready, page-ready window.
    fn reveal(&self, window: &WebviewWindow<R>) -> Result<(), String>;
}

/// Direct Tauri show backend.
#[derive(Clone, Copy, Debug, Default)]
pub struct TauriWindowRevealBackend;

impl<R: Runtime> WindowRevealBackend<R> for TauriWindowRevealBackend {
    fn reveal(&self, window: &WebviewWindow<R>) -> Result<(), String> {
        window.show().map_err(|error| error.to_string())
    }
}

/// Observer invoked by Card 018 immediately before a native apply operation.
pub trait ProgrammaticApplyObserver: Send + Sync {
    /// Installs exact generation and operation evidence.
    fn register_apply(
        &self,
        generation: ApplyGeneration,
        operation: &WindowOperation,
    ) -> Result<(), String>;
}
