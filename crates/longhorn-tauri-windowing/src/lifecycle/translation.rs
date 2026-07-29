use longhorn_core::{PhysicalPoint, PhysicalSize, WindowId};
use longhorn_windowing::WindowLifecycleEvent;
use tauri::{Runtime, WebviewWindow, WindowEvent};

use super::{TauriWindowLifecycleError, WindowGeometryMapper};
use crate::scale_factor_from_tauri;

/// Translates one Tauri event without persistence or product policy.
pub fn translate_tauri_window_event<R: Runtime>(
    window_id: &WindowId,
    window: &WebviewWindow<R>,
    event: &WindowEvent,
    mapper: &dyn WindowGeometryMapper,
) -> Result<Option<WindowLifecycleEvent>, TauriWindowLifecycleError> {
    let translated = match event {
        WindowEvent::Moved(position) => {
            let scale = window_scale(window)?;
            Some(WindowLifecycleEvent::Moved {
                window_id: window_id.clone(),
                outer_origin: mapper
                    .map_outer_origin(PhysicalPoint::new(position.x, position.y), scale)
                    .map_err(translation_error)?,
            })
        }
        WindowEvent::Resized(size) => {
            let scale = window_scale(window)?;
            Some(WindowLifecycleEvent::Resized {
                window_id: window_id.clone(),
                inner_size: mapper
                    .map_inner_size(PhysicalSize::new(size.width, size.height), scale)
                    .map_err(translation_error)?,
            })
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            Some(WindowLifecycleEvent::ScaleChanged {
                window_id: window_id.clone(),
                scale: event_scale(*scale_factor)?,
            })
        }
        WindowEvent::Focused(false) => Some(WindowLifecycleEvent::Blurred {
            window_id: window_id.clone(),
        }),
        WindowEvent::CloseRequested { .. } => Some(WindowLifecycleEvent::CloseRequested {
            window_id: window_id.clone(),
        }),
        WindowEvent::Destroyed => Some(WindowLifecycleEvent::Destroyed {
            window_id: window_id.clone(),
        }),
        WindowEvent::Focused(true) | WindowEvent::DragDrop(_) | WindowEvent::ThemeChanged(_) => {
            None
        }
        _ => None,
    };
    Ok(translated)
}

fn event_scale(value: f64) -> Result<longhorn_core::ScaleFactor, TauriWindowLifecycleError> {
    scale_factor_from_tauri(value).map_err(|error| translation_error(error.to_string()))
}

fn window_scale<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<longhorn_core::ScaleFactor, TauriWindowLifecycleError> {
    let value = window
        .scale_factor()
        .map_err(|error| translation_error(format!("window scale probe failed: {error}")))?;
    event_scale(value)
}

fn translation_error(detail: impl Into<String>) -> TauriWindowLifecycleError {
    TauriWindowLifecycleError::EventTranslation {
        detail: detail.into(),
    }
}
