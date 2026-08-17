//! The production [`NativeDesktopPlane`] over AppKit.
//!
//! Reads the logical desktop macOS already composites, rather than
//! reconstructing one from Tauri's physical facts. See the module header for
//! why that distinction is load-bearing.
//!
//! # Main thread
//!
//! `NSScreen` is main-thread-only. This reader says so with a
//! typed refusal rather than reaching for a thread-unsafe shortcut: a desktop
//! observation taken off the main thread would be reading geometry the window
//! server may be changing underneath it.

use longhorn_core::{PhysicalSize, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize};
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_foundation::NSRect;

use super::{NativeDesktopPlane, NativeDisplayGeometry};

/// Reads the live macOS desktop plane.
///
/// Stateless: the plane is a property of the machine, not of the application,
/// and windows are converted rather than read (see the module header).
#[derive(Clone, Copy, Debug, Default)]
pub struct AppKitDesktopPlane;

/// The primary screen's height in points, which anchors the flip.
///
/// AppKit measures from the bottom-left of the primary screen with y growing
/// up; `ScreenDip` measures from the top-left with y growing down. The primary
/// screen is the one at the origin, which is also what Core Graphics calls the
/// main display.
fn primary_height(screens: &[&NSScreen]) -> Option<f64> {
    screens
        .iter()
        .map(|screen| screen.frame())
        .find(|frame| frame.origin.x == 0.0 && frame.origin.y == 0.0)
        .map(|frame| frame.size.height)
}

/// Converts one AppKit rect into the global top-left logical plane.
fn to_screen_rect(rect: NSRect, primary_height: f64, label: &str) -> Result<ScreenRect, String> {
    let x = rect.origin.x;
    let y = primary_height - (rect.origin.y + rect.size.height);
    let width = rect.size.width;
    let height = rect.size.height;

    if ![x, y, width, height].iter().all(|value| value.is_finite()) {
        return Err(format!("{label} has a non-finite coordinate"));
    }
    if width < 0.0 || height < 0.0 {
        return Err(format!("{label} has a negative extent"));
    }

    // Nearest, named per contract 009. AppKit points are integral for displays
    // and may be fractional for a window the user has dragged.
    let x = round_to_i32(x, label)?;
    let y = round_to_i32(y, label)?;
    let width = round_to_u32(width, label)?;
    let height = round_to_u32(height, label)?;

    Ok(ScreenRect::new(
        ScreenPoint::new(x, y),
        ScreenSize::new(width, height),
    ))
}

fn round_to_i32(value: f64, label: &str) -> Result<i32, String> {
    let rounded = value.round();
    if rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err(format!(
            "{label} coordinate {value} does not fit a screen plane"
        ));
    }
    Ok(rounded as i32)
}

fn round_to_u32(value: f64, label: &str) -> Result<u32, String> {
    let rounded = value.round();
    if rounded < 0.0 || rounded > f64::from(u32::MAX) {
        return Err(format!(
            "{label} extent {value} does not fit a screen plane"
        ));
    }
    Ok(rounded as u32)
}

/// The physical size Tauri reports for a screen: points times its own scale.
///
/// Used only to correlate this screen with the observation describing it. If
/// the host ever stops deriving physical facts this way, correlation fails and
/// the mapper refuses — which is the intended outcome, because the geometry
/// this reader returns would no longer be describing the same displays.
fn correlation_size(frame: NSRect, scale: f64, label: &str) -> Result<PhysicalSize, String> {
    let width = round_to_u32(frame.size.width * scale, label)?;
    let height = round_to_u32(frame.size.height * scale, label)?;
    Ok(PhysicalSize::new(width, height))
}

fn scale_factor(scale: f64, label: &str) -> Result<ScaleFactor, String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(format!("{label} reports an unusable backing scale {scale}"));
    }
    let thousandths = round_to_u32(scale * 1000.0, label)?;
    ScaleFactor::from_thousandths(thousandths)
        .map_err(|error| format!("{label} backing scale {scale} is invalid: {error}"))
}

impl NativeDesktopPlane for AppKitDesktopPlane {
    fn displays(&self) -> Result<Vec<NativeDisplayGeometry>, String> {
        let mtm = MainThreadMarker::new()
            .ok_or("the macOS desktop plane must be read on the main thread")?;
        let screens = NSScreen::screens(mtm).to_vec();
        let borrowed = screens
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<_>>();

        let primary = primary_height(&borrowed)
            .ok_or("no macOS screen sits at the plane origin; the desktop has no primary")?;

        borrowed
            .iter()
            .map(|screen| {
                let frame = screen.frame();
                let scale = screen.backingScaleFactor();
                let label = "macOS screen";
                Ok(NativeDisplayGeometry::new(
                    to_screen_rect(frame, primary, label)?,
                    to_screen_rect(screen.visibleFrame(), primary, label)?,
                    correlation_size(frame, scale, label)?,
                    scale_factor(scale, label)?,
                    frame.origin.x == 0.0 && frame.origin.y == 0.0,
                ))
            })
            .collect()
    }
}
