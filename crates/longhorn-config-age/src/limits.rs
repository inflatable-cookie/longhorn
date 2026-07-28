use std::{error::Error, fmt};

use longhorn_config::BackupArchiveLimits;

const DEFAULT_MAX_CIPHERTEXT_BYTES: usize = 320 * 1024 * 1024;
const HARD_MAX_CIPHERTEXT_BYTES: usize = 544 * 1024 * 1024;

/// Finite ciphertext and verified inner-archive bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgeEnvelopeLimits {
    max_ciphertext_bytes: usize,
    archive_limits: BackupArchiveLimits,
}

impl AgeEnvelopeLimits {
    /// Constructs limits below the adapter hard ceiling.
    pub fn new(
        max_ciphertext_bytes: usize,
        archive_limits: BackupArchiveLimits,
    ) -> Result<Self, AgeEnvelopeLimitsError> {
        if max_ciphertext_bytes == 0 {
            Err(AgeEnvelopeLimitsError::Zero)
        } else if max_ciphertext_bytes > HARD_MAX_CIPHERTEXT_BYTES {
            Err(AgeEnvelopeLimitsError::HardCeiling)
        } else {
            Ok(Self {
                max_ciphertext_bytes,
                archive_limits,
            })
        }
    }

    /// Returns the complete ciphertext byte bound.
    #[must_use]
    pub const fn max_ciphertext_bytes(self) -> usize {
        self.max_ciphertext_bytes
    }

    /// Returns strict inner ZIP inspection bounds.
    #[must_use]
    pub const fn archive_limits(self) -> BackupArchiveLimits {
        self.archive_limits
    }
}

impl Default for AgeEnvelopeLimits {
    fn default() -> Self {
        Self {
            max_ciphertext_bytes: DEFAULT_MAX_CIPHERTEXT_BYTES,
            archive_limits: BackupArchiveLimits::default(),
        }
    }
}

/// Invalid age-envelope safety bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgeEnvelopeLimitsError {
    /// Ciphertext bound was zero.
    Zero,
    /// Ciphertext bound exceeded the hard ceiling.
    HardCeiling,
}

impl fmt::Display for AgeEnvelopeLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("age ciphertext limit must be non-zero"),
            Self::HardCeiling => formatter.write_str("age ciphertext limit exceeds hard ceiling"),
        }
    }
}

impl Error for AgeEnvelopeLimitsError {}
