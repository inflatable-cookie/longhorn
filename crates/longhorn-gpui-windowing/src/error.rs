use std::{error::Error, fmt};

use longhorn_core::WindowId;
use longhorn_windowing::{HostWindowHandle, WindowDiffError};

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
    /// Per-display scale.
    ///
    /// GPUI reports scale only from a live window, but the platform reports
    /// it per display and GPUI's `DisplayId` is the platform's id — so this
    /// means "no reader supplied", not "unknowable".
    ScaleFactor,
    /// The area left after system chrome. GPUI reports full bounds only.
    WorkArea,
    /// Where the display sits in the global desktop plane.
    ///
    /// GPUI's macOS backend reads `CGDisplayBounds`, whose own comment says it
    /// is "in global display coordinates", and then discards the origin —
    /// every display is reported at `(0, 0)`. Size survives; position does
    /// not. Two attached displays therefore claim the same origin, which is
    /// not a coordinate system Longhorn can place windows in.
    ///
    /// Dropped rather than lost: the same call on the same id returns the
    /// real value. This variant means "no reader supplied".
    Position,
}

impl fmt::Display for UnobtainableDisplayFact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ScaleFactor => "per-display scale factor",
            Self::WorkArea => "display work area",
            Self::Position => "display position in the global plane",
        })
    }
}

/// Display observation failure at the GPUI edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiDisplayError {
    /// A display reported a negative or zero extent.
    InvalidBounds {
        /// Ordinal of the failing display in the probe.
        ordinal: usize,
        /// Boundary diagnostic.
        detail: String,
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
            Self::InvalidBounds { ordinal, detail } => {
                write!(
                    formatter,
                    "display {ordinal} reported invalid bounds: {detail}"
                )
            }
            Self::Host { detail } => write!(formatter, "gpui display probe failed: {detail}"),
        }
    }
}

impl Error for GpuiDisplayError {}

/// Window observation failure at the GPUI edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiObservationError {
    /// A rejected native window is still tracked for cleanup.
    PendingWindowDiscard(WindowId),
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
    /// The display probe failed independently of any window.
    Display(GpuiDisplayError),
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
            Self::PendingWindowDiscard(window_id) => {
                write!(
                    formatter,
                    "rejected window {window_id} still needs native cleanup"
                )
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
            Self::Display(source) => write!(formatter, "{source}"),
            Self::Host { handle, detail } => {
                write!(formatter, "gpui probe of window {handle} failed: {detail}")
            }
        }
    }
}

impl Error for GpuiObservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Display(source) => Some(source),
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
    /// A window with this identity is already installed.
    DuplicateWindow {
        /// Repeated identity.
        window_id: WindowId,
    },
    /// A native fact could not be expressed in Longhorn's vocabulary.
    EventTranslation {
        /// Boundary diagnostic.
        detail: String,
    },
    /// The pure coordinator rejected the input.
    Coordination {
        /// Pure coordinator failure.
        source: longhorn_windowing::WindowLifecycleError,
    },
}

impl fmt::Display for GpuiWindowLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownWindow { window_id } => {
                write!(formatter, "window {window_id} is not installed")
            }
            Self::DuplicateWindow { window_id } => {
                write!(formatter, "window {window_id} is already installed")
            }
            Self::EventTranslation { detail } => {
                write!(formatter, "gpui event translation failed: {detail}")
            }
            Self::Coordination { source } => {
                write!(formatter, "window lifecycle coordination failed: {source}")
            }
        }
    }
}

impl Error for GpuiWindowLifecycleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Coordination { source } => Some(source),
            _ => None,
        }
    }
}

/// Failure reported by a GPUI lifecycle service or apply observer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiServiceError {
    /// The injected service refused the request.
    Host(String),
    /// The coordinator could not compute a deadline or generation.
    Coordination(longhorn_windowing::WindowLifecycleError),
    /// The apply generation is older than registered evidence.
    StaleGeneration {
        /// Rejected generation.
        attempted: longhorn_windowing::ApplyGeneration,
        /// Latest registered generation.
        current: longhorn_windowing::ApplyGeneration,
    },
    /// The apply clock preceded registered evidence.
    StaleTimestamp {
        /// Latest registered timestamp.
        latest: longhorn_windowing::MonotonicMillis,
    },
}

impl GpuiServiceError {
    /// Records a service diagnostic.
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self::Host(detail.into())
    }
}
impl fmt::Display for GpuiServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(detail) => formatter.write_str(detail),
            Self::Coordination(source) => write!(formatter, "{source}"),
            Self::StaleGeneration { attempted, current } => write!(
                formatter,
                "apply generation {} is older than {}",
                attempted.get(),
                current.get()
            ),
            Self::StaleTimestamp { latest } => write!(
                formatter,
                "apply evidence timestamp precedes {}",
                latest.get()
            ),
        }
    }
}
impl Error for GpuiServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Coordination(source) => Some(source),
            _ => None,
        }
    }
}
