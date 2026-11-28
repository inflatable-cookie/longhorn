use std::fmt;

use longhorn_config::{
    BackupArchiveInspection, BackupArchiveLimits, BackupEnvelopeError, BackupEnvelopeReader,
};

use crate::{
    AgeEnvelopeLimits, AgeInspectionOutcome, BackupEncryptionProvider, inspect_with_provider,
};

/// Authenticates encrypted operational archives through the active and historical identity ring.
pub struct AgeBackupEnvelopeReader<'a, P> {
    provider: &'a P,
    limits: AgeEnvelopeLimits,
}

impl<P> fmt::Debug for AgeBackupEnvelopeReader<'_, P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgeBackupEnvelopeReader")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl<'a, P: BackupEncryptionProvider> AgeBackupEnvelopeReader<'a, P> {
    /// Binds the injected provider and finite envelope limits.
    #[must_use]
    pub const fn new(provider: &'a P, limits: AgeEnvelopeLimits) -> Self {
        Self { provider, limits }
    }
}

impl<P: BackupEncryptionProvider> BackupEnvelopeReader for AgeBackupEnvelopeReader<'_, P> {
    fn max_ciphertext_bytes(&self) -> usize {
        self.limits.max_ciphertext_bytes()
    }

    fn archive_limits(&self) -> BackupArchiveLimits {
        self.limits.archive_limits()
    }

    fn inspect(&self, ciphertext: &[u8]) -> Result<BackupArchiveInspection, BackupEnvelopeError> {
        match inspect_with_provider(self.provider, ciphertext, self.limits) {
            AgeInspectionOutcome::Verified(inspection) => Ok(inspection.into_archive()),
            AgeInspectionOutcome::Locked(_) => Err(BackupEnvelopeError::Locked),
            AgeInspectionOutcome::ProviderFailed(_) => Err(BackupEnvelopeError::ProviderFailed),
            AgeInspectionOutcome::Corrupt(_) => Err(BackupEnvelopeError::Corrupt),
            AgeInspectionOutcome::Unsupported(_) => Err(BackupEnvelopeError::Unsupported),
            AgeInspectionOutcome::InnerArchiveRejected { error, .. } => {
                Err(BackupEnvelopeError::InnerArchiveRejected(error))
            }
        }
    }
}
