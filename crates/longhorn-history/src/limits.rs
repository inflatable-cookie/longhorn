use std::{error::Error, fmt};

use crate::MAXIMUM_HISTORY_LABEL_BYTES;

/// Defensive hard ceiling for retained entries before retention policy exists.
pub const MAXIMUM_HISTORY_ENTRIES: usize = 65_536;

/// Explicit count and metadata limits for one linear history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryLimits {
    maximum_entries: usize,
    maximum_label_bytes: usize,
}

impl HistoryLimits {
    /// Validates and constructs limits.
    pub const fn new(
        maximum_entries: usize,
        maximum_label_bytes: usize,
    ) -> Result<Self, HistoryLimitsError> {
        if maximum_entries == 0 || maximum_label_bytes == 0 {
            return Err(HistoryLimitsError::Zero);
        }
        if maximum_entries > MAXIMUM_HISTORY_ENTRIES {
            return Err(HistoryLimitsError::TooManyEntries {
                maximum: MAXIMUM_HISTORY_ENTRIES,
                actual: maximum_entries,
            });
        }
        if maximum_label_bytes > MAXIMUM_HISTORY_LABEL_BYTES {
            return Err(HistoryLimitsError::LabelBytesTooLarge {
                maximum: MAXIMUM_HISTORY_LABEL_BYTES,
                actual: maximum_label_bytes,
            });
        }
        Ok(Self {
            maximum_entries,
            maximum_label_bytes,
        })
    }

    /// Returns the maximum entries across applied and future state.
    #[must_use]
    pub const fn maximum_entries(self) -> usize {
        self.maximum_entries
    }

    /// Returns the configured maximum label byte length.
    #[must_use]
    pub const fn maximum_label_bytes(self) -> usize {
        self.maximum_label_bytes
    }
}

impl Default for HistoryLimits {
    fn default() -> Self {
        Self {
            maximum_entries: 100,
            maximum_label_bytes: 1_024,
        }
    }
}

/// Invalid history limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryLimitsError {
    /// A configured limit was zero.
    Zero,
    /// The entry limit exceeded the defensive ceiling.
    TooManyEntries {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
    /// The label-byte limit exceeded the defensive ceiling.
    LabelBytesTooLarge {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
}

impl fmt::Display for HistoryLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("history limits must be nonzero"),
            Self::TooManyEntries { maximum, actual } => write!(
                formatter,
                "history entry limit is {actual}; hard maximum is {maximum}"
            ),
            Self::LabelBytesTooLarge { maximum, actual } => write!(
                formatter,
                "history label limit is {actual} bytes; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryLimitsError {}
