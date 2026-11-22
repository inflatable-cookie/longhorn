use longhorn_core::WindowId;
use longhorn_windowing::{
    WindowFlushOutcome, WindowFlushRequest, WindowLifecycleClock, WindowLifecycleEvent,
    WindowUserCloseHandler, flush_window_placements,
};

use crate::GpuiWindowLifecycleError;

use super::{
    GpuiLifecycleReceipt, GpuiLifecycleScheduler, GpuiWindowCaptureBackend,
    GpuiWindowLifecycleHost, close_is_safe,
};

/// What one shutdown flush did.
///
/// Two halves because they can fail independently: a window that could not be
/// captured is not the same as a store that could not be written, and a caller
/// deciding whether it is safe to exit needs to tell them apart.
#[derive(Debug)]
pub struct GpuiShutdownReceipt {
    per_window: Vec<GpuiLifecycleReceipt>,
    failures: Vec<(WindowId, GpuiWindowLifecycleError)>,
    outcome: Option<WindowFlushOutcome>,
}

impl GpuiShutdownReceipt {
    /// One receipt per window that was asked for a capture.
    #[must_use]
    pub fn per_window(&self) -> &[GpuiLifecycleReceipt] {
        &self.per_window
    }

    /// Per-window failures that prevented a lifecycle receipt.
    #[must_use]
    pub fn failures(&self) -> &[(WindowId, GpuiWindowLifecycleError)] {
        &self.failures
    }

    /// The aggregate write, or `None` when there were no windows to write.
    #[must_use]
    pub const fn outcome(&self) -> Option<&WindowFlushOutcome> {
        self.outcome.as_ref()
    }

    /// Whether everything that was asked for actually happened.
    ///
    /// A shutdown that half-worked is not a shutdown that succeeded, and the
    /// caller is usually about to exit.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.failures.is_empty()
            && self.per_window.iter().all(close_is_safe)
            && self
                .outcome
                .as_ref()
                .is_none_or(|outcome| matches!(outcome, WindowFlushOutcome::Succeeded))
    }
}

impl<C, S, B, U> GpuiWindowLifecycleHost<C, S, B, U>
where
    C: WindowLifecycleClock,
    S: GpuiLifecycleScheduler,
    B: GpuiWindowCaptureBackend,
    U: WindowUserCloseHandler,
{
    /// Settles pending captures and flushes ready placements in one bounded write.
    pub fn shutdown_flush(&mut self) -> GpuiShutdownReceipt {
        let window_ids: Vec<_> = self.windows.keys().cloned().collect();
        let mut per_window = Vec::with_capacity(window_ids.len());
        let mut failures = Vec::new();
        let mut targets = Vec::new();
        for window_id in window_ids {
            let event = WindowLifecycleEvent::FlushRequested {
                window_id: window_id.clone(),
            };
            match self.handle_event(event, &mut Some(&mut targets)) {
                Ok(receipt) => per_window.push(receipt),
                Err(error) => failures.push((window_id, error)),
            }
        }

        let outcome = WindowFlushRequest::for_shutdown(targets, self.policy.flush_timeout())
            .map(|request| flush_window_placements(&*self.services.sink, &request));
        GpuiShutdownReceipt {
            per_window,
            failures,
            outcome,
        }
    }
}
