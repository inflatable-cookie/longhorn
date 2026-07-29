//! Runtime-generic assembly for complete managed Tauri window hosts.

use std::sync::{Arc, Mutex, atomic::AtomicU8};

use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::HostWindowHandle;
use tauri::{Runtime, WebviewWindow};

use crate::{
    ManagedWindowRegistry, ManagedWindowRegistryError, TauriApplyError, TauriApplyReceipt,
    TauriWindowLifecycleError, TauriWindowLifecycleHost, WindowRevealReceipt,
    WindowShutdownReceipt,
};

mod assembly;
mod host;

pub use assembly::{assemble_tauri_single_window_lifecycle_host, assemble_tauri_window_host};

pub(super) const PHASE_ACTIVE: u8 = 0;
pub(super) const PHASE_APPLYING: u8 = 1;
pub(super) const PHASE_TEARING_DOWN: u8 = 2;
pub(super) const PHASE_TORN_DOWN: u8 = 3;

/// One predeclared native window with stable identity and optional normal placement.
pub struct PredeclaredTauriWindow<R: Runtime> {
    pub(super) window_id: WindowId,
    pub(super) window: WebviewWindow<R>,
    pub(super) initial_normal: Option<WindowPlacement>,
}

impl<R: Runtime> PredeclaredTauriWindow<R> {
    /// Declares a managed window without retained normal-placement evidence.
    #[must_use]
    pub const fn new(window_id: WindowId, window: WebviewWindow<R>) -> Self {
        Self {
            window_id,
            window,
            initial_normal: None,
        }
    }

    /// Adds the last known normal placement used while the window is maximized.
    #[must_use]
    pub const fn with_initial_normal(mut self, placement: WindowPlacement) -> Self {
        self.initial_normal = Some(placement);
        self
    }
}

/// Stable identity recorded for one installed lifecycle listener.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriWindowRegistrationReceipt {
    window_id: WindowId,
    transport_handle: HostWindowHandle,
}

impl TauriWindowRegistrationReceipt {
    pub(super) fn new(window_id: WindowId, transport_handle: HostWindowHandle) -> Self {
        Self {
            window_id,
            transport_handle,
        }
    }

    /// Returns the logical window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the immutable Tauri-label transport handle.
    #[must_use]
    pub const fn transport_handle(&self) -> &HostWindowHandle {
        &self.transport_handle
    }
}

/// Whether assembly built a host or returned the process-local existing host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TauriWindowHostInitializationStatus {
    /// The host and all predeclared listeners were installed.
    Initialized,
    /// The app already owned this host; no listeners were installed again.
    Reused,
}

/// Complete assembly receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriWindowHostInitializationReceipt {
    pub(super) status: TauriWindowHostInitializationStatus,
    pub(super) registrations: Vec<TauriWindowRegistrationReceipt>,
}

impl TauriWindowHostInitializationReceipt {
    /// Returns whether this call initialized or reused the host.
    #[must_use]
    pub const fn status(&self) -> TauriWindowHostInitializationStatus {
        self.status
    }

    /// Returns the predeclared listener registrations.
    #[must_use]
    pub fn registrations(&self) -> &[TauriWindowRegistrationReceipt] {
        &self.registrations
    }
}

/// Host plus its assembly receipt.
pub struct TauriWindowHostInitialization<R: Runtime> {
    pub(super) host: Arc<TauriWindowHost<R>>,
    pub(super) receipt: TauriWindowHostInitializationReceipt,
}

impl<R: Runtime> TauriWindowHostInitialization<R> {
    /// Returns the reusable host.
    #[must_use]
    pub const fn host(&self) -> &Arc<TauriWindowHost<R>> {
        &self.host
    }

    /// Returns the assembly result.
    #[must_use]
    pub const fn receipt(&self) -> &TauriWindowHostInitializationReceipt {
        &self.receipt
    }

    /// Consumes the result.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        Arc<TauriWindowHost<R>>,
        TauriWindowHostInitializationReceipt,
    ) {
        (self.host, self.receipt)
    }
}

/// Fatal assembly failure with exact boundary evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TauriWindowHostInitializationError {
    /// Tauri app state or host state could not be locked.
    StateUnavailable {
        /// State category.
        state: String,
    },
    /// Lifecycle scheduler binding failed before listener installation.
    Lifecycle {
        /// Exact lifecycle diagnostic.
        source: TauriWindowLifecycleError,
    },
    /// Initial managed identity or protected-primary policy was invalid.
    Registry {
        /// Registry diagnostic.
        source: ManagedWindowRegistryError,
    },
    /// A predeclared native listener could not be installed.
    Listener {
        /// Logical target.
        window_id: WindowId,
        /// Native target.
        transport_handle: HostWindowHandle,
        /// Lifecycle diagnostic.
        source: TauriWindowLifecycleError,
    },
}

/// Complete reusable host over apply, lifecycle, reveal, shutdown, and teardown.
pub struct TauriWindowHost<R: Runtime> {
    pub(super) lifecycle: Arc<TauriWindowLifecycleHost<R>>,
    pub(super) registry: Mutex<Option<ManagedWindowRegistry<R>>>,
    pub(super) registrations: Vec<TauriWindowRegistrationReceipt>,
    pub(super) phase: AtomicU8,
}

/// Host-level fatal failure before a complete receipt was available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TauriWindowHostError {
    /// Another exclusive host operation is running.
    Busy,
    /// Teardown has deactivated the host.
    Inactive,
    /// Shared host state was unavailable.
    StateUnavailable {
        /// State category.
        state: String,
    },
    /// Planning or generation registration failed before operation receipts.
    Apply(TauriApplyError),
    /// Lifecycle work failed before its complete receipt.
    Lifecycle(TauriWindowLifecycleError),
}

/// Apply receipt plus the guarded-reveal result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriWindowHostApplyReceipt {
    pub(super) receipt: TauriApplyReceipt,
    pub(super) reveal: Result<Vec<WindowRevealReceipt>, TauriWindowLifecycleError>,
}

impl TauriWindowHostApplyReceipt {
    /// Returns native operation and convergence evidence.
    #[must_use]
    pub const fn apply(&self) -> &TauriApplyReceipt {
        &self.receipt
    }

    /// Returns reveal progress or its typed failure.
    pub const fn reveal(&self) -> &Result<Vec<WindowRevealReceipt>, TauriWindowLifecycleError> {
        &self.reveal
    }
}

/// Teardown transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TauriWindowHostTeardownStatus {
    /// Shutdown flush and listener deactivation ran.
    TornDown,
    /// A prior call already completed teardown.
    AlreadyTornDown,
}

/// Idempotent teardown receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriWindowHostTeardownReceipt {
    pub(super) status: TauriWindowHostTeardownStatus,
    pub(super) shutdown: Option<WindowShutdownReceipt>,
    pub(super) deactivated_listeners: usize,
}

impl TauriWindowHostTeardownReceipt {
    /// Returns whether this call performed teardown.
    #[must_use]
    pub const fn status(&self) -> TauriWindowHostTeardownStatus {
        self.status
    }

    /// Returns shutdown capture and flush evidence when performed.
    #[must_use]
    pub const fn shutdown(&self) -> Option<&WindowShutdownReceipt> {
        self.shutdown.as_ref()
    }

    /// Returns the number of listener targets deactivated.
    #[must_use]
    pub const fn deactivated_listeners(&self) -> usize {
        self.deactivated_listeners
    }
}
