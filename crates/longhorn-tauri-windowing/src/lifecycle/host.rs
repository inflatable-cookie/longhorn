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
    revealed: bool,
}

pub(super) struct PendingFlush {
    target: WindowFlushTarget,
    timeout: longhorn_windowing::WindowLifecycleDuration,
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
                    revealed: false,
                },
            );
        }

        let host = std::sync::Arc::downgrade(self);
        let transport_handle = HostWindowHandle::new(window.label())
            .expect("installed Tauri labels are valid host handles");
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
        let mut windows = self.lock_windows()?;
        if windows.contains_key(&window_id) {
            return Err(TauriWindowLifecycleError::DuplicateWindow { window_id });
        }
        let previous = windows
            .iter()
            .find(|(_, installed)| installed.window.label() == transport_handle.as_str())
            .map(|(window_id, _)| window_id.clone())
            .ok_or_else(|| TauriWindowLifecycleError::UnknownWindowHandle {
                transport_handle: transport_handle.clone(),
            })?;
        let installed = windows
            .remove(&previous)
            .expect("located lifecycle window remains present under lock");
        windows.insert(window_id, installed);
        Ok(())
    }

    /// Translates and handles one native event. Irrelevant events return `None`.
    pub fn handle_tauri_event(
        &self,
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
            .map(|event| self.handle_lifecycle_event(event))
            .transpose()
    }

    /// Handles one already translated input, including host-driven deadlines.
    pub fn handle_lifecycle_event(
        &self,
        event: WindowLifecycleEvent,
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
        let mut actions = Vec::new();
        self.execute_directives(directives, &mut actions, None)?;
        Ok(TauriWindowLifecycleReceipt::new(
            window_id, event_kind, actions,
        ))
    }

    /// Delivers one exact wake previously accepted by the injected scheduler.
    pub fn handle_scheduled_wake(
        &self,
        wake: ScheduledWindowLifecycleWake,
    ) -> Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError> {
        self.handle_lifecycle_event(wake.event().clone())
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
        TauriWindowLifecycleHost::handle_scheduled_wake(self, wake)
            .map(|_| ())
            .map_err(|error| format!("{error:?}"))
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
