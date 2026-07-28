use std::io::{self, Write};

use age::Encryptor;
use longhorn_config::{EncodedBackupArchive, Sha256Digest};

use crate::{
    AgeEncryptionError, AgeEncryptionMode, AgeEnvelopeEvidence, AgeEnvelopeLimits,
    AgeEnvelopeReceipt, AgePassphrase, AgeRecipient, BackupEncryptionProvider,
    EncryptedBackupArchive, bounded::BoundedWriter,
};

/// Encrypts a verified archive through a noninteractive operational authority.
pub fn encrypt_operational_backup(
    provider: &impl BackupEncryptionProvider,
    archive: &EncodedBackupArchive,
    limits: AgeEnvelopeLimits,
) -> Result<EncryptedBackupArchive, AgeEncryptionError> {
    let recipients = provider.active_recipients()?;
    encrypt_bytes_to_recipients(
        archive.bytes(),
        archive.sha256().clone(),
        &recipients,
        limits,
    )
}

/// Encrypts an explicit user export to one or more public recipient keys.
pub fn encrypt_export_to_recipients(
    archive: &EncodedBackupArchive,
    recipients: &[AgeRecipient],
    limits: AgeEnvelopeLimits,
) -> Result<EncryptedBackupArchive, AgeEncryptionError> {
    encrypt_bytes_to_recipients(
        archive.bytes(),
        archive.sha256().clone(),
        recipients,
        limits,
    )
}

/// Encrypts an explicit user export with one ephemeral human passphrase.
pub fn encrypt_export_with_passphrase(
    archive: &EncodedBackupArchive,
    passphrase: &AgePassphrase,
    limits: AgeEnvelopeLimits,
) -> Result<EncryptedBackupArchive, AgeEncryptionError> {
    encrypt_bytes_with_passphrase(
        archive.bytes(),
        archive.sha256().clone(),
        passphrase,
        limits,
    )
}

pub(crate) fn encrypt_bytes_to_recipients(
    inner: &[u8],
    inner_sha256: Sha256Digest,
    recipients: &[AgeRecipient],
    limits: AgeEnvelopeLimits,
) -> Result<EncryptedBackupArchive, AgeEncryptionError> {
    validate_inner_size(inner, limits)?;
    if recipients.is_empty() {
        return Err(AgeEncryptionError::NoRecipients);
    }
    let encryptor = Encryptor::with_recipients(
        recipients
            .iter()
            .map(|recipient| &recipient.0 as &dyn age::Recipient),
    )
    .map_err(|_| AgeEncryptionError::InvalidRecipientSet)?;
    encrypt_bytes(
        inner,
        inner_sha256,
        encryptor,
        AgeEncryptionMode::RecipientKeys,
        limits,
    )
}

fn encrypt_bytes_with_passphrase(
    inner: &[u8],
    inner_sha256: Sha256Digest,
    passphrase: &AgePassphrase,
    limits: AgeEnvelopeLimits,
) -> Result<EncryptedBackupArchive, AgeEncryptionError> {
    validate_inner_size(inner, limits)?;
    encrypt_bytes(
        inner,
        inner_sha256,
        Encryptor::with_user_passphrase(passphrase.0.clone()),
        AgeEncryptionMode::Passphrase,
        limits,
    )
}

fn validate_inner_size(inner: &[u8], limits: AgeEnvelopeLimits) -> Result<(), AgeEncryptionError> {
    let limit = limits.archive_limits().max_archive_bytes();
    if inner.len() > limit {
        Err(AgeEncryptionError::InnerArchiveTooLarge {
            limit,
            observed: inner.len(),
        })
    } else {
        Ok(())
    }
}

fn encrypt_bytes(
    inner: &[u8],
    inner_sha256: Sha256Digest,
    encryptor: Encryptor,
    mode: AgeEncryptionMode,
    limits: AgeEnvelopeLimits,
) -> Result<EncryptedBackupArchive, AgeEncryptionError> {
    let mut output = BoundedWriter::new(limits.max_ciphertext_bytes());
    let result = write_envelope(encryptor, inner, &mut output);
    if output.exceeded() {
        return Err(AgeEncryptionError::CiphertextTooLarge {
            limit: limits.max_ciphertext_bytes(),
        });
    }
    result.map_err(|_| AgeEncryptionError::EncryptionFailed)?;
    let bytes = output.into_bytes();
    let receipt = AgeEnvelopeReceipt::new(AgeEnvelopeEvidence::new(&bytes), mode, inner_sha256);
    Ok(EncryptedBackupArchive::new(bytes, receipt))
}

fn write_envelope(
    encryptor: Encryptor,
    inner: &[u8],
    output: &mut BoundedWriter,
) -> io::Result<()> {
    let mut writer = encryptor.wrap_output(output)?;
    writer.write_all(inner)?;
    writer.finish()?;
    Ok(())
}
