use std::sync::{Arc, Mutex, Weak, atomic::AtomicU8};

use longhorn_core::WindowId;
use longhorn_windowing::{HostWindowHandle, WindowLifecyclePolicy};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

use super::{
    PHASE_ACTIVE, PredeclaredTauriWindow, TauriWindowHost, TauriWindowHostInitialization,
    TauriWindowHostInitializationError, TauriWindowHostInitializationReceipt,
    TauriWindowHostInitializationStatus, TauriWindowRegistrationReceipt,
};
use crate::apply::ManagedWindowRegistration;
use crate::{
    ManagedWebviewWindow, ManagedWindowRegistry, ProgrammaticApplyObserver,
    TauriWindowLifecycleError, TauriWindowLifecycleHost, TauriWindowLifecycleServices,
};

struct TauriWindowHostSlot<R: Runtime>(Mutex<Option<Arc<TauriWindowHost<R>>>>);

impl<R: Runtime> Default for TauriWindowHostSlot<R> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

struct LifecycleWindowRegistration<R: Runtime>(Weak<TauriWindowLifecycleHost<R>>);

impl<R: Runtime> ManagedWindowRegistration<R> for LifecycleWindowRegistration<R> {
    fn register_window(
        &self,
        window_id: &WindowId,
        window: &WebviewWindow<R>,
    ) -> Result<(), String> {
        let host = self
            .0
            .upgrade()
            .ok_or_else(|| "window lifecycle host is unavailable".to_string())?;
        let initial_normal = host.capture_initial_normal(window_id, window);
        host.install_window(window_id.clone(), window.clone(), initial_normal)
            .map_err(|error| format!("{error:?}"))
    }

    fn retag_window(
        &self,
        transport_handle: &HostWindowHandle,
        window_id: &WindowId,
    ) -> Result<(), String> {
        self.0
            .upgrade()
            .ok_or_else(|| "window lifecycle host is unavailable".to_string())?
            .retag_window(transport_handle, window_id.clone())
            .map_err(|error| format!("{error:?}"))
    }
}

/// Assembles the lifecycle-only path for one predeclared window.
///
/// Listener installation is included. Consumers must not register a second
/// `on_window_event` callback that forwards the same event to this host.
pub fn assemble_tauri_single_window_lifecycle_host<R>(
    policy: WindowLifecyclePolicy,
    services: TauriWindowLifecycleServices<R>,
    window: PredeclaredTauriWindow<R>,
) -> Result<Arc<TauriWindowLifecycleHost<R>>, TauriWindowLifecycleError>
where
    R: Runtime,
{
    let lifecycle = TauriWindowLifecycleHost::shared(policy, services)?;
    lifecycle.install_window(window.window_id, window.window, window.initial_normal)?;
    Ok(lifecycle)
}

/// Assembles or reuses the app's complete runtime-generic window host.
pub fn assemble_tauri_window_host<R>(
    app: &AppHandle<R>,
    policy: WindowLifecyclePolicy,
    services: TauriWindowLifecycleServices<R>,
    windows: impl IntoIterator<Item = PredeclaredTauriWindow<R>>,
    protected_primary: Option<HostWindowHandle>,
) -> Result<TauriWindowHostInitialization<R>, TauriWindowHostInitializationError>
where
    R: Runtime,
{
    if app.try_state::<TauriWindowHostSlot<R>>().is_none() {
        let _ = app.manage(TauriWindowHostSlot::<R>::default());
    }
    let state = app.try_state::<TauriWindowHostSlot<R>>().ok_or_else(|| {
        TauriWindowHostInitializationError::StateUnavailable {
            state: "Tauri managed host slot".to_string(),
        }
    })?;
    let mut slot =
        state
            .0
            .lock()
            .map_err(|_| TauriWindowHostInitializationError::StateUnavailable {
                state: "Tauri managed host slot".to_string(),
            })?;
    if let Some(host) = slot.as_ref() {
        return Ok(TauriWindowHostInitialization {
            host: host.clone(),
            receipt: TauriWindowHostInitializationReceipt {
                status: TauriWindowHostInitializationStatus::Reused,
                registrations: host.registrations.clone(),
            },
        });
    }

    let windows: Vec<_> = windows.into_iter().collect();
    let managed = windows.iter().map(|window| {
        ManagedWebviewWindow::new(Some(window.window_id.clone()), window.window.clone())
    });
    let registry = ManagedWindowRegistry::new(managed, protected_primary)
        .map_err(|source| TauriWindowHostInitializationError::Registry { source })?;
    let lifecycle = TauriWindowLifecycleHost::shared(policy, services)
        .map_err(|source| TauriWindowHostInitializationError::Lifecycle { source })?;
    let mut registrations = Vec::with_capacity(windows.len());
    for predeclared in windows {
        let transport_handle = HostWindowHandle::new(predeclared.window.label())
            .expect("registry already validated every Tauri label");
        lifecycle
            .install_window(
                predeclared.window_id.clone(),
                predeclared.window,
                predeclared.initial_normal,
            )
            .map_err(|source| TauriWindowHostInitializationError::Listener {
                window_id: predeclared.window_id.clone(),
                transport_handle: transport_handle.clone(),
                source,
            })?;
        registrations.push(TauriWindowRegistrationReceipt::new(
            predeclared.window_id,
            transport_handle,
        ));
    }
    let apply_observer: Arc<dyn ProgrammaticApplyObserver> = lifecycle.clone();
    let window_registration: Arc<dyn ManagedWindowRegistration<R>> =
        Arc::new(LifecycleWindowRegistration(Arc::downgrade(&lifecycle)));
    let registry = registry
        .with_apply_observer(apply_observer)
        .with_window_registration(window_registration);
    let host = Arc::new(TauriWindowHost {
        lifecycle,
        registry: Mutex::new(Some(registry)),
        registrations: registrations.clone(),
        phase: AtomicU8::new(PHASE_ACTIVE),
    });
    *slot = Some(host.clone());
    Ok(TauriWindowHostInitialization {
        host,
        receipt: TauriWindowHostInitializationReceipt {
            status: TauriWindowHostInitializationStatus::Initialized,
            registrations,
        },
    })
}
