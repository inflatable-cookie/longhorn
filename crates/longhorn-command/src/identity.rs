use longhorn_core::{WireCounter, WireCounterOverflow};
use std::fmt;

use longhorn_core::bytes_to_lowercase_hex;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};

/// Monotonic identity of one sealed host command registry composition.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct CommandRegistryGeneration(WireCounter);

impl CommandRegistryGeneration {
    /// Initial generation for the first host composition.
    pub const INITIAL: Self = Self(WireCounter::ZERO);

    /// Constructs a generation from its serialized value.
    ///
    /// # Panics
    ///
    /// Panics if the value exceeds [`WireCounter::MAX`].
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(WireCounter::new(value))
    }

    /// Returns the serialized generation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Returns the next generation without wrapping.
    pub const fn checked_next(self) -> Result<Self, WireCounterOverflow> {
        match self.0.checked_add(1) {
            Ok(value) => Ok(Self(value)),
            Err(error) => Err(error),
        }
    }
}

/// Lowercase hexadecimal SHA-256 digest of canonical sealed registry content.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct CommandRegistryDigest(String);

impl CommandRegistryDigest {
    /// Returns the serialized lowercase hexadecimal digest.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes_to_lowercase_hex(&Sha256::digest(bytes)))
    }

    pub(crate) fn placeholder() -> Self {
        Self(String::new())
    }

    fn parse(value: String) -> Result<Self, &'static str> {
        if value.len() != 64 {
            return Err("command registry digest must contain 64 hexadecimal bytes");
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("command registry digest must be lowercase hexadecimal");
        }
        Ok(Self(value))
    }
}

impl fmt::Display for CommandRegistryDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for CommandRegistryDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CommandRegistryDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}
