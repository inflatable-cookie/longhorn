use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};

use crate::TransferDuration;

const HARD_MAXIMUM_ENTRIES: usize = 4_096;
const HARD_MAXIMUM_ZONES: usize = 65_536;
const HARD_MAXIMUM_INSERTION_POSITION: u32 = 1_000_000;
const HARD_MAXIMUM_LIFETIME: u64 = 86_400_000;

/// Explicit finite bounds for one process-local transfer coordinator.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct TransferLimits {
    maximum_sessions: usize,
    maximum_client_windows: usize,
    maximum_leases: usize,
    maximum_zones_per_lease: usize,
    maximum_insertion_position: u32,
    maximum_session_lifetime: TransferDuration,
    maximum_lease_lifetime: TransferDuration,
}

impl TransferLimits {
    /// Constructs limits and rejects zero or excessive values.
    pub fn new(
        maximum_sessions: usize,
        maximum_client_windows: usize,
        maximum_leases: usize,
        maximum_zones_per_lease: usize,
        maximum_insertion_position: u32,
        maximum_session_lifetime: TransferDuration,
        maximum_lease_lifetime: TransferDuration,
    ) -> Result<Self, TransferLimitsError> {
        check_usize("maximum_sessions", maximum_sessions, HARD_MAXIMUM_ENTRIES)?;
        check_usize(
            "maximum_client_windows",
            maximum_client_windows,
            HARD_MAXIMUM_ENTRIES,
        )?;
        check_usize("maximum_leases", maximum_leases, HARD_MAXIMUM_ENTRIES)?;
        check_usize(
            "maximum_zones_per_lease",
            maximum_zones_per_lease,
            HARD_MAXIMUM_ZONES,
        )?;
        check_u32(
            "maximum_insertion_position",
            maximum_insertion_position,
            HARD_MAXIMUM_INSERTION_POSITION,
        )?;
        check_u64(
            "maximum_session_lifetime",
            maximum_session_lifetime.get(),
            HARD_MAXIMUM_LIFETIME,
        )?;
        check_u64(
            "maximum_lease_lifetime",
            maximum_lease_lifetime.get(),
            HARD_MAXIMUM_LIFETIME,
        )?;
        Ok(Self {
            maximum_sessions,
            maximum_client_windows,
            maximum_leases,
            maximum_zones_per_lease,
            maximum_insertion_position,
            maximum_session_lifetime,
            maximum_lease_lifetime,
        })
    }

    /// Returns session-store capacity.
    #[must_use]
    pub const fn maximum_sessions(self) -> usize {
        self.maximum_sessions
    }

    /// Returns current client-window binding capacity.
    #[must_use]
    pub const fn maximum_client_windows(self) -> usize {
        self.maximum_client_windows
    }

    /// Returns current complete lease capacity.
    #[must_use]
    pub const fn maximum_leases(self) -> usize {
        self.maximum_leases
    }

    /// Returns the maximum zones in one complete lease.
    #[must_use]
    pub const fn maximum_zones_per_lease(self) -> usize {
        self.maximum_zones_per_lease
    }

    /// Returns the greatest accepted advisory insertion position.
    #[must_use]
    pub const fn maximum_insertion_position(self) -> u32 {
        self.maximum_insertion_position
    }

    /// Returns the longest accepted session lifetime.
    #[must_use]
    pub const fn maximum_session_lifetime(self) -> TransferDuration {
        self.maximum_session_lifetime
    }

    /// Returns the longest accepted lease lifetime.
    #[must_use]
    pub const fn maximum_lease_lifetime(self) -> TransferDuration {
        self.maximum_lease_lifetime
    }
}

impl<'de> Deserialize<'de> for TransferLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SerializedLimits {
            maximum_sessions: usize,
            maximum_client_windows: usize,
            maximum_leases: usize,
            maximum_zones_per_lease: usize,
            maximum_insertion_position: u32,
            maximum_session_lifetime: TransferDuration,
            maximum_lease_lifetime: TransferDuration,
        }

        let value = SerializedLimits::deserialize(deserializer)?;
        Self::new(
            value.maximum_sessions,
            value.maximum_client_windows,
            value.maximum_leases,
            value.maximum_zones_per_lease,
            value.maximum_insertion_position,
            value.maximum_session_lifetime,
            value.maximum_lease_lifetime,
        )
        .map_err(de::Error::custom)
    }
}

/// Invalid process-local transfer bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferLimitsError {
    /// A required bound was zero.
    Zero {
        /// Stable field name.
        field: &'static str,
    },
    /// A bound exceeded the library hard ceiling.
    ExceedsHardMaximum {
        /// Stable field name.
        field: &'static str,
        /// Library hard ceiling.
        maximum: u64,
        /// Rejected value.
        actual: u64,
    },
}

impl fmt::Display for TransferLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero { field } => write!(formatter, "{field} must be nonzero"),
            Self::ExceedsHardMaximum {
                field,
                maximum,
                actual,
            } => write!(
                formatter,
                "{field} is {actual}; library hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for TransferLimitsError {}

fn check_usize(
    field: &'static str,
    actual: usize,
    maximum: usize,
) -> Result<(), TransferLimitsError> {
    check_u64(field, actual as u64, maximum as u64)
}

fn check_u32(field: &'static str, actual: u32, maximum: u32) -> Result<(), TransferLimitsError> {
    check_u64(field, u64::from(actual), u64::from(maximum))
}

fn check_u64(field: &'static str, actual: u64, maximum: u64) -> Result<(), TransferLimitsError> {
    if actual == 0 {
        return Err(TransferLimitsError::Zero { field });
    }
    if actual > maximum {
        return Err(TransferLimitsError::ExceedsHardMaximum {
            field,
            maximum,
            actual,
        });
    }
    Ok(())
}
