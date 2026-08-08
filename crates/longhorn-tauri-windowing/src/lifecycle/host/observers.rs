//! Wake and programmatic-apply observer adapters.

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

use super::TauriWindowLifecycleHost;

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

pub(crate) fn coordination_error(error: impl ToString) -> TauriWindowLifecycleError {
    TauriWindowLifecycleError::Coordination {
        detail: error.to_string(),
    }
}
