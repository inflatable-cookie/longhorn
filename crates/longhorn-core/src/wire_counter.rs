//! Nonnegative counters shared with JavaScript number codecs.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// A nonnegative integer that round-trips exactly as a JavaScript number.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(try_from = "u64")]
pub struct WireCounter(u64);

impl WireCounter {
    /// Largest admitted integer, equal to JavaScript's `Number.MAX_SAFE_INTEGER`.
    pub const MAX: u64 = (1 << 53) - 1;

    /// The initial value of a zero-based counter.
    pub const ZERO: Self = Self(0);

    /// Constructs a counter from a known in-range value.
    ///
    /// # Panics
    ///
    /// Panics if the value exceeds [`Self::MAX`]. Use `TryFrom` for external values.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        assert!(
            value <= Self::MAX,
            "wire counter exceeds the JavaScript safe integer range"
        );
        Self(value)
    }

    /// Returns the counter's serialized value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Adds a delta or reports that the result exceeds the wire range.
    pub const fn checked_add(self, delta: u64) -> Result<Self, WireCounterOverflow> {
        match self.0.checked_add(delta) {
            Some(value) if value <= Self::MAX => Ok(Self(value)),
            _ => Err(WireCounterOverflow),
        }
    }
}

impl TryFrom<u64> for WireCounter {
    type Error = WireCounterOverflow;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::ZERO.checked_add(value)
    }
}

/// Counter arithmetic exceeded the JavaScript safe integer range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireCounterOverflow;

impl fmt::Display for WireCounterOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "wire counter exceeds {}", WireCounter::MAX)
    }
}

impl Error for WireCounterOverflow {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_and_arithmetic_respect_the_same_boundary() {
        let max = WireCounter::MAX.to_string();
        let counter: WireCounter = serde_json::from_str(&max).unwrap();
        assert_eq!(serde_json::to_string(&counter).unwrap(), max);
        assert_eq!(counter.checked_add(1), Err(WireCounterOverflow));
        assert!(serde_json::from_str::<WireCounter>(&(WireCounter::MAX + 1).to_string()).is_err());
    }
}
