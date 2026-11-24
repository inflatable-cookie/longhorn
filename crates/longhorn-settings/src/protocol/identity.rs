use std::{error::Error, fmt};

use longhorn_core::WireCounter;
use serde::{Deserialize, Deserializer, Serialize, de};

/// Current settings wire protocol version.
pub const SETTINGS_PROTOCOL_VERSION: u16 = 1;

/// Exact settings protocol version accepted by this crate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct SettingsProtocolVersion(u16);

impl SettingsProtocolVersion {
    /// Current supported protocol version.
    pub const CURRENT: Self = Self(SETTINGS_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for SettingsProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let actual = u16::deserialize(deserializer)?;
        if actual == SETTINGS_PROTOCOL_VERSION {
            Ok(Self::CURRENT)
        } else {
            Err(de::Error::custom(format!(
                "unsupported settings protocol version {actual}; expected {SETTINGS_PROTOCOL_VERSION}"
            )))
        }
    }
}

/// Monotonic revision of one authoritative settings scope.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct SettingsScopeRevision(WireCounter);

impl SettingsScopeRevision {
    /// Initial revision for a new authoritative scope.
    pub const INITIAL: Self = Self(WireCounter::ZERO);

    /// Constructs a revision from its serialized value.
    ///
    /// # Panics
    ///
    /// Panics if the value exceeds [`WireCounter::MAX`]. Use deserialization or
    /// [`Self::checked_next`] for values that may be out of range.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(WireCounter::new(value))
    }

    /// Returns the serialized revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Advances the revision without wrapping past the wire bound.
    pub const fn checked_next(self) -> Result<Self, SettingsProtocolError> {
        match self.0.checked_add(1) {
            Ok(value) => Ok(Self(value)),
            Err(_) => Err(SettingsProtocolError::ScopeRevisionOverflow),
        }
    }
}

/// Settings protocol construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsProtocolError {
    /// A scope revision cannot advance beyond the JavaScript safe integer bound.
    ScopeRevisionOverflow,
}

impl fmt::Display for SettingsProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScopeRevisionOverflow => write!(
                formatter,
                "settings scope revision cannot advance beyond {}",
                WireCounter::MAX
            ),
        }
    }
}

impl Error for SettingsProtocolError {}
