use std::{error::Error, fmt};

use longhorn_core::WindowId;
use tauri::{AppHandle, Runtime, WebviewWindow};

/// Consumer-owned construction of one neutral hidden unmaximized webview window.
pub trait TauriWindowFactory<R: Runtime>: Send {
    /// Returns whether dynamic window creation is available.
    fn can_create(&self) -> bool;

    /// Creates one host slot using consumer-owned URL, chrome, title, and product metadata.
    fn create(
        &mut self,
        app: &AppHandle<R>,
        window_id: &WindowId,
    ) -> Result<WebviewWindow<R>, WindowFactoryError>;

    /// Verifies the returned slot is hidden and unmaximized.
    fn validate_neutral(&mut self, window: &WebviewWindow<R>) -> Result<(), WindowFactoryError> {
        match window.is_visible() {
            Ok(false) => {}
            Ok(true) => return Err(WindowFactoryError::Visible),
            Err(error) => {
                return Err(WindowFactoryError::VisibilityInspectionFailed {
                    detail: error.to_string(),
                });
            }
        }
        match window.is_maximized() {
            Ok(false) => Ok(()),
            Ok(true) => Err(WindowFactoryError::Maximized),
            Err(error) => Err(WindowFactoryError::MaximizedInspectionFailed {
                detail: error.to_string(),
            }),
        }
    }
}

impl<R, F> TauriWindowFactory<R> for F
where
    R: Runtime,
    F: FnMut(&AppHandle<R>, &WindowId) -> Result<WebviewWindow<R>, WindowFactoryError> + Send,
{
    fn can_create(&self) -> bool {
        true
    }

    fn create(
        &mut self,
        app: &AppHandle<R>,
        window_id: &WindowId,
    ) -> Result<WebviewWindow<R>, WindowFactoryError> {
        self(app, window_id)
    }
}

/// Explicit factory for hosts that cannot create dynamic windows.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoWindowFactory;

impl<R: Runtime> TauriWindowFactory<R> for NoWindowFactory {
    fn can_create(&self) -> bool {
        false
    }

    fn create(
        &mut self,
        _app: &AppHandle<R>,
        _window_id: &WindowId,
    ) -> Result<WebviewWindow<R>, WindowFactoryError> {
        Err(WindowFactoryError::Unavailable)
    }
}

/// Consumer factory failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowFactoryError {
    /// This host has no dynamic-window factory.
    Unavailable,
    /// Consumer construction failed.
    Failed {
        /// Consumer diagnostic.
        detail: String,
    },
    /// Factory returned a visible slot.
    Visible,
    /// Factory returned a maximized slot.
    Maximized,
    /// Native visibility inspection failed.
    VisibilityInspectionFailed {
        /// Tauri diagnostic.
        detail: String,
    },
    /// Native maximized-state inspection failed.
    MaximizedInspectionFailed {
        /// Tauri diagnostic.
        detail: String,
    },
}

impl fmt::Display for WindowFactoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("dynamic window creation is unavailable"),
            Self::Failed { detail } => write!(formatter, "window factory failed: {detail}"),
            Self::Visible => formatter.write_str("window factory returned a visible slot"),
            Self::Maximized => formatter.write_str("window factory returned a maximized slot"),
            Self::VisibilityInspectionFailed { detail } => {
                write!(formatter, "window visibility inspection failed: {detail}")
            }
            Self::MaximizedInspectionFailed { detail } => {
                write!(formatter, "window maximized inspection failed: {detail}")
            }
        }
    }
}

impl Error for WindowFactoryError {}
