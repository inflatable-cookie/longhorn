use std::{error::Error, fmt};

use longhorn_config::BackupArchiveError;

use crate::{AgeEnvelopeEvidence, AgeProviderError};

/// Failure before a complete age envelope exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgeEncryptionError {
    /// Injected operational authority failed safely.
    Provider(AgeProviderError),
    /// Operational or recipient export supplied no public recipients.
    NoRecipients,
    /// Verified inner archive exceeds the selected inner byte bound.
    InnerArchiveTooLarge {
        /// Configured byte bound.
        limit: usize,
        /// Observed bytes.
        observed: usize,
    },
    /// Public recipient set was incompatible with age v1.
    InvalidRecipientSet,
    /// Encryption or output finalization failed.
    EncryptionFailed,
    /// Complete ciphertext exceeded the selected bound.
    CiphertextTooLarge {
        /// Configured byte bound.
        limit: usize,
    },
}

impl fmt::Display for AgeEncryptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(error) => error.fmt(formatter),
            Self::NoRecipients => formatter.write_str("age encryption requires a recipient"),
            Self::InnerArchiveTooLarge { limit, observed } => {
                write!(
                    formatter,
                    "inner archive has {observed} bytes; limit is {limit}"
                )
            }
            Self::InvalidRecipientSet => {
                formatter.write_str("age recipient set is invalid or incompatible")
            }
            Self::EncryptionFailed => formatter.write_str("age encryption failed"),
            Self::CiphertextTooLarge { limit } => {
                write!(formatter, "age ciphertext exceeds {limit} bytes")
            }
        }
    }
}

impl Error for AgeEncryptionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider(error) => Some(error),
            _ => None,
        }
    }
}

impl From<AgeProviderError> for AgeEncryptionError {
    fn from(value: AgeProviderError) -> Self {
        Self::Provider(value)
    }
}

/// Failure to authenticate a source or create the replacement envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgeReencryptionError {
    /// Source identity is unavailable or did not match.
    SourceLocked(AgeEnvelopeEvidence),
    /// Source envelope is damaged.
    SourceCorrupt(AgeEnvelopeEvidence),
    /// Source envelope or configured bound is unsupported.
    SourceUnsupported(AgeEnvelopeEvidence),
    /// Source envelope authenticated but its inner archive was invalid.
    SourceInnerArchive {
        /// Authenticated outer evidence.
        evidence: AgeEnvelopeEvidence,
        /// Strict inner archive failure.
        error: BackupArchiveError,
    },
    /// Replacement encryption failed before producing output.
    Target(AgeEncryptionError),
}

impl fmt::Display for AgeReencryptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceLocked(_) => formatter.write_str("source age envelope is locked"),
            Self::SourceCorrupt(_) => formatter.write_str("source age envelope is corrupt"),
            Self::SourceUnsupported(_) => formatter.write_str("source age envelope is unsupported"),
            Self::SourceInnerArchive { .. } => {
                formatter.write_str("source inner archive failed inspection")
            }
            Self::Target(error) => write!(formatter, "replacement age envelope failed: {error}"),
        }
    }
}

impl Error for AgeReencryptionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceInnerArchive { error, .. } => Some(error),
            Self::Target(error) => Some(error),
            _ => None,
        }
    }
}
