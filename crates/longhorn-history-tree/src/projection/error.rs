//! Projection rejection errors.

use std::{error::Error, fmt};

use longhorn_core::HistoryEntryId;

use crate::ForkBranchId;
/// Rejected bounded graph projection.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkProjectionError {
    /// Requested page size was zero.
    ZeroPageSize,
    /// Requested page exceeded the shared hard ceiling.
    PageTooLarge {
        /// Shared hard maximum.
        maximum: usize,
        /// Supplied size.
        actual: usize,
    },
    /// Requested offset exceeded the selected collection.
    OffsetOutOfRange {
        /// Maximum accepted offset.
        maximum: usize,
        /// Supplied offset.
        actual: usize,
    },
    /// Explicit path named no branch reference.
    UnknownBranch(ForkBranchId),
    /// Anchor or run start named no retained entry.
    UnknownEntry(HistoryEntryId),
    /// Validated topology could not be projected consistently.
    InvalidTopology,
}

impl fmt::Display for ForkProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "fork projection failed: {self:?}")
    }
}

impl Error for ForkProjectionError {}
