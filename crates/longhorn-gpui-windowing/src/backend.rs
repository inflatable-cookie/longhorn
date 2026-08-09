use std::{error::Error, fmt};

use longhorn_core::WindowId;

use crate::{GpuiDisplayFacts, GpuiLogicalRect, GpuiLogicalSize, GpuiWindowFacts, GpuiWindowKey};

/// Native GPUI failure normalized at the host boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiWindowError {
    detail: String,
}

impl GpuiWindowError {
    /// Constructs a native diagnostic.
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Returns the host diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for GpuiWindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for GpuiWindowError {}

/// Complete creation-time state for one GPUI window.
///
/// GPUI takes bounds, maximized state, target display and initial focus as
/// `WindowOptions` and offers no way to change the first two afterwards. So
/// the adapter must know the window's final placement before it exists, and
/// the pure planner's neutral-slot-then-mutate order cannot be executed
/// literally. See [`crate::gpui_host_capabilities`].
#[derive(Clone, Debug, PartialEq)]
pub struct GpuiWindowCreateRequest {
    bounds: GpuiLogicalRect,
    maximized: bool,
    focus_on_open: bool,
    display_id: Option<u32>,
}

impl GpuiWindowCreateRequest {
    /// Requests a window at explicit bounds.
    #[must_use]
    pub const fn new(bounds: GpuiLogicalRect) -> Self {
        Self {
            bounds,
            maximized: false,
            focus_on_open: false,
            display_id: None,
        }
    }

    /// Opens the window maximized, retaining these bounds as its restore size.
    #[must_use]
    pub const fn maximized(mut self) -> Self {
        self.maximized = true;
        self
    }

    /// Focuses the window as it opens.
    #[must_use]
    pub const fn focused(mut self) -> Self {
        self.focus_on_open = true;
        self
    }

    /// Opens the window on an explicit display.
    #[must_use]
    pub const fn on_display(mut self, display_id: u32) -> Self {
        self.display_id = Some(display_id);
        self
    }

    /// Returns requested bounds, or restore bounds when maximized.
    #[must_use]
    pub const fn bounds(&self) -> GpuiLogicalRect {
        self.bounds
    }

    /// Returns whether the window opens maximized.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns whether the window takes focus as it opens.
    #[must_use]
    pub const fn focuses_on_open(&self) -> bool {
        self.focus_on_open
    }

    /// Returns the explicit target display.
    #[must_use]
    pub const fn display_id(&self) -> Option<u32> {
        self.display_id
    }
}

/// Injectable boundary for native GPUI window calls.
///
/// This trait is deliberately **not** `Send + Sync`, and every method takes
/// `&mut self`. GPUI windows are reachable only through `&mut App` on the
/// platform's main thread. The Tauri seam's equivalents — `WindowCaptureBackend`,
/// `WindowRevealBackend`, `WindowPlacementSink` — are all `Send + Sync`,
/// because `tauri::WebviewWindow` is a cloneable cross-thread handle. An
/// implementation of this trait therefore holds the GPUI application context
/// for the duration of one apply pass; it cannot be stored in an `Arc` and
/// called from a worker.
///
/// The methods are the operations GPUI actually has. There is no `move`, no
/// `show` and no `hide`, because `PlatformWindow` has none of them.
pub trait GpuiWindowBackend {
    /// Returns whether this host can create new windows.
    fn can_create(&self) -> bool;

    /// Opens a window in its final placement and returns its slot.
    fn create(
        &mut self,
        window_id: &WindowId,
        request: &GpuiWindowCreateRequest,
    ) -> Result<GpuiWindowKey, GpuiWindowError>;

    /// Sets the content size. GPUI resizes about the window's current origin.
    fn resize(&mut self, key: GpuiWindowKey, size: GpuiLogicalSize) -> Result<(), GpuiWindowError>;

    /// Drives the window to an absolute maximized state.
    ///
    /// GPUI exposes `zoom()`, which toggles. An implementation reads
    /// `is_maximized` and toggles only on disagreement, so the operation is
    /// idempotent from Longhorn's side but not atomic: a user zoom between the
    /// read and the toggle inverts the result. Tauri has absolute `maximize`
    /// and `unmaximize` and no such window.
    fn set_maximized(&mut self, key: GpuiWindowKey, maximized: bool)
    -> Result<(), GpuiWindowError>;

    /// Brings the window forward and gives it key focus.
    fn activate(&mut self, key: GpuiWindowKey) -> Result<(), GpuiWindowError>;

    /// Removes the window.
    fn close(&mut self, key: GpuiWindowKey) -> Result<(), GpuiWindowError>;

    /// Reads complete live facts for one window.
    fn observe(&mut self, key: GpuiWindowKey) -> Result<GpuiWindowFacts, GpuiWindowError>;

    /// Reads every display GPUI currently knows about.
    fn displays(&mut self) -> Result<Vec<GpuiDisplayFacts>, GpuiWindowError>;
}
