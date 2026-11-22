use longhorn_core::{ScaleFactor, WindowId};
use longhorn_windowing::WindowLifecycleEvent;

use crate::{GpuiLogicalRect, GpuiLogicalSize, GpuiWindowLifecycleError, scale_factor_from_gpui};

/// One native GPUI window callback, as GPUI delivers it.
///
/// The shape is GPUI's, not Longhorn's. `on_moved` carries no payload, so the
/// adapter's caller reads `Window::bounds` and passes it here. `on_resize`
/// carries the scale factor alongside the size, because GPUI has no separate
/// scale-change callback — the only way to learn that a window crossed onto a
/// display with a different scale is to notice it inside a resize.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuiWindowEvent {
    /// The window's outer bounds changed.
    Moved {
        /// Bounds read from `Window::bounds` after the callback fired.
        bounds: GpuiLogicalRect,
    },
    /// The window's content size or scale changed.
    Resized {
        /// New content size.
        content_size: GpuiLogicalSize,
        /// Scale reported with the resize.
        scale: f32,
    },
    /// The operating system changed the window's active state.
    ActiveStatusChanged {
        /// Whether the window is now active.
        active: bool,
    },
    /// The user asked to close the window and GPUI wants a decision now.
    CloseRequested,
    /// The window has been removed.
    Closed,
}

/// Translates one GPUI callback into Longhorn's vocabulary.
///
/// Returns a sequence, not a single event: a GPUI resize carries a scale, so
/// one callback becomes both `Resized` and `ScaleChanged` when the scale moved.
/// The Tauri translation is one-to-one because Tauri has a dedicated
/// `ScaleFactorChanged` event.
///
/// `previous_scale` is what the caller last recorded for this window. Passing
/// `None` suppresses the `ScaleChanged`, which is correct for the first
/// observation of a window.
pub fn translate_gpui_window_event(
    window_id: &WindowId,
    event: GpuiWindowEvent,
    previous_scale: Option<ScaleFactor>,
) -> Result<Vec<WindowLifecycleEvent>, GpuiWindowLifecycleError> {
    let translated = match event {
        GpuiWindowEvent::Moved { bounds } => vec![WindowLifecycleEvent::Moved {
            window_id: window_id.clone(),
            outer_origin: bounds.to_screen_origin().map_err(translation_error)?,
        }],
        GpuiWindowEvent::Resized {
            content_size,
            scale,
        } => {
            let scale = scale_factor_from_gpui(scale).map_err(translation_error)?;
            let mut events = vec![WindowLifecycleEvent::Resized {
                window_id: window_id.clone(),
                inner_size: content_size.to_screen_size().map_err(translation_error)?,
            }];
            if previous_scale.is_some_and(|previous| previous != scale) {
                events.push(WindowLifecycleEvent::ScaleChanged {
                    window_id: window_id.clone(),
                    scale,
                });
            }
            events
        }
        // GPUI reports focus gain and loss through one callback. Longhorn's
        // vocabulary has only `Blurred`, so focus gain translates to nothing —
        // exactly as it does on Tauri, where `Focused(true)` is dropped.
        GpuiWindowEvent::ActiveStatusChanged { active: false } => {
            vec![WindowLifecycleEvent::Blurred {
                window_id: window_id.clone(),
            }]
        }
        GpuiWindowEvent::ActiveStatusChanged { active: true } => Vec::new(),
        GpuiWindowEvent::CloseRequested => vec![WindowLifecycleEvent::CloseRequested {
            window_id: window_id.clone(),
        }],
        GpuiWindowEvent::Closed => vec![WindowLifecycleEvent::Destroyed {
            window_id: window_id.clone(),
        }],
    };
    Ok(translated)
}

pub(super) fn translation_error(detail: impl ToString) -> GpuiWindowLifecycleError {
    GpuiWindowLifecycleError::EventTranslation {
        detail: detail.to_string(),
    }
}
