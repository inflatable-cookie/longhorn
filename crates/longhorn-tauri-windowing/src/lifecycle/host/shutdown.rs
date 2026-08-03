use longhorn_core::WindowId;
use longhorn_windowing::WindowLifecycleEvent;
use tauri::Runtime;

use super::{PendingFlush, TauriWindowLifecycleHost, coordination_error};
use crate::{
    TauriWindowLifecycleAction, TauriWindowLifecycleError, WindowFlushRequest, WindowFlushScope,
    WindowShutdownReceipt,
};

impl<R: Runtime> TauriWindowLifecycleHost<R> {
    /// Requests current captures and one aggregate bounded sink flush.
    pub fn shutdown_flush(&self) -> Result<WindowShutdownReceipt, TauriWindowLifecycleError> {
        self.ensure_active()?;
        let window_ids: Vec<WindowId> = self.lock_windows()?.keys().cloned().collect();
        let mut actions = Vec::new();
        let mut pending = Vec::<PendingFlush>::new();
        for window_id in window_ids {
            let directives = {
                let mut coordinator = self.lock_coordinator()?;
                coordinator
                    .handle(
                        self.services.clock.now(),
                        WindowLifecycleEvent::FlushRequested { window_id },
                    )
                    .map_err(coordination_error)?
            };
            self.execute_directives(
                directives,
                &mut actions,
                &mut super::FlushDisposition::Collect(&mut pending),
            )?;
        }
        if pending.is_empty() {
            return Ok(WindowShutdownReceipt::new(actions, None));
        }
        pending.sort_by(|left, right| {
            left.target
                .window_id()
                .cmp(right.target.window_id())
                .then(left.target.generation().cmp(&right.target.generation()))
        });
        pending.dedup_by(|left, right| left.target == right.target);
        let timeout = pending
            .iter()
            .map(|flush| flush.timeout)
            .max_by_key(|duration| duration.as_millis())
            .expect("non-empty pending flush list");
        let request = WindowFlushRequest::new(
            pending.into_iter().map(|flush| flush.target).collect(),
            timeout,
            WindowFlushScope::ApplicationShutdown,
        );
        let outcome = self.flush(&request);
        actions.push(TauriWindowLifecycleAction::Flushed {
            request,
            outcome: outcome.clone(),
        });
        Ok(WindowShutdownReceipt::new(actions, Some(outcome)))
    }
}
