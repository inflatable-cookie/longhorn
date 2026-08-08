//! Protocol projection errors and helpers.

use std::{error::Error, fmt};

/// Failed conversion into the renderer protocol surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryProtocolProjectionError {
    /// A collection count exceeded the protocol u64 domain.
    CountOverflow,
}

impl fmt::Display for HistoryProtocolProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountOverflow => formatter.write_str("history protocol count overflow"),
        }
    }
}

impl Error for HistoryProtocolProjectionError {}

pub(crate) fn project_count(value: usize) -> Result<u64, HistoryProtocolProjectionError> {
    u64::try_from(value).map_err(|_| HistoryProtocolProjectionError::CountOverflow)
}
