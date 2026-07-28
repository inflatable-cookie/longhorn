use std::fmt;

use longhorn_config::{BackupArchiveError, BackupArchiveInspection, Sha256Digest};
use serde::Serialize;

use crate::AGE_V1_FORMAT_ID;

/// Recipient class used by one envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgeEncryptionMode {
    /// One or more public recipient keys.
    RecipientKeys,
    /// One ephemeral human passphrase.
    Passphrase,
}

/// Outer evidence available without a matching identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeEnvelopeEvidence {
    format_id: &'static str,
    ciphertext_bytes: u64,
    ciphertext_sha256: Sha256Digest,
}

impl AgeEnvelopeEvidence {
    pub(crate) fn new(ciphertext: &[u8]) -> Self {
        Self {
            format_id: AGE_V1_FORMAT_ID,
            ciphertext_bytes: ciphertext.len() as u64,
            ciphertext_sha256: Sha256Digest::from_bytes(ciphertext),
        }
    }

    /// Returns the age v1 format id.
    #[must_use]
    pub const fn format_id(&self) -> &'static str {
        self.format_id
    }

    /// Returns the complete ciphertext byte length.
    #[must_use]
    pub const fn ciphertext_bytes(&self) -> u64 {
        self.ciphertext_bytes
    }

    /// Returns SHA-256 over the complete ciphertext.
    #[must_use]
    pub fn ciphertext_sha256(&self) -> &Sha256Digest {
        &self.ciphertext_sha256
    }
}

/// Authenticated outer and verified inner archive receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeEnvelopeReceipt {
    evidence: AgeEnvelopeEvidence,
    mode: AgeEncryptionMode,
    inner_archive_sha256: Sha256Digest,
}

impl AgeEnvelopeReceipt {
    pub(crate) fn new(
        evidence: AgeEnvelopeEvidence,
        mode: AgeEncryptionMode,
        inner_archive_sha256: Sha256Digest,
    ) -> Self {
        Self {
            evidence,
            mode,
            inner_archive_sha256,
        }
    }

    /// Returns unauthenticated outer evidence.
    #[must_use]
    pub fn evidence(&self) -> &AgeEnvelopeEvidence {
        &self.evidence
    }

    /// Returns the recipient class.
    #[must_use]
    pub const fn mode(&self) -> AgeEncryptionMode {
        self.mode
    }

    /// Returns SHA-256 over the verified plaintext inner ZIP.
    #[must_use]
    pub fn inner_archive_sha256(&self) -> &Sha256Digest {
        &self.inner_archive_sha256
    }
}

/// Complete binary age v1 envelope.
#[derive(Clone, Eq, PartialEq)]
pub struct EncryptedBackupArchive {
    bytes: Vec<u8>,
    receipt: AgeEnvelopeReceipt,
}

impl EncryptedBackupArchive {
    pub(crate) fn new(bytes: Vec<u8>, receipt: AgeEnvelopeReceipt) -> Self {
        Self { bytes, receipt }
    }

    /// Returns exact binary age v1 bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns outer and inner archive evidence.
    #[must_use]
    pub fn receipt(&self) -> &AgeEnvelopeReceipt {
        &self.receipt
    }
}

impl fmt::Debug for EncryptedBackupArchive {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedBackupArchive")
            .field("receipt", &self.receipt)
            .finish_non_exhaustive()
    }
}

/// Authenticated age envelope plus strict verified inner archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgeBackupInspection {
    archive: BackupArchiveInspection,
    receipt: AgeEnvelopeReceipt,
}

impl AgeBackupInspection {
    pub(crate) fn new(archive: BackupArchiveInspection, receipt: AgeEnvelopeReceipt) -> Self {
        Self { archive, receipt }
    }

    /// Returns the same strict inspection used for plaintext restore.
    #[must_use]
    pub fn archive(&self) -> &BackupArchiveInspection {
        &self.archive
    }

    /// Returns authenticated outer and verified inner evidence.
    #[must_use]
    pub fn receipt(&self) -> &AgeEnvelopeReceipt {
        &self.receipt
    }
}

/// Side-effect-free encrypted archive inspection result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgeInspectionOutcome {
    /// Envelope authenticated and inner archive passed strict inspection.
    Verified(Box<AgeBackupInspection>),
    /// No supplied identity could authenticate the envelope.
    Locked(AgeEnvelopeEvidence),
    /// Envelope was malformed, truncated, modified, or failed authentication.
    Corrupt(AgeEnvelopeEvidence),
    /// Envelope format or configured safety bound is unsupported.
    Unsupported(AgeEnvelopeEvidence),
    /// Envelope authenticated but the inner ZIP failed strict inspection.
    InnerArchiveRejected {
        /// Authenticated outer evidence.
        evidence: AgeEnvelopeEvidence,
        /// Strict inner archive failure.
        error: BackupArchiveError,
    },
}

impl AgeInspectionOutcome {
    /// Returns outer evidence for every terminal state.
    #[must_use]
    pub fn evidence(&self) -> &AgeEnvelopeEvidence {
        match self {
            Self::Verified(inspection) => inspection.receipt().evidence(),
            Self::Locked(evidence) | Self::Corrupt(evidence) | Self::Unsupported(evidence) => {
                evidence
            }
            Self::InnerArchiveRejected { evidence, .. } => evidence,
        }
    }
}
