//! Operational failures for command keymap service.

use std::{error::Error, fmt};

use longhorn_config::{CoordinatedLoadError, MutationError, StoreError, UnavailableState};

use crate::CommandKeymapRecovery;

use super::internal::unavailable_code;

/// Operational failure outside normal stale or rejected domain results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandKeymapServiceError {
    /// Coordinated read authority failed.
    Coordination(CoordinatedLoadError),
    /// Domain registration changed or is missing.
    Store(StoreError),
    /// Coordinated mutation or publication failed.
    Mutation(MutationError),
    /// Source is in explicit recovery.
    Recovery(CommandKeymapRecovery),
    /// Required storage authority is unavailable.
    Unavailable(UnavailableState),
    /// Current supposedly-valid state could not project.
    InvalidState(String),
    /// Canonical patch encoding failed.
    Encoding(String),
}

impl fmt::Display for CommandKeymapServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordination(error) => error.fmt(formatter),
            Self::Store(error) => error.fmt(formatter),
            Self::Mutation(error) => error.fmt(formatter),
            Self::Recovery(recovery) => formatter.write_str(&recovery.detail),
            Self::Unavailable(state) => formatter.write_str(unavailable_code(state)),
            Self::InvalidState(detail) | Self::Encoding(detail) => formatter.write_str(detail),
        }
    }
}

impl Error for CommandKeymapServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Coordination(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Mutation(error) => Some(error),
            Self::Recovery(_)
            | Self::Unavailable(_)
            | Self::InvalidState(_)
            | Self::Encoding(_) => None,
        }
    }
}
