//! Fork-history envelope format, limits, and structural migration.

use std::{convert::Infallible, error::Error, fmt};

use longhorn_core::CompatibilityStore;
use serde_json::Value;

/// Stable structural format family for fork-tree envelopes.
pub const FORK_HISTORY_FORMAT_FAMILY: &str = "longhorn.history-tree";
/// Current graph envelope version.
pub const CURRENT_FORK_HISTORY_STRUCTURAL_VERSION: u32 = 1;
/// Defensive ceiling for one encoded graph envelope.
pub const MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES: usize = 1 << 30;

/// Caller-selected bound for untrusted graph-envelope bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkPersistenceLimits {
    maximum_envelope_bytes: usize,
}

impl ForkPersistenceLimits {
    /// Validates one explicit load and encode bound.
    pub const fn new(maximum_envelope_bytes: usize) -> Result<Self, ForkPersistenceLimitsError> {
        if maximum_envelope_bytes == 0 {
            return Err(ForkPersistenceLimitsError::Zero);
        }
        if maximum_envelope_bytes > MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES {
            return Err(ForkPersistenceLimitsError::TooLarge {
                maximum: MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES,
                actual: maximum_envelope_bytes,
            });
        }
        Ok(Self {
            maximum_envelope_bytes,
        })
    }

    /// Returns the maximum accepted or produced envelope size.
    #[must_use]
    pub const fn maximum_envelope_bytes(self) -> usize {
        self.maximum_envelope_bytes
    }
}

/// Invalid graph-persistence byte bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkPersistenceLimitsError {
    /// The bound was zero.
    Zero,
    /// The bound exceeded the defensive ceiling.
    TooLarge {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied bound.
        actual: usize,
    },
}

impl fmt::Display for ForkPersistenceLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("fork persistence bound must be nonzero"),
            Self::TooLarge { maximum, actual } => write!(
                formatter,
                "fork persistence bound is {actual}; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for ForkPersistenceLimitsError {}

/// Current structural migration authority.
#[derive(Clone, Copy, Debug)]
pub struct ForkStructuralMigrationTarget {
    pub(crate) version: u32,
}

impl ForkStructuralMigrationTarget {
    /// Returns the structural family.
    #[must_use]
    pub const fn family(self) -> &'static str {
        FORK_HISTORY_FORMAT_FAMILY
    }

    /// Returns the current structural version.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }
}

/// One exact next-version structural migration step.
#[derive(Clone, Debug, PartialEq)]
pub struct ForkStructuralMigrationStep {
    pub(crate) version: u32,
    pub(crate) document: Value,
}

impl ForkStructuralMigrationStep {
    /// Constructs one structural migration step.
    #[must_use]
    pub const fn new(version: u32, document: Value) -> Self {
        Self { version, document }
    }
}

/// Registered one-step migration for older graph envelopes.
pub trait ForkStructuralMigration {
    /// Structural migration failure.
    type Error;

    /// Produces one exact next-version migration step.
    fn migrate_one(
        &self,
        from: u32,
        document: Value,
        target: ForkStructuralMigrationTarget,
    ) -> Result<Option<ForkStructuralMigrationStep>, Self::Error>;
}

/// Explicit registration with no older structural migration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoForkStructuralMigration;

impl ForkStructuralMigration for NoForkStructuralMigration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        _from: u32,
        _document: Value,
        _target: ForkStructuralMigrationTarget,
    ) -> Result<Option<ForkStructuralMigrationStep>, Self::Error> {
        Ok(None)
    }
}

