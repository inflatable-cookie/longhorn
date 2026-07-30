use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::SettingsLimits;

/// Consumer-owned JSON value carried without becoming Longhorn product schema.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "bindings",
    ts(type = "{ codecVersion: number; value: unknown }")
)]
pub struct SettingsOpaqueValue {
    codec_version: u32,
    value: serde_json::Value,
    encoded_bytes: usize,
}

impl SettingsOpaqueValue {
    /// Validates and constructs a versioned opaque value.
    pub fn new(
        codec_version: u32,
        value: serde_json::Value,
        limits: SettingsLimits,
    ) -> Result<Self, SettingsOpaqueValueError> {
        if !limits.is_valid() {
            return Err(SettingsOpaqueValueError::InvalidLimits);
        }
        Self::with_maximum(codec_version, value, limits.maximum_opaque_value_bytes)
    }

    /// Returns the consumer codec version.
    #[must_use]
    pub const fn codec_version(&self) -> u32 {
        self.codec_version
    }

    /// Returns the opaque JSON value.
    #[must_use]
    pub const fn value(&self) -> &serde_json::Value {
        &self.value
    }

    /// Returns the canonical encoded envelope length used for limit checks.
    #[must_use]
    pub const fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }

    fn with_maximum(
        codec_version: u32,
        value: serde_json::Value,
        maximum: usize,
    ) -> Result<Self, SettingsOpaqueValueError> {
        if codec_version == 0 {
            return Err(SettingsOpaqueValueError::InvalidCodecVersion);
        }
        let encoded_bytes = serde_json::to_vec(&OpaqueValueRef {
            codec_version,
            value: &value,
        })
        .map_err(|error| SettingsOpaqueValueError::Encoding(error.to_string()))?
        .len();
        if encoded_bytes > maximum {
            return Err(SettingsOpaqueValueError::TooLarge {
                maximum,
                actual: encoded_bytes,
            });
        }
        Ok(Self {
            codec_version,
            value,
            encoded_bytes,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpaqueValueRef<'value> {
    codec_version: u32,
    value: &'value serde_json::Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct OpaqueValueOwned {
    codec_version: u32,
    value: serde_json::Value,
}

impl Serialize for SettingsOpaqueValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        OpaqueValueRef {
            codec_version: self.codec_version,
            value: &self.value,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SettingsOpaqueValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = OpaqueValueOwned::deserialize(deserializer)?;
        Self::with_maximum(
            value.codec_version,
            value.value,
            SettingsLimits::HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
        )
        .map_err(de::Error::custom)
    }
}

/// Invalid consumer-owned opaque settings value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsOpaqueValueError {
    /// Explicit settings limits are invalid.
    InvalidLimits,
    /// Codec version zero is reserved and cannot identify consumer schema.
    InvalidCodecVersion,
    /// The canonical envelope exceeds its configured byte limit.
    TooLarge {
        /// Maximum accepted bytes.
        maximum: usize,
        /// Actual canonical bytes.
        actual: usize,
    },
    /// The JSON envelope could not be encoded.
    Encoding(String),
}

impl fmt::Display for SettingsOpaqueValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("settings value limits are invalid"),
            Self::InvalidCodecVersion => {
                formatter.write_str("settings value codec version must be nonzero")
            }
            Self::TooLarge { maximum, actual } => {
                write!(
                    formatter,
                    "opaque settings value is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::Encoding(detail) => {
                write!(
                    formatter,
                    "could not encode opaque settings value: {detail}"
                )
            }
        }
    }
}

impl Error for SettingsOpaqueValueError {}
