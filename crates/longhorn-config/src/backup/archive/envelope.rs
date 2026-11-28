use std::{error::Error, fmt};

use super::{BackupArchiveError, BackupArchiveInspection, BackupArchiveLimits};

pub(super) const ENCRYPTED_EXTENSION: &str = ".age";

/// Host-provided authentication and bounded inspection of a binary age archive.
///
/// The implementation must authenticate the exact supplied ciphertext and verify
/// the decrypted ZIP with the ordinary archive inspector before returning it.
pub trait BackupEnvelopeReader {
    /// Maximum number of ciphertext bytes accepted by this reader.
    fn max_ciphertext_bytes(&self) -> usize;

    /// Strict ZIP limits shared by the plaintext and decrypted archive inspectors.
    fn archive_limits(&self) -> BackupArchiveLimits;

    /// Authenticates the ciphertext and returns its verified inner archive.
    fn inspect(&self, ciphertext: &[u8]) -> Result<BackupArchiveInspection, BackupEnvelopeError>;
}

/// Why a backup envelope could not supply verified archive evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupEnvelopeError {
    /// No available identity unlocked the archive.
    Locked,
    /// The injected identity provider failed.
    ProviderFailed,
    /// The envelope failed structural or integrity checks.
    Corrupt,
    /// The envelope format or configured limits prevent inspection.
    Unsupported,
    /// The envelope authenticated but the inner ZIP was rejected.
    InnerArchiveRejected(BackupArchiveError),
}

impl fmt::Display for BackupEnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locked => formatter.write_str("archive identity unavailable"),
            Self::ProviderFailed => formatter.write_str("archive identity provider failed"),
            Self::Corrupt => formatter.write_str("invalid backup envelope"),
            Self::Unsupported => formatter.write_str("unsupported backup envelope"),
            Self::InnerArchiveRejected(error) => error.fmt(formatter),
        }
    }
}

impl Error for BackupEnvelopeError {}
