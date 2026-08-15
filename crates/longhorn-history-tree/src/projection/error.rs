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
        match self {
            Self::ZeroPageSize => formatter.write_str("fork projection page size is zero"),
            Self::PageTooLarge { maximum, actual } => write!(
                formatter,
                "fork projection page size {actual} exceeds maximum {maximum}"
            ),
            Self::OffsetOutOfRange { maximum, actual } => write!(
                formatter,
                "fork projection offset {actual} exceeds maximum {maximum}"
            ),
            Self::UnknownBranch(id) => write!(formatter, "fork branch {id} does not exist"),
            Self::UnknownEntry(id) => {
                write!(formatter, "fork history entry {id} is not retained")
            }
            Self::InvalidTopology => {
                formatter.write_str("fork history topology could not be projected consistently")
            }
        }
    }
}

impl Error for ForkProjectionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fork_projection_error_messages_are_hand_written() {
        let cases: [(ForkProjectionError, &str); 6] = [
            (
                ForkProjectionError::ZeroPageSize,
                "fork projection page size is zero",
            ),
            (
                ForkProjectionError::PageTooLarge {
                    maximum: 128,
                    actual: 129,
                },
                "fork projection page size 129 exceeds maximum 128",
            ),
            (
                ForkProjectionError::OffsetOutOfRange {
                    maximum: 10,
                    actual: 11,
                },
                "fork projection offset 11 exceeds maximum 10",
            ),
            (
                ForkProjectionError::UnknownBranch(
                    ForkBranchId::new("branch:gone").expect("fixture branch id"),
                ),
                "fork branch branch:gone does not exist",
            ),
            (
                ForkProjectionError::UnknownEntry(
                    HistoryEntryId::new("entry:gone").expect("fixture entry id"),
                ),
                "fork history entry entry:gone is not retained",
            ),
            (
                ForkProjectionError::InvalidTopology,
                "fork history topology could not be projected consistently",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
