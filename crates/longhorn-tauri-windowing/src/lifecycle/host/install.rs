//! Window install and retag operations.

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

use super::{FlushDisposition, InstalledWindow, PendingFlush, TauriWindowLifecycleHost, coordination_error};

impl<R: Runtime> TauriWindowLifecycleHost<R> {
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
            if should_prevent && let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
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

}
