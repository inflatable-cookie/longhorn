use std::{error::Error, fmt};

/// Monotonic insertion sequence for retained history entries.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HistoryEntrySequence(u64);

impl HistoryEntrySequence {
    /// First sequence allocated by an empty history.
    pub const FIRST: Self = Self(1);

    /// Constructs a nonzero sequence.
    pub const fn new(value: u64) -> Result<Self, HistoryEntrySequenceZero> {
        if value == 0 {
            Err(HistoryEntrySequenceZero)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next sequence or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, HistoryEntrySequenceOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(HistoryEntrySequenceOverflow),
        }
    }
}

/// Entry sequence zero is reserved and cannot identify an insertion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryEntrySequenceZero;

impl fmt::Display for HistoryEntrySequenceZero {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("history entry sequence must be nonzero")
    }
}

impl Error for HistoryEntrySequenceZero {}

/// A history entry sequence could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryEntrySequenceOverflow;

impl fmt::Display for HistoryEntrySequenceOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("history entry sequence cannot advance beyond u64::MAX")
    }
}

impl Error for HistoryEntrySequenceOverflow {}
