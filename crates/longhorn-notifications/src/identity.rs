use std::{error::Error, fmt};

/// Nonzero epoch for one live notification authority instance.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NotificationAuthorityEpoch(u64);

impl NotificationAuthorityEpoch {
    /// Validates and constructs an authority epoch.
    pub const fn new(value: u64) -> Result<Self, NotificationAuthorityEpochError> {
        if value == 0 {
            Err(NotificationAuthorityEpochError)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Authority epoch zero is reserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotificationAuthorityEpochError;

impl fmt::Display for NotificationAuthorityEpochError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("notification authority epoch must be nonzero")
    }
}

impl Error for NotificationAuthorityEpochError {}

/// Monotonic insertion sequence for one notification ledger.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NotificationSequence(u64);

impl NotificationSequence {
    /// First sequence allocated by an empty ledger.
    pub const FIRST: Self = Self(1);

    /// Returns the numeric sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next sequence or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, NotificationSequenceOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(NotificationSequenceOverflow),
        }
    }

    #[cfg(test)]
    pub(crate) const fn for_test(value: u64) -> Self {
        Self(value)
    }
}

/// A notification sequence could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotificationSequenceOverflow;

impl fmt::Display for NotificationSequenceOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("notification sequence cannot advance beyond u64::MAX")
    }
}

impl Error for NotificationSequenceOverflow {}
