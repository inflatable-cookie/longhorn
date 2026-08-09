//! Binds `longhorn-gpui-windowing`'s host seam to real GPUI.
//!
//! This crate exists to prove that [`GpuiWindowBackend`] is satisfiable by
//! `gpui` itself, and to be the place the adapter's shape was measured. It is
//! excluded from the Longhorn workspace, like every other `prototypes/` crate:
//! `gpui` pulls several hundred transitive crates and a Metal shader build,
//! and putting that in `effigy qa` would tax every lane in the repository for
//! one adapter. `packages/gpui/adapter` in Poodle draws the same line — its
//! adapter has no `gpui` dependency and only its preview binary does.
//!
//! Because it is outside the gate, the workspace crate's tests run against an
//! in-memory fake instead. That fake implements exactly what
//! `gpui::PlatformWindow` offers and nothing more, and this crate is the proof
//! that the list is right.

use std::{collections::BTreeMap, rc::Rc};

use gpui::{
    AnyWindowHandle, App, AppContext, Bounds, Context, DisplayId, IntoElement, Pixels,
    PlatformDisplay, Render, Window, WindowBounds, WindowOptions, div, point, px, size,
};
use longhorn_core::WindowId;
use longhorn_gpui_windowing::{
    GpuiDisplayFacts, GpuiLogicalRect, GpuiLogicalSize, GpuiWindowBackend, GpuiWindowBoundsState,
    GpuiWindowCreateRequest, GpuiWindowError, GpuiWindowFacts, GpuiWindowKey,
};

/// The root view of a Longhorn-managed GPUI window.
///
/// Longhorn does not own what a window renders — that is the projection tier's
/// business. The host adapter needs a root view only because `open_window`
/// requires one.
pub struct HostWindowRoot;

impl Render for HostWindowRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A [`GpuiWindowBackend`] over a live GPUI application context.
///
/// The borrow is the point. `App` is reachable only on the platform main
/// thread, so this type cannot be `Send`, cannot be stored in an `Arc`, and
/// cannot outlive the callback it was constructed in. The Tauri equivalents
/// are all `Send + Sync` because `tauri::WebviewWindow` is a cloneable
/// cross-thread handle.
pub struct GpuiAppBackend<'app> {
    app: &'app mut App,
    windows: BTreeMap<u64, AnyWindowHandle>,
    can_create: bool,
}

impl<'app> GpuiAppBackend<'app> {
    /// Borrows a GPUI application context for one apply pass.
    #[must_use]
    pub fn new(app: &'app mut App) -> Self {
        Self {
            app,
            windows: BTreeMap::new(),
            can_create: true,
        }
    }

    /// Declares that this host must not open new windows.
    #[must_use]
    pub const fn without_create(mut self) -> Self {
        self.can_create = false;
        self
    }

    /// Adopts a window opened before the adapter took over.
    pub fn adopt(&mut self, handle: AnyWindowHandle) -> GpuiWindowKey {
        let key = GpuiWindowKey::new(handle.window_id().as_u64());
        self.windows.insert(key.slot(), handle);
        key
    }

    fn handle(&self, key: GpuiWindowKey) -> Result<AnyWindowHandle, GpuiWindowError> {
        self.windows
            .get(&key.slot())
            .copied()
            .ok_or_else(|| GpuiWindowError::new(format!("{key} is not an adopted gpui window")))
    }

    fn with_window<R>(
        &mut self,
        key: GpuiWindowKey,
        update: impl FnOnce(&mut Window) -> R,
    ) -> Result<R, GpuiWindowError> {
        let handle = self.handle(key)?;
        handle
            .update(self.app, |_view, window, _cx| update(window))
            .map_err(|error| GpuiWindowError::new(error.to_string()))
    }
}

impl GpuiWindowBackend for GpuiAppBackend<'_> {
    fn can_create(&self) -> bool {
        self.can_create
    }

    fn create(
        &mut self,
        _window_id: &WindowId,
        request: &GpuiWindowCreateRequest,
    ) -> Result<GpuiWindowKey, GpuiWindowError> {
        let bounds = to_gpui_bounds(request.bounds())?;
        let display_id = match request.display_id() {
            // `DisplayId` has no public constructor, so a target display is
            // resolved by matching an id read back from `App::displays`
            // rather than by minting one.
            Some(wanted) => Some(
                self.app
                    .displays()
                    .iter()
                    .map(|display| display.id())
                    .find(|id| u32::from(*id) == wanted)
                    .ok_or_else(|| GpuiWindowError::new(format!("gpui has no display {wanted}")))?,
            ),
            None => None,
        };
        let options = WindowOptions {
            // Bounds, maximized state, target display and initial focus are
            // all creation-time only. Two of the four cannot be changed
            // afterwards, which is why the adapter composes them here from
            // desired state instead of executing the plan's neutral-slot
            // sequence.
            window_bounds: Some(if request.is_maximized() {
                WindowBounds::Maximized(bounds)
            } else {
                WindowBounds::Windowed(bounds)
            }),
            focus: request.focuses_on_open(),
            show: true,
            display_id,
            ..Default::default()
        };
        let handle = self
            .app
            .open_window(options, |_window, cx| cx.new(|_cx| HostWindowRoot))
            .map_err(|error| GpuiWindowError::new(error.to_string()))?;
        Ok(self.adopt(handle.into()))
    }

    fn resize(
        &mut self,
        key: GpuiWindowKey,
        new_size: GpuiLogicalSize,
    ) -> Result<(), GpuiWindowError> {
        self.with_window(key, |window| {
            window.resize(size(px(new_size.width()), px(new_size.height())));
        })
    }

    fn set_maximized(
        &mut self,
        key: GpuiWindowKey,
        maximized: bool,
    ) -> Result<(), GpuiWindowError> {
        // `zoom_window` toggles. There is no absolute setter, so the only way
        // to reach a requested state is to read first and act on disagreement.
        // The read and the toggle are not atomic: a user zoom in between
        // inverts the result.
        self.with_window(key, |window| {
            if window.is_maximized() != maximized {
                window.zoom_window();
            }
        })
    }

    fn activate(&mut self, key: GpuiWindowKey) -> Result<(), GpuiWindowError> {
        self.with_window(key, |window| window.activate_window())
    }

    fn close(&mut self, key: GpuiWindowKey) -> Result<(), GpuiWindowError> {
        self.with_window(key, Window::remove_window)?;
        self.windows.remove(&key.slot());
        Ok(())
    }

    fn observe(&mut self, key: GpuiWindowKey) -> Result<GpuiWindowFacts, GpuiWindowError> {
        self.with_window(key, |window| {
            GpuiWindowFacts::new(
                from_gpui_bounds(window.bounds()),
                GpuiLogicalSize::new(
                    f32::from(window.viewport_size().width),
                    f32::from(window.viewport_size().height),
                ),
                bounds_state(window),
                window.scale_factor(),
                window.is_window_active(),
            )
        })
    }

    fn displays(&mut self) -> Result<Vec<GpuiDisplayFacts>, GpuiWindowError> {
        let primary = self.app.primary_display().map(|display| display.id());
        Ok(self
            .app
            .displays()
            .iter()
            .map(|display| display_facts(display, primary))
            .collect())
    }
}

fn bounds_state(window: &Window) -> GpuiWindowBoundsState {
    // GPUI reports the restore bounds of a maximized or fullscreen window
    // itself, so there is nothing for the caller to retain. The Tauri capture
    // backend fails without a `retained_normal` placement threaded back in.
    match window.window_bounds() {
        WindowBounds::Windowed(bounds) => GpuiWindowBoundsState::Windowed(from_gpui_bounds(bounds)),
        WindowBounds::Maximized(bounds) => {
            GpuiWindowBoundsState::Maximized(from_gpui_bounds(bounds))
        }
        WindowBounds::Fullscreen(bounds) => {
            GpuiWindowBoundsState::Fullscreen(from_gpui_bounds(bounds))
        }
    }
}

fn display_facts(
    display: &Rc<dyn PlatformDisplay>,
    primary: Option<DisplayId>,
) -> GpuiDisplayFacts {
    // Three facts, and that is all `PlatformDisplay` has. No scale factor, no
    // work area, no built-in flag — the adapter reports their absence rather
    // than inventing them.
    GpuiDisplayFacts::new(
        u32::from(display.id()),
        display.uuid().ok().map(|uuid| uuid.to_string()),
        from_gpui_bounds(display.bounds()),
        primary == Some(display.id()),
    )
}

fn to_gpui_bounds(rect: GpuiLogicalRect) -> Result<Bounds<Pixels>, GpuiWindowError> {
    let origin = rect
        .to_screen_origin()
        .map_err(|error| GpuiWindowError::new(error.to_string()))?;
    let extent = rect
        .to_screen_size()
        .map_err(|error| GpuiWindowError::new(error.to_string()))?;
    Ok(Bounds {
        origin: point(px(origin.x().get() as f32), px(origin.y().get() as f32)),
        size: size(px(extent.width() as f32), px(extent.height() as f32)),
    })
}

fn from_gpui_bounds(bounds: Bounds<Pixels>) -> GpuiLogicalRect {
    GpuiLogicalRect::new(
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    )
}

/// Reads a display's backing scale factor without opening a window on it.
///
/// GPUI's `PlatformDisplay` reports no scale, and `Window::scale_factor` needs
/// a window — which looks like it makes a display's scale unknowable until
/// something has been placed there. It does not. `MacDisplay` is a newtype
/// over `CGDirectDisplayID`, and `DisplayId` exposes it through
/// `impl From<DisplayId> for u32`, so the id GPUI already hands over is
/// exactly the key CoreGraphics wants.
///
/// The scale is the ratio of the current mode's pixel width to its point
/// width: a 2× panel reports twice as many pixels as points. Safe bindings,
/// so this holds under the crate's `unsafe_code = "forbid"`.
///
/// Returns `None` when the display has no current mode — asleep, or
/// disconnected between the enumeration and this call.
#[must_use]
pub fn display_scale_factor(display_id: u32) -> Option<f32> {
    let mode = core_graphics::display::CGDisplay::new(display_id).display_mode()?;
    let points = mode.width();
    if points == 0 {
        return None;
    }
    Some(mode.pixel_width() as f32 / points as f32)
}

/// Reads a display's origin in the global plane, which GPUI discards.
///
/// `MacDisplay::bounds` reads `CGDisplayBounds` — documented in gpui's own
/// source as global coordinates — and then substitutes `Default::default()`
/// for the origin, so every display reports `(0, 0)`. The same
/// `CGDirectDisplayID` gpui exposes reads the real value straight back.
///
/// Returns `(x, y)` in points, top-left origin, as CoreGraphics reports it.
#[must_use]
pub fn display_origin(display_id: u32) -> (f64, f64) {
    let bounds = core_graphics::display::CGDisplay::new(display_id).bounds();
    (bounds.origin.x, bounds.origin.y)
}
