use longhorn_core::WindowId;
use longhorn_windowing::WindowLifecycleEvent;
use tauri::{Runtime, WebviewWindow, WindowEvent};

use super::{TauriWindowLifecycleError, capture::map_lifecycle_window};
use crate::{DesktopCoordinateMapper, DesktopMappingError, scale_factor_from_tauri};

/// Translates one Tauri event without persistence or product policy.
pub fn translate_tauri_window_event<R: Runtime>(
    window_id: &WindowId,
    window: &WebviewWindow<R>,
    event: &WindowEvent,
    mapper: &(impl DesktopCoordinateMapper + ?Sized),
) -> Result<Option<WindowLifecycleEvent>, TauriWindowLifecycleError> {
    let translated = match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            let (snapshot, mapped) = map_lifecycle_window(window_id, window, Some(event), mapper)
                .map_err(|error| translation_error(error.to_string()))?;
            let geometry = snapshot
                .windows()
                .first()
                .and_then(|raw| {
                    mapped
                        .windows()
                        .iter()
                        .find(|geometry| geometry.transport_handle() == raw.transport_handle())
                })
                .ok_or_else(|| {
                    translation_error(DesktopMappingError::MissingLifecycleGeometry.to_string())
                })?;
            Some(if matches!(event, WindowEvent::Moved(_)) {
                WindowLifecycleEvent::Moved {
                    window_id: window_id.clone(),
                    outer_origin: geometry.outer_bounds().origin(),
                }
            } else {
                WindowLifecycleEvent::Resized {
                    window_id: window_id.clone(),
                    inner_size: geometry.inner_size(),
                }
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

fn translation_error(detail: impl Into<String>) -> TauriWindowLifecycleError {
    TauriWindowLifecycleError::EventTranslation {
        detail: detail.into(),
    }
}
