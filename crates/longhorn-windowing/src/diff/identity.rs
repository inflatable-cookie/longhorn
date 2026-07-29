use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const MAX_HOST_WINDOW_HANDLE_BYTES: usize = 256;

/// Opaque host transport identity such as a Tauri window label.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HostWindowHandle(String);

impl HostWindowHandle {
    /// Validates and constructs a transport handle without interpreting it.
    pub fn new(value: impl Into<String>) -> Result<Self, HostWindowHandleError> {
        let value = value.into();
        if value.is_empty() {
            return Err(HostWindowHandleError::Empty);
        }
        if value.len() > MAX_HOST_WINDOW_HANDLE_BYTES {
            return Err(HostWindowHandleError::TooLong {
                maximum: MAX_HOST_WINDOW_HANDLE_BYTES,
                actual: value.len(),
            });
        }
        if let Some((index, _)) = value
            .char_indices()
            .find(|(_, character)| character.is_control())
        {
            return Err(HostWindowHandleError::ControlCharacter { index });
        }
        Ok(Self(value))
    }

    /// Returns the opaque serialized handle.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HostWindowHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for HostWindowHandle {
    type Err = HostWindowHandleError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl Serialize for HostWindowHandle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HostWindowHandle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Host transport-handle validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostWindowHandleError {
    /// The handle was empty.
    Empty,
    /// The handle exceeded its bounded serialized length.
    TooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// The handle contained a control character.
    ControlCharacter {
        /// Invalid character byte index.
        index: usize,
    },
}

impl fmt::Display for HostWindowHandleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("host window handle cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "host window handle is {actual} bytes; maximum is {maximum}"
            ),
            Self::ControlCharacter { index } => {
                write!(
                    formatter,
                    "host window handle has a control character at byte {index}"
                )
            }
        }
    }
}

impl Error for HostWindowHandleError {}

/// Caller-supplied identity for one programmatic apply pass.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ApplyGeneration(u64);

impl ApplyGeneration {
    /// Constructs a generation. Ordering and rollover policy remain host-owned.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized generation value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
