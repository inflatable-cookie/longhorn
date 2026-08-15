use core::fmt;
use std::error::Error;

use serde::{Deserialize, Serialize};

/// A random per-installation identifier.
///
/// Random, and derived from nothing. Not a MAC address, not a hardware
/// serial, not anything about the user: those are privacy-hostile, unstable
/// under virtual machines and adapter churn, and would turn seat accounting
/// into tracking. Seat counting needs a value that is stable and unique per
/// installation, which is all this is.
///
/// Generating it belongs to the host; this crate is pure.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct MachineId(String);

impl From<MachineId> for String {
    fn from(value: MachineId) -> Self {
        value.0
    }
}

impl MachineId {
    /// Shortest value accepted, in bytes.
    ///
    /// Enough that a host cannot accidentally supply something guessable or
    /// enumerable, such as a counter or a hostname.
    pub const MINIMUM_BYTES: usize = 16;

    /// Validates and records an identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, MachineIdError> {
        let value = value.into();
        if value.len() < Self::MINIMUM_BYTES {
            return Err(MachineIdError::TooShort {
                minimum: Self::MINIMUM_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for MachineId {
    type Error = MachineIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for MachineId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Machine identifier validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineIdError {
    /// The identifier was too short to be unguessable.
    TooShort {
        /// Shortest accepted.
        minimum: usize,
        /// Supplied length.
        actual: usize,
    },
}

impl fmt::Display for MachineIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { minimum, actual } => write!(
                formatter,
                "machine id is {actual} bytes; at least {minimum} are needed"
            ),
        }
    }
}

impl Error for MachineIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_id_must_be_long_enough_to_be_unguessable() {
        assert!(matches!(
            MachineId::new("short"),
            Err(MachineIdError::TooShort { actual: 5, .. })
        ));
        assert!(MachineId::new("0123456789abcdef").is_ok());
    }
}
