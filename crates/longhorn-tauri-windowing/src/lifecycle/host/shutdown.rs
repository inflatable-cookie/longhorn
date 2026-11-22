use longhorn_core::WindowId;
use longhorn_windowing::{WindowFlushRequest, WindowLifecycleEvent};
use tauri::Runtime;

use super::{PendingFlush, TauriWindowLifecycleHost};
use crate::{TauriWindowLifecycleAction, TauriWindowLifecycleError, WindowShutdownReceipt};

impl<R: Runtime> TauriWindowLifecycleHost<R> {
    /// Requests current captures and one aggregate bounded sink flush.
    pub fn shutdown_flush(&self) -> Result<WindowShutdownReceipt, TauriWindowLifecycleError> {
        self.ensure_active()?;
        let window_ids: Vec<WindowId> = self.lock_windows()?.keys().cloned().collect();
        let mut actions = Vec::new();
        let mut pending = Vec::<PendingFlush>::new();
        for window_id in window_ids {
            let receipt = self.handle_event_with(
                WindowLifecycleEvent::FlushRequested { window_id },
                &mut super::FlushDisposition::Collect(&mut pending),
            )?;
            let (_, _, window_actions) = receipt.into_parts();
            actions.extend(window_actions);
        }
        let Some(timeout) = pending
            .iter()
            .map(|flush| flush.timeout)
            .max_by_key(|duration| duration.as_millis())
        else {
            return Ok(WindowShutdownReceipt::new(actions, None));
        };
        let Some(request) = WindowFlushRequest::for_shutdown(
            pending.into_iter().map(|flush| flush.target).collect(),
            timeout,
        ) else {
            return Ok(WindowShutdownReceipt::new(actions, None));
        };
        let outcome = self.flush(&request);
        actions.push(TauriWindowLifecycleAction::Flushed {
            request,
            outcome: outcome.clone(),
        });
        Ok(WindowShutdownReceipt::new(actions, Some(outcome)))
    }
}
