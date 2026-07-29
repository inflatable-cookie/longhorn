use std::{error::Error, fmt};

use longhorn_core::WindowId;

use super::HostWindowHandle;

/// Invalid desired/live snapshot shape that cannot be resolved by ordering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowDiffError {
    /// Desired input repeated stable logical identity.
    DuplicateDesiredWindowId(WindowId),
    /// Live input repeated stable logical identity.
    DuplicateLiveWindowId(WindowId),
    /// Live input repeated a transport handle.
    DuplicateTransportHandle(HostWindowHandle),
}

impl fmt::Display for WindowDiffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDesiredWindowId(id) => {
                write!(formatter, "duplicate desired window id {id}")
            }
            Self::DuplicateLiveWindowId(id) => {
                write!(formatter, "duplicate live window id {id}")
            }
            Self::DuplicateTransportHandle(handle) => {
                write!(formatter, "duplicate live transport handle {handle}")
            }
        }
    }
}

impl Error for WindowDiffError {}
