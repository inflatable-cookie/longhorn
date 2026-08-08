use std::cmp::Ordering;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Finite canonical floating-point value accepted by command schemas.
/// Finite canonical floating-point value accepted by command schemas.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct CommandFiniteNumber(f64);

impl CommandFiniteNumber {
    /// Constructs a finite value, normalizing negative zero.
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() {
            return Err("command number must be finite");
        }
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }

    /// Returns the finite value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for CommandFiniteNumber {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for CommandFiniteNumber {}

impl PartialOrd for CommandFiniteNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CommandFiniteNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Serialize for CommandFiniteNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for CommandFiniteNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(f64::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}
