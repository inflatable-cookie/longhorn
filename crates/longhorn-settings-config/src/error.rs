use std::{error::Error, fmt};

use longhorn_config::{CoordinatedLoadError, DomainIssue, MutationError, StoreError};
use longhorn_settings::SettingsProtocolError;

use crate::SettingsConfigProjectionError;

/// Failure to load and project one config-backed settings scope.
#[derive(Debug)]
pub enum SettingsConfigLoadError {
    /// Stable coordinated read could not complete.
    Coordination(CoordinatedLoadError),
    /// The domain is not registered or its descriptor changed.
    Store(StoreError),
    /// The domain could not encode a validated value for authority evidence.
    Encode(DomainIssue),
    /// Consumer projection failed.
    Projection(SettingsConfigProjectionError),
}

impl fmt::Display for SettingsConfigLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordination(error) => error.fmt(formatter),
            Self::Store(error) => error.fmt(formatter),
            Self::Encode(error) => write!(formatter, "{}: {}", error.code, error.message),
            Self::Projection(error) => error.fmt(formatter),
        }
    }
}

impl Error for SettingsConfigLoadError {}

/// Operational failure outside an ordinary settings conflict or rejection.
#[derive(Debug)]
pub enum SettingsConfigError {
    /// Stable coordinated load failed.
    Load(SettingsConfigLoadError),
    /// Coordinated mutation or publication failed.
    Mutation(MutationError),
    /// Consumer projection or activation failed.
    Projection(SettingsConfigProjectionError),
    /// The host's process-local authority revision exhausted.
    Protocol(SettingsProtocolError),
    /// Authority material could not be encoded into a host token.
    AuthorityEncoding(String),
    /// A prior panic poisoned process-local authority tracking.
    AuthorityStatePoisoned,
}

impl fmt::Display for SettingsConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => error.fmt(formatter),
            Self::Mutation(error) => error.fmt(formatter),
            Self::Projection(error) => error.fmt(formatter),
            Self::Protocol(error) => error.fmt(formatter),
            Self::AuthorityEncoding(detail) => {
                write!(formatter, "cannot encode settings authority: {detail}")
            }
            Self::AuthorityStatePoisoned => {
                formatter.write_str("settings authority state is poisoned")
            }
        }
    }
}

impl Error for SettingsConfigError {}

impl From<SettingsConfigProjectionError> for SettingsConfigError {
    fn from(error: SettingsConfigProjectionError) -> Self {
        Self::Projection(error)
    }
}
