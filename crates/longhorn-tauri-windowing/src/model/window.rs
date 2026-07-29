use longhorn_core::{PhysicalRect, PhysicalSize, ScaleFactor, WindowId};
use longhorn_display::ObservedDisplay;
use longhorn_windowing::{HostWindowHandle, LiveWindow};
use serde::{Deserialize, Serialize};
use tauri::{Runtime, WebviewWindow};

use super::PhysicalDisplayObservation;

/// Raw complete facts for one explicitly managed Tauri window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhysicalLiveWindowObservation {
    window_id: Option<WindowId>,
    transport_handle: HostWindowHandle,
    outer_bounds: PhysicalRect,
    inner_size: PhysicalSize,
    scale: ScaleFactor,
    maximized: bool,
    visible: bool,
    focused: bool,
}

impl PhysicalLiveWindowObservation {
    /// Constructs one complete physical window observation.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        window_id: Option<WindowId>,
        transport_handle: HostWindowHandle,
        outer_bounds: PhysicalRect,
        inner_size: PhysicalSize,
        scale: ScaleFactor,
        maximized: bool,
        visible: bool,
        focused: bool,
    ) -> Self {
        Self {
            window_id,
            transport_handle,
            outer_bounds,
            inner_size,
            scale,
            maximized,
            visible,
            focused,
        }
    }

    /// Returns caller-supplied stable identity.
    #[must_use]
    pub const fn window_id(&self) -> Option<&WindowId> {
        self.window_id.as_ref()
    }

    /// Returns the Tauri-label transport handle.
    #[must_use]
    pub const fn transport_handle(&self) -> &HostWindowHandle {
        &self.transport_handle
    }

    /// Returns physical outer-frame bounds.
    #[must_use]
    pub const fn outer_bounds(&self) -> PhysicalRect {
        self.outer_bounds
    }

    /// Returns physical inner content size.
    #[must_use]
    pub const fn inner_size(&self) -> PhysicalSize {
        self.inner_size
    }

    /// Returns validated scale evidence.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }

    /// Returns maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns visibility state.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Returns focus state.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.focused
    }
}

/// Complete raw desktop input for one coordinate-mapping pass.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhysicalDesktopSnapshot {
    displays: Vec<PhysicalDisplayObservation>,
    windows: Vec<PhysicalLiveWindowObservation>,
}

impl PhysicalDesktopSnapshot {
    /// Constructs one complete raw desktop snapshot.
    #[must_use]
    pub fn new(
        displays: impl IntoIterator<Item = PhysicalDisplayObservation>,
        windows: impl IntoIterator<Item = PhysicalLiveWindowObservation>,
    ) -> Self {
        Self {
            displays: displays.into_iter().collect(),
            windows: windows.into_iter().collect(),
        }
    }

    /// Returns all raw display observations.
    #[must_use]
    pub fn displays(&self) -> &[PhysicalDisplayObservation] {
        &self.displays
    }

    /// Returns all explicitly managed raw window observations.
    #[must_use]
    pub fn windows(&self) -> &[PhysicalLiveWindowObservation] {
        &self.windows
    }
}

/// A Tauri webview window plus optional caller-owned stable identity.
pub struct ManagedWebviewWindow<R: Runtime> {
    window_id: Option<WindowId>,
    window: WebviewWindow<R>,
}

impl<R: Runtime> Clone for ManagedWebviewWindow<R> {
    fn clone(&self) -> Self {
        Self {
            window_id: self.window_id.clone(),
            window: self.window.clone(),
        }
    }
}

impl<R: Runtime> ManagedWebviewWindow<R> {
    /// Marks one webview window as managed by the current observation.
    #[must_use]
    pub const fn new(window_id: Option<WindowId>, window: WebviewWindow<R>) -> Self {
        Self { window_id, window }
    }

    /// Returns caller-owned stable identity.
    #[must_use]
    pub const fn window_id(&self) -> Option<&WindowId> {
        self.window_id.as_ref()
    }

    pub(crate) const fn window(&self) -> &WebviewWindow<R> {
        &self.window
    }

    pub(crate) fn set_window_id(&mut self, window_id: WindowId) {
        self.window_id = Some(window_id);
    }
}

/// Complete logical observation ready for pure correlation and diff planning.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DesktopObservation {
    displays: Vec<ObservedDisplay>,
    windows: Vec<LiveWindow>,
}

impl DesktopObservation {
    /// Constructs a complete already-mapped desktop observation.
    #[must_use]
    pub const fn new(displays: Vec<ObservedDisplay>, windows: Vec<LiveWindow>) -> Self {
        Self { displays, windows }
    }

    /// Returns display observations without canonical ids.
    #[must_use]
    pub fn displays(&self) -> &[ObservedDisplay] {
        &self.displays
    }

    /// Returns the complete explicitly managed live-window snapshot.
    #[must_use]
    pub fn windows(&self) -> &[LiveWindow] {
        &self.windows
    }
}
