//! Protocol version, authority epoch, and mode.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Current exact metadata-only renderer protocol version.
pub const HISTORY_PROTOCOL_VERSION: u32 = 1;

/// Exact metadata-only history protocol line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct HistoryProtocolVersion(u32);

impl HistoryProtocolVersion {
    /// Current exact protocol line.
    pub const CURRENT: Self = Self(HISTORY_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Nonzero identity for one live history authority lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct HistoryAuthorityEpoch(u64);

impl HistoryAuthorityEpoch {
    /// Constructs a nonzero live authority epoch.
    pub const fn new(value: u64) -> Result<Self, HistoryAuthorityEpochError> {
        if value == 0 {
            Err(HistoryAuthorityEpochError)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the serialized epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for HistoryAuthorityEpoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// A history authority epoch was zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryAuthorityEpochError;

impl fmt::Display for HistoryAuthorityEpochError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("history authority epoch must be nonzero")
    }
}

impl Error for HistoryAuthorityEpochError {}

/// Public history topology mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryProtocolMode {
    /// One applied path and one retained redo path.
    Linear,
}

/// Authoritative topology position of one projected entry.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryProjectionPosition {
    /// Applied before the current entry.
    Past,
    /// Current applied entry.
    Current,
    /// Retained redo entry.
    Future,
}
