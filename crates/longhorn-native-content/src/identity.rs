use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Monotonic nonzero identity for one native-content attach attempt.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct AttachGeneration(u64);

impl AttachGeneration {
    /// First generation for a new native-content island.
    pub const INITIAL: Self = Self(1);

    /// Validates and constructs an attach generation.
    pub const fn new(value: u64) -> Result<Self, PositiveCounterError> {
        if value == 0 {
            Err(PositiveCounterError::AttachGenerationZero)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the serialized generation value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next generation or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, CounterOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(CounterOverflow::AttachGeneration),
        }
    }
}

impl Serialize for AttachGeneration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for AttachGeneration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u64::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Ordered nonzero identity for one operation in an apply plan.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct PlanStepId(u16);

impl PlanStepId {
    /// Validates and constructs a plan-local step identity.
    pub const fn new(value: u16) -> Result<Self, PositiveCounterError> {
        if value == 0 {
            Err(PositiveCounterError::PlanStepZero)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the serialized step number.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    pub(crate) fn from_plan_index(index: usize) -> Self {
        let value = u16::try_from(index + 1).expect("native-content plans are statically bounded");
        Self(value)
    }
}

impl Serialize for PlanStepId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.0)
    }
}

impl<'de> Deserialize<'de> for PlanStepId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u16::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// A serialized native-content counter violated its nonzero invariant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositiveCounterError {
    /// Attach generation zero was supplied.
    AttachGenerationZero,
    /// Apply-plan step zero was supplied.
    PlanStepZero,
}

impl fmt::Display for PositiveCounterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AttachGenerationZero => {
                formatter.write_str("attach generation must be greater than zero")
            }
            Self::PlanStepZero => formatter.write_str("plan step must be greater than zero"),
        }
    }
}

impl Error for PositiveCounterError {}

/// A native-content monotonic counter could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterOverflow {
    /// The attach-generation counter reached its maximum.
    AttachGeneration,
}

impl fmt::Display for CounterOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("attach generation cannot advance beyond u64::MAX")
    }
}

impl Error for CounterOverflow {}
