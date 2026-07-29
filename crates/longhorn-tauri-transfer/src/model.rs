use std::collections::BTreeSet;

use longhorn_core::{ScreenRect, WindowId};
use longhorn_transfer::LiveTransferWindow;
use longhorn_windowing::HostWindowHandle;

use crate::TransferRuntimeError;

/// Current checked screen-space geometry for one managed transfer window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedTransferWindow {
    window_id: WindowId,
    transport_handle: HostWindowHandle,
    outer_bounds: ScreenRect,
    content_bounds: ScreenRect,
}

impl ManagedTransferWindow {
    /// Constructs one already-mapped managed window.
    #[must_use]
    pub const fn new(
        window_id: WindowId,
        transport_handle: HostWindowHandle,
        outer_bounds: ScreenRect,
        content_bounds: ScreenRect,
    ) -> Self {
        Self {
            window_id,
            transport_handle,
            outer_bounds,
            content_bounds,
        }
    }

    /// Returns stable managed-window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the current Tauri transport handle.
    #[must_use]
    pub const fn transport_handle(&self) -> &HostWindowHandle {
        &self.transport_handle
    }

    /// Returns current logical outer-frame bounds.
    #[must_use]
    pub const fn outer_bounds(&self) -> ScreenRect {
        self.outer_bounds
    }

    /// Returns current logical webview-content bounds.
    #[must_use]
    pub const fn content_bounds(&self) -> ScreenRect {
        self.content_bounds
    }

    /// Projects the transfer core's fresh window evidence.
    #[must_use]
    pub fn live_window(&self) -> LiveTransferWindow {
        LiveTransferWindow::new(self.window_id.clone(), self.outer_bounds)
    }
}

/// One coherent current runtime snapshot bound to the invoking window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedTransferSnapshot {
    caller: ManagedTransferWindow,
    windows: Vec<ManagedTransferWindow>,
}

impl ManagedTransferSnapshot {
    /// Validates current unique identity and resolves the exact caller.
    pub fn new(
        caller_handle: &HostWindowHandle,
        windows: impl IntoIterator<Item = ManagedTransferWindow>,
    ) -> Result<Self, TransferRuntimeError> {
        let windows: Vec<_> = windows.into_iter().collect();
        let mut handles = BTreeSet::new();
        let mut window_ids = BTreeSet::new();
        let mut caller = None;
        for window in &windows {
            if !handles.insert(window.transport_handle().clone()) {
                return Err(TransferRuntimeError::DuplicateTransportHandle(
                    window.transport_handle().clone(),
                ));
            }
            if !window_ids.insert(window.window_id().clone()) {
                return Err(TransferRuntimeError::DuplicateWindowId(
                    window.window_id().clone(),
                ));
            }
            if window.transport_handle() == caller_handle {
                caller = Some(window.clone());
            }
        }
        Ok(Self {
            caller: caller
                .ok_or_else(|| TransferRuntimeError::UnmanagedCaller(caller_handle.clone()))?,
            windows,
        })
    }

    /// Returns current caller identity and geometry.
    #[must_use]
    pub const fn caller(&self) -> &ManagedTransferWindow {
        &self.caller
    }

    /// Returns all current managed transfer windows.
    #[must_use]
    pub fn windows(&self) -> &[ManagedTransferWindow] {
        self.windows.as_slice()
    }

    /// Returns fresh transfer-core window evidence.
    #[must_use]
    pub fn live_windows(&self) -> Vec<LiveTransferWindow> {
        self.windows
            .iter()
            .map(ManagedTransferWindow::live_window)
            .collect()
    }
}
