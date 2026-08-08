//! Protocol input and projection errors.

use std::{error::Error, fmt};

use super::{OPERATION_PROTOCOL_VERSION, OperationRejection, OperationRejectionCode};

pub(crate) fn incompatible_protocol() -> OperationRejection {
    OperationRejection {
        code: OperationRejectionCode::IncompatibleProtocol,
        detail: format!("operation protocol version must be {OPERATION_PROTOCOL_VERSION}"),
        refresh_required: false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum OperationProtocolInputError {
    AuthorityEpoch,
    Progress(String),
    Phase(String),
    Label(String),
    Limits,
}

impl fmt::Display for OperationProtocolInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorityEpoch => {
                formatter.write_str("operation authority epoch must be nonzero")
            }
            Self::Progress(detail) | Self::Phase(detail) | Self::Label(detail) => {
                formatter.write_str(detail)
            }
            Self::Limits => formatter.write_str("operation catalogue limits are invalid"),
        }
    }
}

/// A bounded internal projection could not fit the protocol integer domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProtocolProjectionError(pub(crate) String);

impl fmt::Display for OperationProtocolProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for OperationProtocolProjectionError {}

pub(crate) fn project_usize(value: usize) -> Result<u64, OperationProtocolProjectionError> {
    u64::try_from(value)
        .map_err(|_| OperationProtocolProjectionError("operation count does not fit u64".into()))
}
