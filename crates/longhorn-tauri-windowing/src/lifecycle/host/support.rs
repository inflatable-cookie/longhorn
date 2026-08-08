//! Host activity and lock helpers.

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

use super::{InstalledWindow, TauriWindowLifecycleHost};

impl<R: Runtime> TauriWindowLifecycleHost<R> {
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

    pub(crate) fn ensure_active(&self) -> Result<(), TauriWindowLifecycleError> {
        if self.is_active() {
            Ok(())
        } else {
            Err(TauriWindowLifecycleError::InactiveHost)
        }
    }

    pub(crate) fn ensure_installed(&self, window_id: &WindowId) -> Result<(), TauriWindowLifecycleError> {
        if self.lock_windows()?.contains_key(window_id) {
            Ok(())
        } else {
            Err(TauriWindowLifecycleError::UnknownWindow {
                window_id: window_id.clone(),
            })
        }
    }

    pub(crate) fn window_id_for_handle(
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

    pub(crate) fn lock_windows(
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

    pub(crate) fn lock_coordinator(
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
