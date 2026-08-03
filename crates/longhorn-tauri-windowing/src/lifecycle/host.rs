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

use super::{
    ProgrammaticApplyObserver, ScheduledWindowLifecycleWake, TauriWindowLifecycleAction,
    TauriWindowLifecycleError, TauriWindowLifecycleReceipt, TauriWindowLifecycleServices,
    WindowFlushTarget, WindowLifecycleReport, WindowLifecycleWakeHandler,
    translate_tauri_window_event,
};

mod directives;
mod reveal;
mod shutdown;

pub(super) struct InstalledWindow<R: Runtime> {
    window: WebviewWindow<R>,
    retained_normal: Option<WindowPlacement>,
    page_ready: bool,
    placement_ready: bool,
    reveal_started: bool,
    reveal_retry: bool,
    revealed: bool,
}

pub(super) struct PendingFlush {
    target: WindowFlushTarget,
    timeout: longhorn_windowing::WindowLifecycleDuration,
    scope: super::WindowFlushScope,
}

pub(super) enum FlushDisposition<'a> {
    /// Execute each flush at its directive position on the current thread.
    Inline,
    /// Record flushes for the caller to execute or defer.
    Collect(&'a mut Vec<PendingFlush>),
}

/// Tauri listener and I/O adapter over the pure lifecycle coordinator.
pub struct TauriWindowLifecycleHost<R: Runtime> {
    active: AtomicBool,
    coordinator: Mutex<WindowLifecycleCoordinator>,
    windows: Mutex<BTreeMap<WindowId, InstalledWindow<R>>>,
    services: TauriWindowLifecycleServices<R>,
}

impl<R: Runtime> TauriWindowLifecycleHost<R> {
    /// Constructs an empty host with caller-owned policy and runtime seams.
    #[must_use]
    pub fn new(policy: WindowLifecyclePolicy, services: TauriWindowLifecycleServices<R>) -> Self {
        Self {
            active: AtomicBool::new(true),
            coordinator: Mutex::new(WindowLifecycleCoordinator::new(policy)),
            windows: Mutex::new(BTreeMap::new()),
            services,
        }
    }

    /// Constructs a shared host and binds schedulers that need its weak target.
    pub fn shared(
        policy: WindowLifecyclePolicy,
        services: TauriWindowLifecycleServices<R>,
    ) -> Result<std::sync::Arc<Self>, TauriWindowLifecycleError> {
        let host = std::sync::Arc::new(Self::new(policy, services));
        let handler: std::sync::Arc<dyn WindowLifecycleWakeHandler> = host.clone();
        host.services
            .scheduler
            .bind(std::sync::Arc::downgrade(&handler))
            .map_err(|detail| TauriWindowLifecycleError::SchedulerBinding { detail })?;
        Ok(host)
    }

    /// Installs a listener for a predeclared or dynamically created window.
    pub fn install_window(
        self: &std::sync::Arc<Self>,
        window_id: WindowId,
        window: WebviewWindow<R>,
        initial_normal: Option<WindowPlacement>,
    ) -> Result<(), TauriWindowLifecycleError> {
        self.ensure_active()?;
        let transport_handle = HostWindowHandle::new(window.label()).map_err(|error| {
            TauriWindowLifecycleError::InvalidWindowLabel {
                detail: error.to_string(),
            }
        })?;
        {
            let mut windows = self.lock_windows()?;
            if windows.contains_key(&window_id) {
                return Err(TauriWindowLifecycleError::DuplicateWindow { window_id });
            }
            windows.insert(
                window_id.clone(),
                InstalledWindow {
                    window: window.clone(),
                    retained_normal: initial_normal,
                    page_ready: false,
                    placement_ready: false,
                    reveal_started: false,
                    reveal_retry: false,
                    revealed: false,
                },
            );
        }

        let host = std::sync::Arc::downgrade(self);
        window.on_window_event(move |event| {
            let Some(host) = host.upgrade() else {
                return;
            };
            if !host.is_active() {
                return;
            }
            let current_window_id = match host.window_id_for_handle(&transport_handle) {
                Ok(window_id) => window_id,
                Err(error) => {
                    host.services.reporter.report(WindowLifecycleReport::new(
                        window_id.clone(),
                        None,
                        Err(error),
                    ));
                    return;
                }
            };
            let result = host.handle_tauri_event(&current_window_id, event);
            let event_kind = result
                .as_ref()
                .ok()
                .and_then(|receipt| receipt.as_ref().map(TauriWindowLifecycleReceipt::event));
            let should_prevent = result
                .as_ref()
                .ok()
                .and_then(|receipt| receipt.as_ref())
                .is_some_and(|receipt| {
                    receipt.actions().iter().any(|action| {
                        matches!(
                            action,
                            TauriWindowLifecycleAction::UserCloseReported
                                | TauriWindowLifecycleAction::UserCloseFailed { .. }
                        )
                    })
                });
            if should_prevent {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
            }
            if let Some(receipt_result) = result.transpose() {
                host.services.reporter.report(WindowLifecycleReport::new(
                    current_window_id,
                    event_kind,
                    receipt_result,
                ));
            }
        });
        Ok(())
    }

    pub(crate) fn retag_window(
        &self,
        transport_handle: &HostWindowHandle,
        window_id: WindowId,
    ) -> Result<(), TauriWindowLifecycleError> {
        self.ensure_active()?;
        let previous = {
            let windows = self.lock_windows()?;
            if windows.contains_key(&window_id) {
                return Err(TauriWindowLifecycleError::DuplicateWindow { window_id });
            }
            windows
                .iter()
                .find(|(_, installed)| installed.window.label() == transport_handle.as_str())
                .map(|(window_id, _)| window_id.clone())
                .ok_or_else(|| TauriWindowLifecycleError::UnknownWindowHandle {
                    transport_handle: transport_handle.clone(),
                })?
        };
        // Coordinator state migrates with the identity; pending deadlines are
        // re-scheduled under the new id, and any wake still queued under the
        // previous id fails as unknown and counts as superseded.
        let reschedule = self
            .lock_coordinator()?
            .retag(&previous, &window_id)
            .map_err(coordination_error)?;
        {
            let mut windows = self.lock_windows()?;
            let installed = windows.remove(&previous).ok_or_else(|| {
                TauriWindowLifecycleError::UnknownWindowHandle {
                    transport_handle: transport_handle.clone(),
                }
            })?;
            windows.insert(window_id.clone(), installed);
        }
        let mut actions = Vec::new();
        self.execute_directives(reschedule, &mut actions, &mut FlushDisposition::Inline)?;
        for action in actions {
            if let TauriWindowLifecycleAction::ScheduleFailed { detail, .. } = action {
                self.services.reporter.report(WindowLifecycleReport::new(
                    window_id.clone(),
                    None,
                    Err(TauriWindowLifecycleError::Coordination {
                        detail: format!("retag wake re-schedule failed: {detail}"),
                    }),
                ));
            }
        }
        Ok(())
    }

    /// Translates and handles one native event on the Tauri event thread.
    /// Irrelevant events return `None`. Bounded flushes never block this
    /// thread: they are deferred to the runtime's blocking pool and their
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

    fn handle_event_collecting(
        &self,
        event: WindowLifecycleEvent,
    ) -> Result<(TauriWindowLifecycleReceipt, Vec<PendingFlush>), TauriWindowLifecycleError> {
        let mut pending = Vec::new();
        let receipt =
            self.handle_event_with(event, &mut FlushDisposition::Collect(&mut pending))?;
        Ok((receipt, pending))
    }

    fn handle_event_with(
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

    fn defer_flushes(
        self: &std::sync::Arc<Self>,
        receipt: TauriWindowLifecycleReceipt,
        pending: Vec<PendingFlush>,
    ) -> TauriWindowLifecycleReceipt {
        let (window_id, event_kind, mut actions) = receipt.into_parts();
        let deferred: Vec<(WindowId, super::WindowFlushRequest)> = pending
            .into_iter()
            .map(|flush| {
                let target_window_id = flush.target.window_id().clone();
                let request =
                    super::WindowFlushRequest::new(vec![flush.target], flush.timeout, flush.scope);
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

    /// Best-effort initial normal placement for a freshly created window, so
    /// a dynamic window maximized before its first settled capture still has
    /// a persistable normal placement.
    pub(crate) fn capture_initial_normal(
        &self,
        window_id: &WindowId,
        window: &WebviewWindow<R>,
    ) -> Option<WindowPlacement> {
        self.services
            .capture
            .capture(window_id, window, None)
            .ok()
            .filter(|placement| !placement.is_maximized())
            .map(|placement| placement.normal_placement())
    }

    /// Returns whether the host still accepts lifecycle work.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(crate) fn deactivate(&self) -> Result<usize, TauriWindowLifecycleError> {
        if !self.active.swap(false, Ordering::AcqRel) {
            return Ok(0);
        }
        let mut windows = self.lock_windows()?;
        let count = windows.len();
        windows.clear();
        Ok(count)
    }

    pub(crate) fn installed_window_count(&self) -> Result<usize, TauriWindowLifecycleError> {
        Ok(self.lock_windows()?.len())
    }

    fn ensure_active(&self) -> Result<(), TauriWindowLifecycleError> {
        if self.is_active() {
            Ok(())
        } else {
            Err(TauriWindowLifecycleError::InactiveHost)
        }
    }

    fn ensure_installed(&self, window_id: &WindowId) -> Result<(), TauriWindowLifecycleError> {
        if self.lock_windows()?.contains_key(window_id) {
            Ok(())
        } else {
            Err(TauriWindowLifecycleError::UnknownWindow {
                window_id: window_id.clone(),
            })
        }
    }

    fn window_id_for_handle(
        &self,
        transport_handle: &HostWindowHandle,
    ) -> Result<WindowId, TauriWindowLifecycleError> {
        self.lock_windows()?
            .iter()
            .find(|(_, installed)| installed.window.label() == transport_handle.as_str())
            .map(|(window_id, _)| window_id.clone())
            .ok_or_else(|| TauriWindowLifecycleError::UnknownWindowHandle {
                transport_handle: transport_handle.clone(),
            })
    }

    fn lock_windows(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, BTreeMap<WindowId, InstalledWindow<R>>>,
        TauriWindowLifecycleError,
    > {
        self.windows
            .lock()
            .map_err(|_| TauriWindowLifecycleError::StateUnavailable {
                state: "installed windows".to_string(),
            })
    }

    fn lock_coordinator(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, WindowLifecycleCoordinator>, TauriWindowLifecycleError>
    {
        self.coordinator
            .lock()
            .map_err(|_| TauriWindowLifecycleError::StateUnavailable {
                state: "lifecycle coordinator".to_string(),
            })
    }
}

impl<R: Runtime> WindowLifecycleWakeHandler for TauriWindowLifecycleHost<R> {
    fn handle_scheduled_wake(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        let window_id = wake.event().window_id().clone();
        let event_kind = wake.event().kind();
        match TauriWindowLifecycleHost::handle_scheduled_wake(self, wake) {
            Ok(receipt) => {
                self.services.reporter.report(WindowLifecycleReport::new(
                    window_id,
                    Some(event_kind),
                    Ok(receipt),
                ));
                Ok(())
            }
            Err(error) => {
                let detail = format!("{error:?}");
                self.services.reporter.report(WindowLifecycleReport::new(
                    window_id,
                    Some(event_kind),
                    Err(error),
                ));
                Err(detail)
            }
        }
    }
}

impl<R: Runtime> ProgrammaticApplyObserver for TauriWindowLifecycleHost<R> {
    fn register_apply(
        &self,
        generation: ApplyGeneration,
        operation: &WindowOperation,
    ) -> Result<(), String> {
        if !self.is_active() {
            return Err("window lifecycle host is inactive".to_string());
        }
        let outcome = self
            .coordinator
            .lock()
            .map_err(|_| "lifecycle coordinator lock is poisoned".to_string())?
            .register_apply(self.services.clock.now(), generation, operation)
            .map_err(|error| error.to_string())?;
        match outcome {
            ApplyRegistrationOutcome::Registered | ApplyRegistrationOutcome::Extended => Ok(()),
            ApplyRegistrationOutcome::StaleGeneration { current } => Err(format!(
                "apply generation {} is older than {}",
                generation.get(),
                current.get()
            )),
            ApplyRegistrationOutcome::StaleTimestamp { latest } => Err(format!(
                "apply evidence timestamp precedes {}",
                latest.get()
            )),
        }?;
        if let WindowOperation::MoveResize {
            window_id,
            placement,
            ..
        } = operation
        {
            let mut windows = self
                .windows
                .lock()
                .map_err(|_| "installed windows lock is poisoned".to_string())?;
            let installed = windows
                .get_mut(window_id)
                .ok_or_else(|| format!("unknown installed window {window_id}"))?;
            installed.retained_normal = Some(*placement);
        }
        Ok(())
    }
}

pub(super) fn coordination_error(error: impl ToString) -> TauriWindowLifecycleError {
    TauriWindowLifecycleError::Coordination {
        detail: error.to_string(),
    }
}
