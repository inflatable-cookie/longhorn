use std::{error::Error, fmt};

use longhorn_windowing::WindowLifecycleDuration;

/// Successful ordered Surface flush and window-host shutdown evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowShutdownReceipt<SurfaceReceipt, WindowReceipt> {
    surface: SurfaceReceipt,
    window: WindowReceipt,
}

impl<SurfaceReceipt, WindowReceipt> SurfaceWindowShutdownReceipt<SurfaceReceipt, WindowReceipt> {
    /// Returns completed Surface persistence flush evidence.
    #[must_use]
    pub const fn surface(&self) -> &SurfaceReceipt {
        &self.surface
    }

    /// Returns subsequent window-host shutdown evidence.
    #[must_use]
    pub const fn window(&self) -> &WindowReceipt {
        &self.window
    }
}

/// Failed ordered shutdown with completed prior-stage evidence retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceWindowShutdownError<SurfaceError, WindowError, SurfaceReceipt> {
    /// Surface persistence did not flush; window shutdown was not attempted.
    Surface(SurfaceError),
    /// Surface persistence flushed before window shutdown failed.
    Window {
        /// Completed Surface flush evidence.
        surface: SurfaceReceipt,
        /// Window-host shutdown failure.
        source: WindowError,
    },
}

impl<SurfaceError, WindowError, SurfaceReceipt> fmt::Display
    for SurfaceWindowShutdownError<SurfaceError, WindowError, SurfaceReceipt>
where
    SurfaceError: fmt::Display,
    WindowError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Surface(error) => write!(formatter, "Surface flush failed: {error}"),
            Self::Window { source, .. } => write!(formatter, "window shutdown failed: {source}"),
        }
    }
}

impl<SurfaceError, WindowError, SurfaceReceipt> Error
    for SurfaceWindowShutdownError<SurfaceError, WindowError, SurfaceReceipt>
where
    SurfaceError: Error,
    WindowError: Error,
    SurfaceReceipt: fmt::Debug,
{
}

/// Flushes Surface persistence before invoking the existing window-host shutdown.
///
/// Both operations stay injected. The Surface timeout is explicit and a
/// failed Surface flush prevents native teardown so the caller may retry.
pub fn shutdown_surface_window_host<
    Flush,
    Shutdown,
    SurfaceReceipt,
    WindowReceipt,
    SurfaceError,
    WindowError,
>(
    surface_timeout: WindowLifecycleDuration,
    flush_surfaces: Flush,
    shutdown_windows: Shutdown,
) -> Result<
    SurfaceWindowShutdownReceipt<SurfaceReceipt, WindowReceipt>,
    SurfaceWindowShutdownError<SurfaceError, WindowError, SurfaceReceipt>,
>
where
    Flush: FnOnce(WindowLifecycleDuration) -> Result<SurfaceReceipt, SurfaceError>,
    Shutdown: FnOnce() -> Result<WindowReceipt, WindowError>,
{
    let surface = flush_surfaces(surface_timeout).map_err(SurfaceWindowShutdownError::Surface)?;
    let window = match shutdown_windows() {
        Ok(receipt) => receipt,
        Err(source) => {
            return Err(SurfaceWindowShutdownError::Window { surface, source });
        }
    };
    Ok(SurfaceWindowShutdownReceipt { surface, window })
}
