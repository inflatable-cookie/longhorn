//! Host handle and installed-window state.

use std::{
    collections::BTreeMap,
    sync::{Mutex, atomic::AtomicBool},
};

use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::{WindowLifecycleCoordinator, WindowLifecyclePolicy};
use tauri::{Runtime, WebviewWindow};

use crate::lifecycle::{
    TauriWindowLifecycleError, TauriWindowLifecycleServices, WindowFlushScope, WindowFlushTarget,
    WindowLifecycleWakeHandler,
};

pub(crate) struct InstalledWindow<R: Runtime> {
    pub(crate) window: WebviewWindow<R>,
    // Retained per axis, because placement is applied per axis now. A pure
    // move leaves the size untouched, and composing a normal placement from a
    // half-applied one would invent the other half.
    pub(crate) retained_origin: Option<longhorn_core::ScreenPoint>,
    pub(crate) retained_size: Option<longhorn_core::ScreenSize>,
    pub(crate) page_ready: bool,
    pub(crate) placement_ready: bool,
    pub(crate) reveal_started: bool,
    pub(crate) reveal_retry: bool,
    pub(crate) revealed: bool,
}

impl<R: Runtime> InstalledWindow<R> {
    /// Returns retained normal geometry only when both axes are known.
    pub(crate) fn retained_normal(&self) -> Option<WindowPlacement> {
        Some(WindowPlacement::new(
            self.retained_origin?,
            self.retained_size?,
        ))
    }

    pub(crate) fn set_retained_normal(&mut self, placement: Option<WindowPlacement>) {
        self.retained_origin = placement.map(|p| p.outer_origin());
        self.retained_size = placement.map(|p| p.inner_size());
    }
}

pub(crate) struct PendingFlush {
    pub(crate) target: WindowFlushTarget,
    pub(crate) timeout: longhorn_windowing::WindowLifecycleDuration,
    pub(crate) scope: WindowFlushScope,
}

pub(crate) enum FlushDisposition<'a> {
    /// Execute each flush at its directive position on the current thread.
    Inline,
    /// Record flushes for the caller to execute or defer.
    Collect(&'a mut Vec<PendingFlush>),
}

/// Tauri listener and I/O adapter over the pure lifecycle coordinator.
pub struct TauriWindowLifecycleHost<R: Runtime> {
    pub(crate) active: AtomicBool,
    pub(crate) coordinator: Mutex<WindowLifecycleCoordinator>,
    pub(crate) windows: Mutex<BTreeMap<WindowId, InstalledWindow<R>>>,
    pub(crate) services: TauriWindowLifecycleServices<R>,
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
}
