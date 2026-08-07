use core::fmt;

use serde::{Deserialize, Serialize};

/// A point in time, as seconds since the Unix epoch.
///
/// Deliberately a plain integer rather than a date type: this crate is pure
/// policy and never reads a clock. Every function that needs "now" is handed
/// one, which is also what makes expiry testable without waiting.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct Timestamp(i64);

impl Timestamp {
    /// Records a point in time.
    #[must_use]
    pub const fn from_unix_seconds(value: i64) -> Self {
        Self(value)
    }

    /// Returns seconds since the Unix epoch.
    #[must_use]
    pub const fn as_unix_seconds(self) -> i64 {
        self.0
    }

    /// Returns this point advanced by a span, saturating at the bounds.
    #[must_use]
    pub const fn saturating_add(self, span: Span) -> Self {
        Self(self.0.saturating_add(span.as_seconds()))
    }

    /// Returns how far `self` is behind `other`, or zero when it is not.
    #[must_use]
    pub const fn saturating_behind(self, other: Self) -> Span {
        // `from_seconds` already clamps a negative result to zero, so this
        // stays const without needing `Ord::max`.
        Span::from_seconds(other.0.saturating_sub(self.0))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A non-negative duration in seconds.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "i64", into = "i64")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct Span(i64);

impl Span {
    /// No time at all.
    pub const ZERO: Self = Self(0);

    /// Records a span, clamping a negative value to zero.
    #[must_use]
    pub const fn from_seconds(value: i64) -> Self {
        Self(if value < 0 { 0 } else { value })
    }

    /// Records a span in days.
    #[must_use]
    pub const fn from_days(days: i64) -> Self {
        Self::from_seconds(days.saturating_mul(86_400))
    }

    /// Returns the span in seconds.
    #[must_use]
    pub const fn as_seconds(self) -> i64 {
        self.0
    }
}

impl From<Span> for i64 {
    fn from(value: Span) -> Self {
        value.0
    }
}

impl From<i64> for Span {
    fn from(value: i64) -> Self {
        Self::from_seconds(value)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}s", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_negative_span_clamps_to_zero() {
        assert_eq!(Span::from_seconds(-5), Span::ZERO);
        assert_eq!(serde_json::from_str::<Span>("-5").unwrap(), Span::ZERO);
    }

    #[test]
    fn behind_is_zero_when_ahead() {
        let earlier = Timestamp::from_unix_seconds(100);
        let later = Timestamp::from_unix_seconds(160);

        assert_eq!(earlier.saturating_behind(later), Span::from_seconds(60));
        assert_eq!(later.saturating_behind(earlier), Span::ZERO);
    }
}
