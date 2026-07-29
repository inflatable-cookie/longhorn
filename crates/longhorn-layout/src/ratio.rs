use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Maximum ratio value representing exactly one.
pub const RATIO_ONE_MILLIONTHS: u32 = 1_000_000;

/// Cross-language-stable layout ratio encoded as integer millionths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct LayoutRatio(u32);

impl LayoutRatio {
    /// Zero ratio.
    pub const ZERO: Self = Self(0);
    /// Ratio representing exactly one.
    pub const ONE: Self = Self(RATIO_ONE_MILLIONTHS);

    /// Constructs a ratio from integer millionths.
    pub const fn from_millionths(value: u32) -> Result<Self, LayoutRatioError> {
        if value <= RATIO_ONE_MILLIONTHS {
            Ok(Self(value))
        } else {
            Err(LayoutRatioError {
                maximum: RATIO_ONE_MILLIONTHS,
                actual: value,
            })
        }
    }

    /// Returns the serialized integer millionths.
    #[must_use]
    pub const fn millionths(self) -> u32 {
        self.0
    }
}

/// Ratio exceeded the closed zero-to-one interval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutRatioError {
    maximum: u32,
    actual: u32,
}

impl LayoutRatioError {
    /// Returns the maximum accepted millionths.
    #[must_use]
    pub const fn maximum(self) -> u32 {
        self.maximum
    }

    /// Returns the rejected millionths.
    #[must_use]
    pub const fn actual(self) -> u32 {
        self.actual
    }
}

impl fmt::Display for LayoutRatioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "layout ratio {} exceeds maximum {} millionths",
            self.actual, self.maximum
        )
    }
}

impl Error for LayoutRatioError {}
