use std::{error::Error, fmt};

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
pub struct SettingsScopeRevision(u64);

impl SettingsScopeRevision {
    /// Initial revision for a new authoritative scope.
    pub const INITIAL: Self = Self(0);

    /// Constructs a revision from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Advances the revision without wrapping.
    pub const fn checked_next(self) -> Result<Self, SettingsProtocolError> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(SettingsProtocolError::ScopeRevisionOverflow),
        }
    }
}

/// Settings protocol construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsProtocolError {
    /// A scope revision cannot advance beyond `u64::MAX`.
    ScopeRevisionOverflow,
}

impl fmt::Display for SettingsProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScopeRevisionOverflow => {
                formatter.write_str("settings scope revision cannot advance beyond u64::MAX")
            }
        }
    }
}

impl Error for SettingsProtocolError {}
