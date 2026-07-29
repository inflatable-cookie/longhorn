use std::sync::atomic::Ordering;

use longhorn_core::WindowId;
use longhorn_windowing::{WindowDiffInput, WindowLifecycleEvent};
use tauri::{AppHandle, Runtime};

use super::{
    PHASE_ACTIVE, PHASE_APPLYING, PHASE_TEARING_DOWN, PHASE_TORN_DOWN, TauriWindowHost,
    TauriWindowHostApplyReceipt, TauriWindowHostError, TauriWindowHostTeardownReceipt,
    TauriWindowHostTeardownStatus,
};
use crate::{
    ManagedDesktopReadback, ManagedWindowRegistry, ScheduledWindowLifecycleWake,
    TauriWindowFactory, TauriWindowLifecycleReceipt, WindowMutationBackend, WindowRevealReceipt,
    execute_tauri_window_apply_in_place,
};

impl<R: Runtime> TauriWindowHost<R> {
    /// Returns exact native capabilities for this apply's factory availability.
    #[must_use]
    pub fn capabilities(&self, can_create: bool) -> longhorn_windowing::HostCapabilities {
        crate::tauri_host_capabilities(can_create)
    }

    /// Returns whether teardown has not completed.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.phase.load(Ordering::Acquire) != PHASE_TORN_DOWN && self.lifecycle.is_active()
    }

    /// Returns the number of currently installed lifecycle windows.
    pub fn installed_window_count(&self) -> Result<usize, TauriWindowHostError> {
        self.lifecycle
            .installed_window_count()
            .map_err(TauriWindowHostError::Lifecycle)
    }

    /// Clones the current managed-window set for narrow peer host adapters.
    pub fn managed_windows(
        &self,
    ) -> Result<Vec<crate::ManagedWebviewWindow<R>>, TauriWindowHostError> {
        self.registry
            .lock()
            .map_err(|_| TauriWindowHostError::StateUnavailable {
                state: "managed window registry".to_string(),
            })?
            .as_ref()
            .map(ManagedWindowRegistry::managed_windows)
            .ok_or(TauriWindowHostError::Busy)
    }

    /// Executes one apply without retaining a registry lock across injected calls.
    pub fn apply<F, B, Q>(
        &self,
        app: &AppHandle<R>,
        input: WindowDiffInput,
        mut factory: F,
        mut backend: B,
        mut readback: Q,
    ) -> Result<TauriWindowHostApplyReceipt, TauriWindowHostError>
    where
        F: TauriWindowFactory<R>,
        B: WindowMutationBackend<R>,
        Q: ManagedDesktopReadback<R>,
    {
        self.begin_exclusive(PHASE_APPLYING)?;
        let mut registry = match self.take_registry() {
            Ok(registry) => registry,
            Err(error) => {
                self.phase.store(PHASE_ACTIVE, Ordering::Release);
                return Err(error);
            }
        };
        let apply = execute_tauri_window_apply_in_place(
            app,
            input,
            &mut registry,
            &mut factory,
            &mut backend,
            &mut readback,
        );
        if let Err(error) = self.restore_registry(registry) {
            self.phase.store(PHASE_ACTIVE, Ordering::Release);
            return Err(error);
        }
        let receipt = match apply {
            Ok(receipt) => receipt,
            Err(error) => {
                self.phase.store(PHASE_ACTIVE, Ordering::Release);
                return Err(TauriWindowHostError::Apply(error));
            }
        };
        let reveal = self.lifecycle.mark_apply_converged(&receipt);
        self.phase.store(PHASE_ACTIVE, Ordering::Release);
        Ok(TauriWindowHostApplyReceipt { receipt, reveal })
    }

    /// Marks one renderer protocol ready for guarded reveal.
    pub fn mark_page_ready(
        &self,
        window_id: &WindowId,
    ) -> Result<WindowRevealReceipt, TauriWindowHostError> {
        self.lifecycle
            .mark_page_ready(window_id)
            .map_err(TauriWindowHostError::Lifecycle)
    }

    /// Handles a translated lifecycle event.
    pub fn handle_lifecycle_event(
        &self,
        event: WindowLifecycleEvent,
    ) -> Result<TauriWindowLifecycleReceipt, TauriWindowHostError> {
        self.lifecycle
            .handle_lifecycle_event(event)
            .map_err(TauriWindowHostError::Lifecycle)
    }

    /// Delivers an injected scheduler wake.
    pub fn handle_scheduled_wake(
        &self,
        wake: ScheduledWindowLifecycleWake,
    ) -> Result<TauriWindowLifecycleReceipt, TauriWindowHostError> {
        self.lifecycle
            .handle_scheduled_wake(wake)
            .map_err(TauriWindowHostError::Lifecycle)
    }

    /// Flushes once, deactivates listeners, and remains idempotent thereafter.
    pub fn teardown(&self) -> Result<TauriWindowHostTeardownReceipt, TauriWindowHostError> {
        match self.phase.compare_exchange(
            PHASE_ACTIVE,
            PHASE_TEARING_DOWN,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {}
            Err(PHASE_TORN_DOWN) => {
                return Ok(TauriWindowHostTeardownReceipt {
                    status: TauriWindowHostTeardownStatus::AlreadyTornDown,
                    shutdown: None,
                    deactivated_listeners: 0,
                });
            }
            Err(PHASE_APPLYING | PHASE_TEARING_DOWN) => {
                return Err(TauriWindowHostError::Busy);
            }
            Err(_) => return Err(TauriWindowHostError::Inactive),
        }
        let shutdown = match self.lifecycle.shutdown_flush() {
            Ok(receipt) => receipt,
            Err(error) => {
                self.phase.store(PHASE_ACTIVE, Ordering::Release);
                return Err(TauriWindowHostError::Lifecycle(error));
            }
        };
        let deactivated_listeners = match self.lifecycle.deactivate() {
            Ok(count) => count,
            Err(error) => {
                self.phase.store(PHASE_TORN_DOWN, Ordering::Release);
                return Err(TauriWindowHostError::Lifecycle(error));
            }
        };
        self.phase.store(PHASE_TORN_DOWN, Ordering::Release);
        Ok(TauriWindowHostTeardownReceipt {
            status: TauriWindowHostTeardownStatus::TornDown,
            shutdown: Some(shutdown),
            deactivated_listeners,
        })
    }

    fn begin_exclusive(&self, phase: u8) -> Result<(), TauriWindowHostError> {
        self.phase
            .compare_exchange(PHASE_ACTIVE, phase, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|current| match current {
                PHASE_APPLYING | PHASE_TEARING_DOWN => TauriWindowHostError::Busy,
                _ => TauriWindowHostError::Inactive,
            })
    }

    fn take_registry(&self) -> Result<ManagedWindowRegistry<R>, TauriWindowHostError> {
        self.registry
            .lock()
            .map_err(|_| TauriWindowHostError::StateUnavailable {
                state: "managed window registry".to_string(),
            })?
            .take()
            .ok_or(TauriWindowHostError::Busy)
    }

    fn restore_registry(
        &self,
        registry: ManagedWindowRegistry<R>,
    ) -> Result<(), TauriWindowHostError> {
        let mut slot =
            self.registry
                .lock()
                .map_err(|_| TauriWindowHostError::StateUnavailable {
                    state: "managed window registry".to_string(),
                })?;
        *slot = Some(registry);
        Ok(())
    }
}
