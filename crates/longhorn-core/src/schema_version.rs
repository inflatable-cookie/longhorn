use std::{error::Error, fmt, num::NonZeroU32};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Positive schema version for a serialized Longhorn domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SchemaVersion(NonZeroU32);

impl SchemaVersion {
    /// Validates and constructs a schema version.
    pub const fn new(value: u32) -> Result<Self, SchemaVersionError> {
        match NonZeroU32::new(value) {
            Some(value) => Ok(Self(value)),
            None => Err(SchemaVersionError::Zero),
        }
    }

    /// Returns the numeric schema version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// Returns the following schema version when it fits in `u32`.
    #[must_use]
    pub fn checked_next(self) -> Option<Self> {
        self.get()
            .checked_add(1)
            .and_then(|value| Self::new(value).ok())
    }
}

impl TryFrom<u32> for SchemaVersion {
    type Error = SchemaVersionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SchemaVersion> for u32 {
    fn from(value: SchemaVersion) -> Self {
        value.get()
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(formatter)
    }
}

impl Serialize for SchemaVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.get())
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Schema version validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaVersionError {
    /// Schema version zero is invalid.
    Zero,
}

impl fmt::Display for SchemaVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("schema version must be positive")
    }
}

impl Error for SchemaVersionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero() {
        assert_eq!(SchemaVersion::new(0), Err(SchemaVersionError::Zero));
    }

    #[test]
    fn serde_round_trip_uses_a_number() {
        let version = SchemaVersion::new(4).unwrap();
        let json = serde_json::to_string(&version).unwrap();

        assert_eq!(json, "4");
        assert_eq!(
            serde_json::from_str::<SchemaVersion>(&json).unwrap(),
            version
        );
    }
}
