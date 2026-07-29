use std::{error::Error, fmt};

use longhorn_core::{ScreenPoint, ScreenSize};
use tauri::{LogicalPosition, LogicalSize, Runtime, WebviewWindow};

/// Native mutation failure normalized at the Tauri boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeWindowMutationError {
    detail: String,
}

impl NativeWindowMutationError {
    /// Constructs a native mutation diagnostic.
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

impl fmt::Display for NativeWindowMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for NativeWindowMutationError {}

/// Injectable boundary for native window calls.
pub trait WindowMutationBackend<R: Runtime>: Send {
    /// Leaves maximized state.
    fn unmaximize(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError>;

    /// Applies the outer-frame origin.
    fn set_outer_position(
        &mut self,
        window: &WebviewWindow<R>,
        origin: ScreenPoint,
    ) -> Result<(), NativeWindowMutationError>;

    /// Applies the inner-content size.
    fn set_inner_size(
        &mut self,
        window: &WebviewWindow<R>,
        size: ScreenSize,
    ) -> Result<(), NativeWindowMutationError>;

    /// Enters maximized state.
    fn maximize(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError>;

    /// Shows the window.
    fn show(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError>;

    /// Hides the window.
    fn hide(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError>;

    /// Focuses the window.
    fn focus(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError>;

    /// Requests window close.
    fn close(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError>;
}

/// Direct Tauri 2 mutation backend.
#[derive(Clone, Copy, Debug, Default)]
pub struct TauriWindowMutationBackend;

impl<R: Runtime> WindowMutationBackend<R> for TauriWindowMutationBackend {
    fn unmaximize(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        host(window.unmaximize())
    }

    fn set_outer_position(
        &mut self,
        window: &WebviewWindow<R>,
        origin: ScreenPoint,
    ) -> Result<(), NativeWindowMutationError> {
        host(window.set_position(LogicalPosition::new(
            f64::from(origin.x().get()),
            f64::from(origin.y().get()),
        )))
    }

    fn set_inner_size(
        &mut self,
        window: &WebviewWindow<R>,
        size: ScreenSize,
    ) -> Result<(), NativeWindowMutationError> {
        host(window.set_size(LogicalSize::new(
            f64::from(size.width()),
            f64::from(size.height()),
        )))
    }

    fn maximize(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        host(window.maximize())
    }

    fn show(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        host(window.show())
    }

    fn hide(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        host(window.hide())
    }

    fn focus(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        host(window.set_focus())
    }

    fn close(&mut self, window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        host(window.close())
    }
}

fn host(result: tauri::Result<()>) -> Result<(), NativeWindowMutationError> {
    result.map_err(|source| NativeWindowMutationError::new(source.to_string()))
}
