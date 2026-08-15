use std::{error::Error, fmt};

use longhorn_core::bytes_to_lowercase_hex;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Monotonic revision of authoritative active-preset and sparse override state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct CommandKeymapRevision(u64);

impl CommandKeymapRevision {
    /// Initial empty keymap configuration revision.
    pub const INITIAL: Self = Self(0);

    /// Constructs a revision.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next revision without wrapping.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl Serialize for CommandKeymapRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for CommandKeymapRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(u64::deserialize(deserializer)?))
    }
}

/// Lowercase SHA-256 digest of one canonical typed keymap patch.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct CommandKeymapPatchDigest(String);

impl CommandKeymapPatchDigest {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256};

        Self(bytes_to_lowercase_hex(&Sha256::digest(bytes)))
    }

    /// Returns the lowercase hexadecimal digest.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(value: String) -> Result<Self, CommandKeymapPatchDigestError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(CommandKeymapPatchDigestError);
        }
        Ok(Self(value))
    }
}

impl Serialize for CommandKeymapPatchDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CommandKeymapPatchDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Invalid patch digest encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommandKeymapPatchDigestError;

impl fmt::Display for CommandKeymapPatchDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("command keymap patch digest must be 64 lowercase hexadecimal bytes")
    }
}

impl Error for CommandKeymapPatchDigestError {}
