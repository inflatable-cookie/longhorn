use std::{error::Error, fmt};

use longhorn_core::WindowId;
use longhorn_windowing::{HostWindowHandle, HostWindowHandleError, WindowDiffError};

use crate::GpuiWindowRegistryError;

/// GPUI scale-factor validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuiScaleFactorError {
    /// The reported scale was NaN or infinite.
    NonFinite,
    /// The reported scale was zero or negative.
    NonPositive,
    /// The reported scale rounded to zero thousandths.
    RoundedToZero,
    /// The reported scale exceeded the fixed-point domain.
    Overflow,
}

impl fmt::Display for GpuiScaleFactorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "gpui scale factor was not finite",
            Self::NonPositive => "gpui scale factor was not positive",
            Self::RoundedToZero => "gpui scale factor rounded to zero thousandths",
            Self::Overflow => "gpui scale factor exceeded the fixed-point domain",
        })
    }
}

impl Error for GpuiScaleFactorError {}

/// Which unobtainable display fact a caller asked a GPUI host for.
///
/// GPUI's display API reports logical bounds and a stable identity, and
/// nothing else. Contract 020 asks a host for displays "with scale factors",
/// which GPUI answers only per window. These variants exist so the gap is a
/// typed refusal rather than a fabricated number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnobtainableDisplayFact {
    /// Per-display scale. GPUI reports scale only from a live window.
    ScaleFactor,
    /// The area left after system chrome. GPUI reports full bounds only.
    WorkArea,
    /// Physical pixel bounds. GPUI reports logical bounds only.
    PhysicalBounds,
    /// Where the display sits in the global desktop plane.
    ///
    /// GPUI's macOS backend reads `CGDisplayBounds`, whose own comment says it
    /// is "in global display coordinates", and then discards the origin —
    /// every display is reported at `(0, 0)`. Size survives; position does
    /// not. Two attached displays therefore claim the same origin, which is
    /// not a coordinate system Longhorn can place windows in.
    Position,
    /// Whether the display is the machine's built-in panel.
    BuiltinStatus,
}

impl fmt::Display for UnobtainableDisplayFact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ScaleFactor => "per-display scale factor",
            Self::WorkArea => "display work area",
            Self::PhysicalBounds => "physical display bounds",
            Self::BuiltinStatus => "built-in display status",
            Self::Position => "display position in the global plane",
        })
    }
}

/// Display observation failure at the GPUI edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiDisplayError {
    /// A display reported an unusable scale.
    InvalidScale {
        /// Ordinal of the failing display in the probe.
        ordinal: usize,
        /// Validation failure.
        source: GpuiScaleFactorError,
    },
    /// A display reported a negative or zero extent.
    InvalidBounds {
        /// Ordinal of the failing display in the probe.
        ordinal: usize,
        /// Boundary diagnostic.
        detail: String,
    },
    /// Two displays in one probe claimed the same observation id.
    DuplicateObservationId {
        /// The repeated id.
        observation_id: String,
    },
    /// The host could not supply a fact contract 020 requires.
    Unobtainable {
        /// Ordinal of the failing display in the probe.
        ordinal: usize,
        /// The missing fact.
        fact: UnobtainableDisplayFact,
    },
    /// A GPUI call failed.
    Host {
        /// Boundary diagnostic.
        detail: String,
    },
}

impl fmt::Display for GpuiDisplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScale { ordinal, source } => {
                write!(
                    formatter,
                    "display {ordinal} reported an invalid scale: {source}"
                )
            }
            Self::InvalidBounds { ordinal, detail } => {
                write!(
                    formatter,
                    "display {ordinal} reported invalid bounds: {detail}"
                )
            }
            Self::DuplicateObservationId { observation_id } => {
                write!(
                    formatter,
                    "duplicate display observation id {observation_id}"
                )
            }
            Self::Unobtainable { ordinal, fact } => write!(
                formatter,
                "gpui cannot report the {fact} of display {ordinal}"
            ),
            Self::Host { detail } => write!(formatter, "gpui display probe failed: {detail}"),
        }
    }
}

impl Error for GpuiDisplayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidScale { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Window observation failure at the GPUI edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiObservationError {
    /// A GPUI window key violated opaque handle syntax.
    InvalidTransportHandle {
        /// The rejected key.
        key: String,
        /// Validation failure.
        source: HostWindowHandleError,
    },
    /// Two observed windows shared a handle.
    DuplicateTransportHandle(HostWindowHandle),
    /// Two observed windows shared a stable id.
    DuplicateWindowId(WindowId),
    /// A window reported an unusable scale.
    InvalidScale {
        /// The observed window.
        handle: HostWindowHandle,
        /// Validation failure.
        source: GpuiScaleFactorError,
    },
    /// Logical-to-screen conversion left the representable domain.
    Geometry {
        /// The observed window.
        handle: HostWindowHandle,
        /// Boundary diagnostic.
        detail: String,
    },
    /// A GPUI call failed.
    Host {
        /// The observed window.
        handle: HostWindowHandle,
        /// Boundary diagnostic.
        detail: String,
    },
}

impl fmt::Display for GpuiObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransportHandle { key, .. } => {
                write!(formatter, "invalid gpui window key {key:?}")
            }
            Self::DuplicateTransportHandle(handle) => {
                write!(formatter, "duplicate observed handle {handle}")
            }
            Self::DuplicateWindowId(window_id) => {
                write!(formatter, "duplicate observed window id {window_id}")
            }
            Self::InvalidScale { handle, source } => {
                write!(
                    formatter,
                    "window {handle} reported an invalid scale: {source}"
                )
            }
            Self::Geometry { handle, detail } => {
                write!(
                    formatter,
                    "window {handle} geometry conversion failed: {detail}"
                )
            }
            Self::Host { handle, detail } => {
                write!(formatter, "gpui probe of window {handle} failed: {detail}")
            }
        }
    }
}

impl Error for GpuiObservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTransportHandle { source, .. } => Some(source),
            Self::InvalidScale { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Failure before ordered operation execution can begin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiApplyError {
    /// Pure diff planning rejected input identity.
    Planning(WindowDiffError),
    /// Registry rejected generation or initial execution state.
    Registry(GpuiWindowRegistryError),
}

impl fmt::Display for GpuiApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planning(source) => write!(formatter, "window apply planning failed: {source}"),
            Self::Registry(source) => write!(formatter, "window registry failed: {source}"),
        }
    }
}

impl Error for GpuiApplyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Planning(source) => Some(source),
            Self::Registry(source) => Some(source),
        }
    }
}

/// Lifecycle translation or coordination failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiWindowLifecycleError {
    /// The host received an event for a window it does not manage.
    UnknownWindow {
        /// The unmanaged identity.
        window_id: WindowId,
    },
    /// A native fact could not be expressed in Longhorn's vocabulary.
    EventTranslation {
        /// Boundary diagnostic.
        detail: String,
    },
    /// The pure coordinator rejected the input.
    Coordination {
        /// Boundary diagnostic.
        detail: String,
    },
}

impl fmt::Display for GpuiWindowLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownWindow { window_id } => {
                write!(formatter, "window {window_id} is not installed")
            }
            Self::EventTranslation { detail } => {
                write!(formatter, "gpui event translation failed: {detail}")
            }
            Self::Coordination { detail } => {
                write!(formatter, "window lifecycle coordination failed: {detail}")
            }
        }
    }
}

impl Error for GpuiWindowLifecycleError {}
