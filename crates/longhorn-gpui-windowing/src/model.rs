use std::fmt;

use longhorn_core::{
    LiveWindowMetrics, RoundingMode, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize,
};
use longhorn_windowing::HostWindowHandle;
use serde::{Deserialize, Serialize};

/// Opaque process-local identity for one GPUI window.
///
/// GPUI identifies a window by a slot index, not by a caller-chosen label.
/// Longhorn's transport handle is a string, so the adapter renders the slot
/// into one. The rendering is the adapter's private business: Longhorn does
/// not interpret the handle, and nothing recovers the slot from it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct GpuiWindowKey(u64);

impl GpuiWindowKey {
    /// Records one GPUI window slot.
    #[must_use]
    pub const fn new(slot: u64) -> Self {
        Self(slot)
    }

    /// Returns the GPUI window slot.
    #[must_use]
    pub const fn slot(self) -> u64 {
        self.0
    }

    /// Returns the transport handle Longhorn's pure planner sees.
    ///
    /// Infallible: the rendering is ASCII and bounded, so it cannot violate
    /// `HostWindowHandle`'s syntax.
    #[must_use]
    pub fn transport_handle(self) -> HostWindowHandle {
        HostWindowHandle::new(self.to_string()).expect("rendered gpui window key is a valid handle")
    }
}

impl fmt::Display for GpuiWindowKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "gpui-window:{}", self.0)
    }
}

/// A logical-pixel rectangle exactly as GPUI reports it.
///
/// GPUI works in `f32` logical pixels. Longhorn's screen plane is integer
/// DIPs, so every value crossing this boundary is rounded explicitly rather
/// than truncated at a cast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuiLogicalRect {
    origin_x: f32,
    origin_y: f32,
    width: f32,
    height: f32,
}

impl GpuiLogicalRect {
    /// Records one GPUI `Bounds<Pixels>`.
    #[must_use]
    pub const fn new(origin_x: f32, origin_y: f32, width: f32, height: f32) -> Self {
        Self {
            origin_x,
            origin_y,
            width,
            height,
        }
    }

    /// Converts to the global screen plane with explicit nearest rounding.
    pub fn to_screen_rect(self) -> Result<ScreenRect, GpuiGeometryError> {
        Ok(ScreenRect::new(
            self.to_screen_origin()?,
            self.to_screen_size()?,
        ))
    }

    /// Converts only the origin.
    pub fn to_screen_origin(self) -> Result<ScreenPoint, GpuiGeometryError> {
        Ok(ScreenPoint::new(
            signed_dip(self.origin_x)?,
            signed_dip(self.origin_y)?,
        ))
    }

    /// Converts only the extent.
    pub fn to_screen_size(self) -> Result<ScreenSize, GpuiGeometryError> {
        Ok(ScreenSize::new(
            unsigned_dip(self.width)?,
            unsigned_dip(self.height)?,
        ))
    }
}

/// A logical-pixel size exactly as GPUI reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuiLogicalSize {
    width: f32,
    height: f32,
}

impl GpuiLogicalSize {
    /// Records one GPUI `Size<Pixels>`.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Converts to the global screen plane with explicit nearest rounding.
    pub fn to_screen_size(self) -> Result<ScreenSize, GpuiGeometryError> {
        Ok(ScreenSize::new(
            unsigned_dip(self.width)?,
            unsigned_dip(self.height)?,
        ))
    }

    /// Returns the logical width GPUI reported.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }

    /// Returns the logical height GPUI reported.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }
}

impl From<ScreenSize> for GpuiLogicalSize {
    fn from(value: ScreenSize) -> Self {
        // Widening an integer DIP into f32 is exact for every value a display
        // can report, so the round trip back through `to_screen_size` is
        // lossless in the direction Longhorn drives.
        Self::new(value.width() as f32, value.height() as f32)
    }
}

/// Logical/integer conversion failure at the GPUI edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuiGeometryError {
    /// GPUI reported a NaN or infinite coordinate.
    NonFinite,
    /// GPUI reported a negative extent.
    NegativeExtent,
    /// The rounded value left the representable screen plane.
    Overflow,
}

impl fmt::Display for GpuiGeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "gpui reported a non-finite logical coordinate",
            Self::NegativeExtent => "gpui reported a negative logical extent",
            Self::Overflow => "gpui logical coordinate left the screen plane",
        })
    }
}

impl std::error::Error for GpuiGeometryError {}

fn signed_dip(value: f32) -> Result<i32, GpuiGeometryError> {
    if !value.is_finite() {
        return Err(GpuiGeometryError::NonFinite);
    }
    let rounded = f64::from(value).round();
    if rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err(GpuiGeometryError::Overflow);
    }
    Ok(rounded as i32)
}

fn unsigned_dip(value: f32) -> Result<u32, GpuiGeometryError> {
    if !value.is_finite() {
        return Err(GpuiGeometryError::NonFinite);
    }
    if value < 0.0 {
        return Err(GpuiGeometryError::NegativeExtent);
    }
    let rounded = f64::from(value).round();
    if rounded > f64::from(u32::MAX) {
        return Err(GpuiGeometryError::Overflow);
    }
    Ok(rounded as u32)
}

/// GPUI's own window state, with the restore bounds it retains for itself.
///
/// GPUI reports the normal geometry of a maximized or fullscreen window. Tauri
/// does not, which is why Longhorn's Tauri capture threads a `retained_normal`
/// placement through every call. On GPUI that parameter has nothing to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuiWindowBoundsState {
    /// Ordinary windowed state. The bounds are the live bounds.
    Windowed(GpuiLogicalRect),
    /// Maximized. The bounds are the restore size.
    Maximized(GpuiLogicalRect),
    /// Fullscreen. The bounds are the restore size.
    Fullscreen(GpuiLogicalRect),
}

impl GpuiWindowBoundsState {
    /// Returns the geometry the window returns to when unmaximized.
    #[must_use]
    pub const fn restore_bounds(self) -> GpuiLogicalRect {
        match self {
            Self::Windowed(bounds) | Self::Maximized(bounds) | Self::Fullscreen(bounds) => bounds,
        }
    }

    /// Returns whether GPUI considers the window maximized.
    #[must_use]
    pub const fn is_maximized(self) -> bool {
        matches!(self, Self::Maximized(_))
    }

    /// Returns whether GPUI considers the window fullscreen.
    ///
    /// Longhorn's window vocabulary has no fullscreen state. It is reported so
    /// a caller can refuse to persist geometry captured in it, not so the
    /// planner can drive it.
    #[must_use]
    pub const fn is_fullscreen(self) -> bool {
        matches!(self, Self::Fullscreen(_))
    }
}

/// Everything GPUI can report about one live window.
///
/// `visible` is absent by construction. GPUI has no window visibility query
/// and no runtime show or hide; a window is on screen from creation until it
/// is removed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuiWindowFacts {
    bounds: GpuiLogicalRect,
    content_size: GpuiLogicalSize,
    bounds_state: GpuiWindowBoundsState,
    scale: f32,
    active: bool,
}

impl GpuiWindowFacts {
    /// Records one complete GPUI window observation.
    #[must_use]
    pub const fn new(
        bounds: GpuiLogicalRect,
        content_size: GpuiLogicalSize,
        bounds_state: GpuiWindowBoundsState,
        scale: f32,
        active: bool,
    ) -> Self {
        Self {
            bounds,
            content_size,
            bounds_state,
            scale,
            active,
        }
    }

    /// Returns live outer bounds in the global logical plane.
    #[must_use]
    pub const fn bounds(&self) -> GpuiLogicalRect {
        self.bounds
    }

    /// Returns live content size.
    #[must_use]
    pub const fn content_size(&self) -> GpuiLogicalSize {
        self.content_size
    }

    /// Returns maximized, fullscreen, or windowed state with restore bounds.
    #[must_use]
    pub const fn bounds_state(&self) -> GpuiWindowBoundsState {
        self.bounds_state
    }

    /// Returns the raw scale GPUI reported for this window's display.
    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    /// Returns whether the operating system considers this window active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Returns validated fixed-point scale.
    pub fn scale_factor(&self) -> Result<ScaleFactor, crate::GpuiScaleFactorError> {
        crate::scale_factor_from_gpui(self.scale)
    }

    /// Projects into Longhorn's live metrics vocabulary.
    pub fn to_live_metrics(&self) -> Result<LiveWindowMetrics, GpuiGeometryError> {
        Ok(LiveWindowMetrics::new(
            self.bounds.to_screen_rect()?,
            self.content_size.to_screen_size()?,
        ))
    }
}

/// Everything GPUI can report about one display.
///
/// GPUI's display API has three members: an id, a persistable UUID, and
/// logical bounds. There is no scale factor, no work area, and no built-in
/// flag. Those absences are the point of this type — see
/// [`crate::GpuiDisplayObservation`] for how they are reported rather than
/// invented.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuiDisplayFacts {
    display_id: u32,
    stable_uuid: Option<String>,
    bounds: GpuiLogicalRect,
    is_primary: bool,
}

impl GpuiDisplayFacts {
    /// Records one GPUI display observation.
    #[must_use]
    pub const fn new(
        display_id: u32,
        stable_uuid: Option<String>,
        bounds: GpuiLogicalRect,
        is_primary: bool,
    ) -> Self {
        Self {
            display_id,
            stable_uuid,
            bounds,
            is_primary,
        }
    }

    /// Returns the process-local GPUI display id.
    #[must_use]
    pub const fn display_id(&self) -> u32 {
        self.display_id
    }

    /// Returns the identity GPUI persists across system restarts.
    ///
    /// Tauri has no equivalent: its adapter matches monitors by name, position
    /// and size, which is why `probe_tauri_desktop` carries an ambiguity error.
    #[must_use]
    pub fn stable_uuid(&self) -> Option<&str> {
        self.stable_uuid.as_deref()
    }

    /// Returns full logical bounds.
    #[must_use]
    pub const fn bounds(&self) -> GpuiLogicalRect {
        self.bounds
    }

    /// Returns whether this is GPUI's primary display.
    #[must_use]
    pub const fn is_primary(&self) -> bool {
        self.is_primary
    }

    /// Converts logical bounds using a caller-supplied scale.
    ///
    /// The scale is a parameter because GPUI will not supply one. A caller
    /// that has no live window on this display has no scale to pass, and
    /// contract 020's display facts are unobtainable for it.
    pub fn physical_bounds(
        &self,
        scale: ScaleFactor,
    ) -> Result<longhorn_core::PhysicalRect, GpuiGeometryError> {
        scale
            .screen_rect_to_physical(self.bounds.to_screen_rect()?, RoundingMode::Nearest)
            .map_err(|_| GpuiGeometryError::Overflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_key_renders_into_a_valid_opaque_handle() {
        let key = GpuiWindowKey::new(7);

        assert_eq!(key.transport_handle().as_str(), "gpui-window:7");
        // The widest possible slot still renders well inside the handle's
        // 256-byte bound, so the infallible construction is honest.
        assert_eq!(
            GpuiWindowKey::new(u64::MAX).transport_handle().as_str(),
            "gpui-window:18446744073709551615"
        );
    }

    #[test]
    fn logical_pixels_round_to_nearest_rather_than_truncating() {
        // GPUI hands over f32. A cast would floor 99.6 to 99 and lose a pixel
        // every time a window is dragged onto a fractional-scale display.
        let rect = GpuiLogicalRect::new(10.4, -10.6, 99.6, 0.4);
        let screen = rect.to_screen_rect().unwrap();

        assert_eq!(screen.origin().x().get(), 10);
        assert_eq!(screen.origin().y().get(), -11);
        assert_eq!(screen.size().width(), 100);
        assert_eq!(screen.size().height(), 0);
    }

    #[test]
    fn non_finite_and_negative_logical_values_are_refused() {
        assert_eq!(
            GpuiLogicalRect::new(f32::NAN, 0.0, 1.0, 1.0).to_screen_origin(),
            Err(GpuiGeometryError::NonFinite)
        );
        assert_eq!(
            GpuiLogicalRect::new(0.0, 0.0, -1.0, 1.0).to_screen_size(),
            Err(GpuiGeometryError::NegativeExtent)
        );
        assert_eq!(
            GpuiLogicalRect::new(f32::MAX, 0.0, 1.0, 1.0).to_screen_origin(),
            Err(GpuiGeometryError::Overflow)
        );
    }

    #[test]
    fn a_maximized_window_still_reports_its_restore_geometry() {
        // The Tauri capture backend fails with "maximized capture has no
        // retained normal placement" unless the caller threads the previous
        // placement back in. GPUI keeps it, so the parameter is dead weight
        // on this host.
        let restore = GpuiLogicalRect::new(100.0, 100.0, 800.0, 600.0);
        let state = GpuiWindowBoundsState::Maximized(restore);

        assert!(state.is_maximized());
        assert_eq!(state.restore_bounds(), restore);
    }
}
