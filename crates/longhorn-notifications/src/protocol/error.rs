//! Protocol input and projection errors.

use std::{error::Error, fmt};

use super::{NOTIFICATION_PROTOCOL_VERSION, NotificationRejection, NotificationRejectionCode};

pub(crate) fn incompatible_rejection() -> NotificationRejection {
    NotificationRejection {
        code: NotificationRejectionCode::IncompatibleProtocol,
        detail: format!("notification protocol version must be {NOTIFICATION_PROTOCOL_VERSION}"),
        refresh_required: false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NotificationProtocolInputError {
    AuthorityEpoch,
    Metadata(String),
    Limits,
}

impl fmt::Display for NotificationProtocolInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorityEpoch => {
                formatter.write_str("notification authority epoch must be nonzero")
            }
            Self::Metadata(detail) => formatter.write_str(detail),
            Self::Limits => formatter.write_str("notification ledger limits are invalid"),
        }
    }
}

/// A protocol query or projection could not be represented safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationProtocolError(pub(crate) String);

impl NotificationProtocolError {
    pub(crate) fn incompatible() -> Self {
        Self(format!(
            "notification protocol version must be {NOTIFICATION_PROTOCOL_VERSION}"
        ))
    }

    pub(crate) fn input(detail: impl Into<String>) -> Self {
        Self(detail.into())
    }

    pub(crate) fn projection(detail: impl Into<String>) -> Self {
        Self(detail.into())
    }
}

impl fmt::Display for NotificationProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for NotificationProtocolError {}

pub(crate) fn project_usize(value: usize) -> Result<u64, NotificationProtocolError> {
    u64::try_from(value)
        .map_err(|_| NotificationProtocolError::projection("usize value exceeds u64"))
}
