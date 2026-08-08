//! Tauri and lifecycle event handling.

use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::{
    ApplyGeneration, ApplyRegistrationOutcome, HostWindowHandle, WindowLifecycleCoordinator,
    WindowLifecycleEvent, WindowLifecyclePolicy, WindowOperation,
};
use tauri::{Runtime, WebviewWindow, WindowEvent};

use crate::lifecycle::{
    ProgrammaticApplyObserver, ScheduledWindowLifecycleWake, TauriWindowLifecycleAction,
    TauriWindowLifecycleError, TauriWindowLifecycleReceipt, TauriWindowLifecycleServices,
    WindowFlushRequest, WindowFlushScope, WindowFlushTarget, WindowLifecycleReport,
    WindowLifecycleWakeHandler, translate_tauri_window_event,
};

use super::{FlushDisposition, PendingFlush, TauriWindowLifecycleHost, coordination_error};

impl<R: Runtime> TauriWindowLifecycleHost<R> {
    /// terminal outcomes arrive as later reporter receipts.
    pub fn handle_tauri_event(
        self: &std::sync::Arc<Self>,
        window_id: &WindowId,
        event: &WindowEvent,
    ) -> Result<Option<TauriWindowLifecycleReceipt>, TauriWindowLifecycleError> {
        self.ensure_active()?;
        let window = {
            let windows = self.lock_windows()?;
            windows
                .get(window_id)
                .map(|installed| installed.window.clone())
                .ok_or_else(|| TauriWindowLifecycleError::UnknownWindow {
                    window_id: window_id.clone(),
                })?
        };
        let translated =
            translate_tauri_window_event(window_id, &window, event, self.services.mapper.as_ref())?;
        translated
            .map(|event| {
                let (mut receipt, pending) = self.handle_event_collecting(event)?;
                if !pending.is_empty() {
                    receipt = self.defer_flushes(receipt, pending);
                }
                Ok(receipt)
            })
            .transpose()
    }

    /// Handles one already translated input, including host-driven deadlines.
    /// Bounded flushes execute synchronously at their directive positions
    /// within the caller's thread.
    pub fn handle_lifecycle_event(
        &self,
        event: WindowLifecycleEvent,
    ) -> Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError> {
        self.handle_event_with(event, &mut FlushDisposition::Inline)
    }

    pub(crate) fn handle_event_collecting(
        &self,
        event: WindowLifecycleEvent,
    ) -> Result<(TauriWindowLifecycleReceipt, Vec<PendingFlush>), TauriWindowLifecycleError> {
        let mut pending = Vec::new();
        let receipt =
            self.handle_event_with(event, &mut FlushDisposition::Collect(&mut pending))?;
        Ok((receipt, pending))
    }

    pub(crate) fn handle_event_with(
        &self,
        event: WindowLifecycleEvent,
        flushes: &mut FlushDisposition,
    ) -> Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError> {
        self.ensure_active()?;
        let window_id = event.window_id().clone();
        self.ensure_installed(&window_id)?;
        let event_kind = event.kind();
        let directives = {
            let mut coordinator = self.lock_coordinator()?;
            coordinator
                .handle(self.services.clock.now(), event)
                .map_err(coordination_error)?
        };
        // A concurrent Destroyed between the installed check and the
        // coordinator call removes both entries; the coordinator would have
        // just recreated its state for this event, so release it instead of
        // leaking a tracked entry for a forgotten window.
        if event_kind != longhorn_windowing::WindowLifecycleEventKind::Destroyed
            && !self.lock_windows()?.contains_key(&window_id)
        {
            self.lock_coordinator()?.release(&window_id);
            return Err(TauriWindowLifecycleError::UnknownWindow { window_id });
        }
        let mut actions = Vec::new();
        self.execute_directives(directives, &mut actions, flushes)?;
        Ok(TauriWindowLifecycleReceipt::new(
            window_id, event_kind, actions,
        ))
    }

    pub(crate) fn defer_flushes(
        self: &std::sync::Arc<Self>,
        receipt: TauriWindowLifecycleReceipt,
        pending: Vec<PendingFlush>,
    ) -> TauriWindowLifecycleReceipt {
        let (window_id, event_kind, mut actions) = receipt.into_parts();
        let deferred: Vec<(WindowId, WindowFlushRequest)> = pending
            .into_iter()
            .map(|flush| {
                let target_window_id = flush.target.window_id().clone();
                let request =
                    WindowFlushRequest::new(vec![flush.target], flush.timeout, flush.scope);
                (target_window_id, request)
            })
            .collect();
        for (_, request) in &deferred {
            actions.push(TauriWindowLifecycleAction::FlushDeferred {
                request: request.clone(),
            });
        }
        let host = std::sync::Arc::clone(self);
        tauri::async_runtime::spawn_blocking(move || {
            for (target_window_id, request) in deferred {
                let outcome = host.flush(&request);
                host.services.reporter.report(WindowLifecycleReport::new(
                    target_window_id.clone(),
                    Some(event_kind),
                    Ok(TauriWindowLifecycleReceipt::new(
                        target_window_id,
                        event_kind,
                        vec![TauriWindowLifecycleAction::Flushed { request, outcome }],
                    )),
                ));
            }
        });
        TauriWindowLifecycleReceipt::new(window_id, event_kind, actions)
    }

    /// Delivers one exact wake previously accepted by the injected scheduler.
    pub fn handle_scheduled_wake(
        &self,
        wake: ScheduledWindowLifecycleWake,
    ) -> Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError> {
        self.handle_lifecycle_event(wake.event().clone())
    }

}
