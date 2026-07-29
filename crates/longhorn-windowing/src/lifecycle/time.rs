use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Caller-supplied monotonic milliseconds from an arbitrary process-local epoch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MonotonicMillis(u64);

impl MonotonicMillis {
    /// Constructs a monotonic timestamp.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the caller's monotonic millisecond value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn checked_add(
        self,
        duration: WindowLifecycleDuration,
    ) -> Result<Self, WindowLifecycleError> {
        self.0
            .checked_add(duration.0)
            .map(Self)
            .ok_or(WindowLifecycleError::DeadlineOverflow { at: self, duration })
    }
}

/// Caller-supplied lifecycle interval in milliseconds.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WindowLifecycleDuration(u64);

impl WindowLifecycleDuration {
    /// Constructs an interval. Zero explicitly disables that interval.
    #[must_use]
    pub const fn from_millis(value: u64) -> Self {
        Self(value)
    }

    /// Returns the interval in milliseconds.
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

/// Pure lifecycle arithmetic failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowLifecycleError {
    /// A caller-supplied timestamp and interval exceeded the monotonic domain.
    DeadlineOverflow {
        /// Starting timestamp.
        at: MonotonicMillis,
        /// Interval that could not be added.
        duration: WindowLifecycleDuration,
    },
    /// One window exhausted its capture generation domain.
    CaptureGenerationExhausted,
}

impl fmt::Display for WindowLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeadlineOverflow { at, duration } => write!(
                formatter,
                "lifecycle deadline overflow at {} plus {} milliseconds",
                at.get(),
                duration.as_millis()
            ),
            Self::CaptureGenerationExhausted => {
                formatter.write_str("window capture generation exhausted")
            }
        }
    }
}

impl Error for WindowLifecycleError {}
