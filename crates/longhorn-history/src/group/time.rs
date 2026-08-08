//! Injected monotonic time and consumer-selected timed-group duration.

use std::{error::Error, fmt};

/// Caller-injected monotonic millisecond reading.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HistoryMonotonicMillis(u64);

impl HistoryMonotonicMillis {
    /// Constructs an injected monotonic reading.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the injected reading.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Nonzero consumer-selected timed-group gap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryGroupDurationMillis(u64);

impl HistoryGroupDurationMillis {
    /// Validates a consumer-selected duration.
    pub const fn new(value: u64) -> Result<Self, HistoryGroupDurationError> {
        if value == 0 {
            return Err(HistoryGroupDurationError);
        }
        Ok(Self(value))
    }

    /// Returns the duration in milliseconds.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A zero timed-group duration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryGroupDurationError;

impl fmt::Display for HistoryGroupDurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("history group duration must be nonzero")
    }
}

impl Error for HistoryGroupDurationError {}
