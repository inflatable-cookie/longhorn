use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{BridgeNegotiationError, BridgeNegotiationErrorCode};

/// Current bridge wire protocol version.
pub const BRIDGE_PROTOCOL_VERSION: u16 = 1;

/// Exact bridge protocol version accepted by this crate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeProtocolVersion(u16);

impl BridgeProtocolVersion {
    /// Current supported protocol version.
    pub const CURRENT: Self = Self(BRIDGE_PROTOCOL_VERSION);

    /// Validates an exact serialized protocol version.
    pub fn new(actual: u16) -> Result<Self, BridgeNegotiationError> {
        if actual == BRIDGE_PROTOCOL_VERSION {
            Ok(Self::CURRENT)
        } else {
            Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::IncompatibleProtocol,
                format!(
                    "unsupported bridge protocol version {actual}; expected {BRIDGE_PROTOCOL_VERSION}"
                ),
            ))
        }
    }

    /// Returns the serialized protocol version.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for BridgeProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let actual = u16::deserialize(deserializer)?;
        Self::new(actual).map_err(de::Error::custom)
    }
}

impl fmt::Display for BridgeProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.get())
    }
}
