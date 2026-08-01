use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use super::NATIVE_CONTENT_PROTOCOL_VERSION;

/// Exact native-content protocol version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct NativeContentProtocolVersion(u32);

impl NativeContentProtocolVersion {
    /// Current exact protocol version.
    pub const CURRENT: Self = Self(NATIVE_CONTENT_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

macro_rules! positive_epoch {
    ($name:ident, $zero:ident, $overflow:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        pub struct $name(u64);

        impl $name {
            /// Validates and constructs the epoch.
            pub const fn new(value: u64) -> Result<Self, NativeContentProtocolCounterError> {
                if value == 0 {
                    Err(NativeContentProtocolCounterError::$zero)
                } else {
                    Ok(Self(value))
                }
            }

            /// Returns the serialized value.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }

            /// Returns the next epoch or fails instead of wrapping.
            pub const fn checked_next(self) -> Result<Self, NativeContentProtocolCounterError> {
                match self.0.checked_add(1) {
                    Some(value) => Ok(Self(value)),
                    None => Err(NativeContentProtocolCounterError::Overflow($overflow)),
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_u64(self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::new(u64::deserialize(deserializer)?).map_err(de::Error::custom)
            }
        }
    };
}

positive_epoch!(
    NativeContentAuthorityEpoch,
    AuthorityEpochZero,
    "native-content authority epoch",
    "Nonzero lifetime of one authoritative native-content protocol host."
);
positive_epoch!(
    NativeContentClientEpoch,
    ClientEpochZero,
    "native-content client epoch",
    "Host-issued nonzero renderer session epoch, distinct from attach generation."
);

/// Invalid or exhausted native-content protocol epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeContentProtocolCounterError {
    /// Authority epochs cannot be zero.
    AuthorityEpochZero,
    /// Client epochs cannot be zero.
    ClientEpochZero,
    /// A monotonic epoch could not advance.
    Overflow(&'static str),
}

impl fmt::Display for NativeContentProtocolCounterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorityEpochZero => formatter.write_str("authority epoch must be nonzero"),
            Self::ClientEpochZero => formatter.write_str("client epoch must be nonzero"),
            Self::Overflow(name) => write!(formatter, "{name} is exhausted"),
        }
    }
}

impl Error for NativeContentProtocolCounterError {}
