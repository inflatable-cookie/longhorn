use std::{error::Error, fmt};

use longhorn_core::WindowId;
use longhorn_display::ObservationId;
use longhorn_windowing::{HostWindowHandle, HostWindowHandleError};

/// Tauri scale-factor validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TauriScaleFactorError {
    /// The host value was NaN or infinite.
    NonFinite,
    /// The host value was zero or negative.
    NonPositive,
    /// The value rounded below one thousandth.
    RoundedToZero,
    /// The value in thousandths exceeded `u32`.
    Overflow,
}

impl fmt::Display for TauriScaleFactorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("Tauri scale factor must be finite"),
            Self::NonPositive => formatter.write_str("Tauri scale factor must be positive"),
            Self::RoundedToZero => {
                formatter.write_str("Tauri scale factor rounded to zero thousandths")
            }
            Self::Overflow => formatter.write_str("Tauri scale factor exceeded fixed-point range"),
        }
    }
}

impl Error for TauriScaleFactorError {}

/// Native getter that failed during a complete Tauri probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HostProbeOperation {
    /// Enumerate available monitors.
    AvailableMonitors,
    /// Read the primary monitor.
    PrimaryMonitor,
    /// Read window scale.
    WindowScale,
    /// Read outer-frame position.
    OuterPosition,
    /// Read outer-frame size.
    OuterSize,
    /// Read inner content size.
    InnerSize,
    /// Read maximized state.
    Maximized,
    /// Read visibility state.
    Visible,
    /// Read focus state.
    Focused,
}

impl fmt::Display for HostProbeOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::AvailableMonitors => "available_monitors",
            Self::PrimaryMonitor => "primary_monitor",
            Self::WindowScale => "window_scale",
            Self::OuterPosition => "outer_position",
            Self::OuterSize => "outer_size",
            Self::InnerSize => "inner_size",
            Self::Maximized => "is_maximized",
            Self::Visible => "is_visible",
            Self::Focused => "is_focused",
        };
        formatter.write_str(name)
    }
}

/// Subject whose scale factor failed validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbeTarget {
    /// Available monitor at the named probe ordinal.
    Display(usize),
    /// Separately returned primary-monitor descriptor.
    PrimaryDisplay,
    /// Explicitly managed window.
    Window(HostWindowHandle),
}

impl fmt::Display for ProbeTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Display(ordinal) => write!(formatter, "display {ordinal}"),
            Self::PrimaryDisplay => formatter.write_str("primary display"),
            Self::Window(handle) => write!(formatter, "window {handle}"),
        }
    }
}

/// Complete physical Tauri probe failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TauriProbeError {
    /// A Tauri getter failed.
    Host {
        /// Getter that failed.
        operation: HostProbeOperation,
        /// Managed transport handle, when the operation was window-scoped.
        handle: Option<HostWindowHandle>,
        /// Tauri's diagnostic text.
        detail: String,
    },
    /// A Tauri scale factor was invalid.
    InvalidScale {
        /// Display or window being converted.
        target: ProbeTarget,
        /// Scale validation failure.
        source: TauriScaleFactorError,
    },
    /// A Tauri label could not be represented as a transport handle.
    InvalidTransportHandle {
        /// Original Tauri label.
        label: String,
        /// Handle validation failure.
        source: HostWindowHandleError,
    },
    /// The primary descriptor did not match an available monitor.
    PrimaryMonitorNotFound,
    /// The primary descriptor matched more than one available monitor.
    AmbiguousPrimaryMonitor {
        /// Exact match count.
        matches: usize,
    },
    /// A metadata provider reused an observation id.
    DuplicateObservationId(ObservationId),
    /// The managed input repeated a transport handle.
    DuplicateTransportHandle(HostWindowHandle),
    /// The managed input assigned one stable id to multiple windows.
    DuplicateWindowId(WindowId),
}

impl fmt::Display for TauriProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host {
                operation,
                handle,
                detail,
            } => {
                if let Some(handle) = handle {
                    write!(formatter, "{operation} failed for {handle}: {detail}")
                } else {
                    write!(formatter, "{operation} failed: {detail}")
                }
            }
            Self::InvalidScale { target, source } => {
                write!(formatter, "invalid scale for {target}: {source}")
            }
            Self::InvalidTransportHandle { label, source } => {
                write!(formatter, "invalid Tauri window label {label:?}: {source}")
            }
            Self::PrimaryMonitorNotFound => {
                formatter.write_str("primary monitor did not match an available monitor")
            }
            Self::AmbiguousPrimaryMonitor { matches } => {
                write!(
                    formatter,
                    "primary monitor matched {matches} available monitors"
                )
            }
            Self::DuplicateObservationId(id) => {
                write!(formatter, "duplicate display observation id {id}")
            }
            Self::DuplicateTransportHandle(handle) => {
                write!(formatter, "duplicate managed transport handle {handle}")
            }
            Self::DuplicateWindowId(id) => write!(formatter, "duplicate managed window id {id}"),
        }
    }
}

impl Error for TauriProbeError {}
