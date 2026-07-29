use std::{error::Error, fmt};

use longhorn_core::{ScaleConversionError, ScaleFactor, WindowId};
use longhorn_display::{DisplayTextError, ObservationId};
use longhorn_windowing::HostWindowHandle;

mod probe;

pub use probe::{HostProbeOperation, ProbeTarget, TauriProbeError, TauriScaleFactorError};

/// Whole-desktop coordinate-mapping failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopMappingError {
    /// More than one scale exists and the uniform mapper cannot establish a plane.
    MixedScaleUnavailable {
        /// Distinct validated scales in ascending order.
        scales: Vec<ScaleFactor>,
    },
    /// Checked physical-to-screen conversion overflowed.
    Conversion(ScaleConversionError),
    /// An injected mapper rejected the snapshot.
    Provider {
        /// Provider diagnostic.
        detail: String,
    },
}

impl fmt::Display for DesktopMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MixedScaleUnavailable { scales } => write!(
                formatter,
                "uniform desktop mapping unavailable for {} distinct scales",
                scales.len()
            ),
            Self::Conversion(source) => write!(formatter, "desktop conversion failed: {source}"),
            Self::Provider { detail } => write!(formatter, "desktop mapper failed: {detail}"),
        }
    }
}

impl Error for DesktopMappingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Conversion(source) => Some(source),
            _ => None,
        }
    }
}

impl From<ScaleConversionError> for DesktopMappingError {
    fn from(value: ScaleConversionError) -> Self {
        Self::Conversion(value)
    }
}

/// Projection failure between raw and mapped complete snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopObservationError {
    /// Coordinate mapping failed.
    Mapping(DesktopMappingError),
    /// Raw display ids were not unique.
    DuplicateDisplayObservation(ObservationId),
    /// Raw window handles were not unique.
    DuplicateWindowObservation(HostWindowHandle),
    /// Raw stable window ids were not unique.
    DuplicateStableWindowId(WindowId),
    /// A mapped display id was repeated.
    DuplicateMappedDisplay(ObservationId),
    /// A mapped window handle was repeated.
    DuplicateMappedWindow(HostWindowHandle),
    /// A raw display had no mapped geometry.
    MissingDisplayMapping(ObservationId),
    /// A raw window had no mapped geometry.
    MissingWindowMapping(HostWindowHandle),
    /// A mapper returned geometry for an unknown display.
    UnexpectedDisplayMapping(ObservationId),
    /// A mapper returned geometry for an unmanaged window.
    UnexpectedWindowMapping(HostWindowHandle),
    /// A machine monitor label violated the display text contract.
    InvalidMachineLabel {
        /// Observation carrying the label.
        observation_id: ObservationId,
        /// Label validation failure.
        source: DisplayTextError,
    },
}

impl fmt::Display for DesktopObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mapping(source) => write!(formatter, "coordinate mapping failed: {source}"),
            Self::DuplicateDisplayObservation(id) => {
                write!(formatter, "duplicate raw display observation {id}")
            }
            Self::DuplicateWindowObservation(handle) => {
                write!(formatter, "duplicate raw window observation {handle}")
            }
            Self::DuplicateStableWindowId(id) => {
                write!(formatter, "duplicate raw stable window id {id}")
            }
            Self::DuplicateMappedDisplay(id) => {
                write!(formatter, "duplicate mapped display {id}")
            }
            Self::DuplicateMappedWindow(handle) => {
                write!(formatter, "duplicate mapped window {handle}")
            }
            Self::MissingDisplayMapping(id) => write!(formatter, "missing display mapping {id}"),
            Self::MissingWindowMapping(handle) => {
                write!(formatter, "missing window mapping {handle}")
            }
            Self::UnexpectedDisplayMapping(id) => {
                write!(formatter, "unexpected display mapping {id}")
            }
            Self::UnexpectedWindowMapping(handle) => {
                write!(formatter, "unexpected window mapping {handle}")
            }
            Self::InvalidMachineLabel {
                observation_id,
                source,
            } => write!(
                formatter,
                "invalid machine label for observation {observation_id}: {source}"
            ),
        }
    }
}

impl Error for DesktopObservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Mapping(source) => Some(source),
            Self::InvalidMachineLabel { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<DesktopMappingError> for DesktopObservationError {
    fn from(value: DesktopMappingError) -> Self {
        Self::Mapping(value)
    }
}

/// End-to-end Tauri observation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TauriObservationError {
    /// Physical host probing failed.
    Probe(TauriProbeError),
    /// Whole-desktop mapping or projection failed.
    Projection(DesktopObservationError),
}

impl fmt::Display for TauriObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Probe(source) => write!(formatter, "Tauri probe failed: {source}"),
            Self::Projection(source) => write!(formatter, "desktop projection failed: {source}"),
        }
    }
}

impl Error for TauriObservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Probe(source) => Some(source),
            Self::Projection(source) => Some(source),
        }
    }
}

impl From<TauriProbeError> for TauriObservationError {
    fn from(value: TauriProbeError) -> Self {
        Self::Probe(value)
    }
}

impl From<DesktopObservationError> for TauriObservationError {
    fn from(value: DesktopObservationError) -> Self {
        Self::Projection(value)
    }
}
