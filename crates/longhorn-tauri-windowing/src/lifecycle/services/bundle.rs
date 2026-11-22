use longhorn_windowing::{WindowLifecycleClock, WindowPlacementSink, WindowUserCloseHandler};
use std::sync::Arc;

use tauri::Runtime;

use super::{
    WindowCaptureBackend, WindowLifecycleReporter, WindowLifecycleScheduler, WindowRevealBackend,
};

/// Complete injected runtime boundary for the lifecycle host.
pub struct TauriWindowLifecycleServices<R: Runtime> {
    pub(crate) clock: Arc<dyn WindowLifecycleClock + Send + Sync>,
    pub(crate) scheduler: Arc<dyn WindowLifecycleScheduler>,
    pub(crate) capture: Arc<dyn WindowCaptureBackend<R>>,
    pub(crate) sink: Arc<dyn WindowPlacementSink>,
    pub(crate) user_close: Arc<dyn WindowUserCloseHandler + Send + Sync>,
    pub(crate) reporter: Arc<dyn WindowLifecycleReporter>,
    pub(crate) reveal: Arc<dyn WindowRevealBackend<R>>,
}

impl<R: Runtime> TauriWindowLifecycleServices<R> {
    /// Collects the host's explicit clock, scheduling, I/O, and callback seams.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        clock: Arc<dyn WindowLifecycleClock + Send + Sync>,
        scheduler: Arc<dyn WindowLifecycleScheduler>,
        capture: Arc<dyn WindowCaptureBackend<R>>,
        sink: Arc<dyn WindowPlacementSink>,
        user_close: Arc<dyn WindowUserCloseHandler + Send + Sync>,
        reporter: Arc<dyn WindowLifecycleReporter>,
        reveal: Arc<dyn WindowRevealBackend<R>>,
    ) -> Self {
        Self {
            clock,
            scheduler,
            capture,
            sink,
            user_close,
            reporter,
            reveal,
        }
    }
}
