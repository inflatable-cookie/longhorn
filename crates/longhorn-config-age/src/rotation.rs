use longhorn_config::inspect_backup_archive;

use crate::{
    AgeEnvelopeLimits, AgeReencryptionError, BackupEncryptionProvider, EncryptedBackupArchive,
    envelope::encrypt_bytes_to_recipients,
    inspection::{DecryptionFailure, decrypt_with_provider},
};

/// Authenticates, verifies, and writes a fresh envelope to the target authority.
///
/// The source is borrowed and never modified. No output exists when target
/// encryption fails.
pub fn reencrypt_operational_backup(
    source_provider: &impl BackupEncryptionProvider,
    target_provider: &impl BackupEncryptionProvider,
    source: &[u8],
    limits: AgeEnvelopeLimits,
) -> Result<EncryptedBackupArchive, AgeReencryptionError> {
    let decrypted = decrypt_with_provider(source_provider, source, limits).map_err(map_source)?;
    let inner_sha256 = inspect_backup_archive(&decrypted.bytes, limits.archive_limits())
        .map(|inspection| inspection.archive_sha256().clone())
        .map_err(|error| AgeReencryptionError::SourceInnerArchive {
            evidence: decrypted.evidence.clone(),
            error,
        })?;
    let recipients = target_provider
        .active_recipients()
        .map_err(|error| AgeReencryptionError::Target(error.into()))?;
    encrypt_bytes_to_recipients(&decrypted.bytes, inner_sha256, &recipients, limits)
        .map_err(AgeReencryptionError::Target)
}

fn map_source(failure: DecryptionFailure) -> AgeReencryptionError {
    match failure {
        DecryptionFailure::Locked(evidence) => AgeReencryptionError::SourceLocked(evidence),
        DecryptionFailure::Corrupt(evidence) => AgeReencryptionError::SourceCorrupt(evidence),
        DecryptionFailure::Unsupported(evidence) => {
            AgeReencryptionError::SourceUnsupported(evidence)
        }
    }
}
