use std::io;

use age::{DecryptError, Decryptor};
use longhorn_config::inspect_backup_archive;

use crate::{
    AgeBackupInspection, AgeEncryptionMode, AgeEnvelopeEvidence, AgeEnvelopeLimits,
    AgeEnvelopeReceipt, AgeIdentity, AgeInspectionOutcome, AgePassphrase, BackupEncryptionProvider,
    bounded::BoundedWriter,
};

pub(crate) struct DecryptedAgeArchive {
    pub(crate) bytes: Vec<u8>,
    pub(crate) mode: AgeEncryptionMode,
    pub(crate) evidence: AgeEnvelopeEvidence,
}

pub(crate) enum DecryptionFailure {
    Locked(AgeEnvelopeEvidence),
    Corrupt(AgeEnvelopeEvidence),
    Unsupported(AgeEnvelopeEvidence),
}

/// Inspects through the provider's active and historical identity ring.
#[must_use]
pub fn inspect_with_provider(
    provider: &impl BackupEncryptionProvider,
    ciphertext: &[u8],
    limits: AgeEnvelopeLimits,
) -> AgeInspectionOutcome {
    let identities = match provider.decryption_identities() {
        Ok(identities) => identities,
        Err(_) => return AgeInspectionOutcome::Locked(AgeEnvelopeEvidence::new(ciphertext)),
    };
    inspect_with_identities(&identities, ciphertext, limits)
}

/// Inspects with explicit X25519 identities.
#[must_use]
pub fn inspect_with_identities(
    identities: &[AgeIdentity],
    ciphertext: &[u8],
    limits: AgeEnvelopeLimits,
) -> AgeInspectionOutcome {
    match decrypt_with_identities(identities, ciphertext, limits) {
        Ok(decrypted) => inspect_decrypted(decrypted, limits),
        Err(failure) => failure.into_outcome(),
    }
}

/// Inspects an explicit passphrase-protected export.
#[must_use]
pub fn inspect_with_passphrase(
    passphrase: &AgePassphrase,
    ciphertext: &[u8],
    limits: AgeEnvelopeLimits,
) -> AgeInspectionOutcome {
    match decrypt_with_passphrase(passphrase, ciphertext, limits) {
        Ok(decrypted) => inspect_decrypted(decrypted, limits),
        Err(failure) => failure.into_outcome(),
    }
}

pub(crate) fn decrypt_with_provider(
    provider: &impl BackupEncryptionProvider,
    ciphertext: &[u8],
    limits: AgeEnvelopeLimits,
) -> Result<DecryptedAgeArchive, DecryptionFailure> {
    let identities = provider
        .decryption_identities()
        .map_err(|_| DecryptionFailure::Locked(AgeEnvelopeEvidence::new(ciphertext)))?;
    decrypt_with_identities(&identities, ciphertext, limits)
}

fn decrypt_with_identities(
    identities: &[AgeIdentity],
    ciphertext: &[u8],
    limits: AgeEnvelopeLimits,
) -> Result<DecryptedAgeArchive, DecryptionFailure> {
    let evidence = validate_ciphertext_size(ciphertext, limits)?;
    if identities.is_empty() {
        return Err(DecryptionFailure::Locked(evidence));
    }
    let decryptor = parse_decryptor(ciphertext, &evidence)?;
    let mode = mode(&decryptor);
    let identity_refs = identities
        .iter()
        .map(|identity| &identity.0 as &dyn age::Identity);
    let reader = decryptor
        .decrypt(identity_refs)
        .map_err(|error| classify_decrypt_error(error, evidence.clone()))?;
    read_plaintext(reader, mode, evidence, limits)
}

fn decrypt_with_passphrase(
    passphrase: &AgePassphrase,
    ciphertext: &[u8],
    limits: AgeEnvelopeLimits,
) -> Result<DecryptedAgeArchive, DecryptionFailure> {
    let evidence = validate_ciphertext_size(ciphertext, limits)?;
    let decryptor = parse_decryptor(ciphertext, &evidence)?;
    let mode = mode(&decryptor);
    let identity = age::scrypt::Identity::new(passphrase.0.clone());
    let reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|error| classify_decrypt_error(error, evidence.clone()))?;
    read_plaintext(reader, mode, evidence, limits)
}

fn validate_ciphertext_size(
    ciphertext: &[u8],
    limits: AgeEnvelopeLimits,
) -> Result<AgeEnvelopeEvidence, DecryptionFailure> {
    let evidence = AgeEnvelopeEvidence::new(ciphertext);
    if ciphertext.len() > limits.max_ciphertext_bytes() {
        Err(DecryptionFailure::Unsupported(evidence))
    } else {
        Ok(evidence)
    }
}

fn parse_decryptor<'a>(
    ciphertext: &'a [u8],
    evidence: &AgeEnvelopeEvidence,
) -> Result<Decryptor<&'a [u8]>, DecryptionFailure> {
    Decryptor::new_buffered(ciphertext)
        .map_err(|error| classify_parse_error(error, evidence.clone()))
}

fn mode(decryptor: &Decryptor<&[u8]>) -> AgeEncryptionMode {
    if decryptor.is_scrypt() {
        AgeEncryptionMode::Passphrase
    } else {
        AgeEncryptionMode::RecipientKeys
    }
}

fn read_plaintext(
    mut reader: impl io::Read,
    mode: AgeEncryptionMode,
    evidence: AgeEnvelopeEvidence,
    limits: AgeEnvelopeLimits,
) -> Result<DecryptedAgeArchive, DecryptionFailure> {
    let mut output = BoundedWriter::new(limits.archive_limits().max_archive_bytes());
    let result = io::copy(&mut reader, &mut output);
    if output.exceeded() {
        return Err(DecryptionFailure::Unsupported(evidence));
    }
    result.map_err(|_| DecryptionFailure::Corrupt(evidence.clone()))?;
    Ok(DecryptedAgeArchive {
        bytes: output.into_bytes(),
        mode,
        evidence,
    })
}

fn inspect_decrypted(
    decrypted: DecryptedAgeArchive,
    limits: AgeEnvelopeLimits,
) -> AgeInspectionOutcome {
    match inspect_backup_archive(&decrypted.bytes, limits.archive_limits()) {
        Ok(archive) => {
            let receipt = AgeEnvelopeReceipt::new(
                decrypted.evidence,
                decrypted.mode,
                archive.archive_sha256().clone(),
            );
            AgeInspectionOutcome::Verified(Box::new(AgeBackupInspection::new(archive, receipt)))
        }
        Err(error) => AgeInspectionOutcome::InnerArchiveRejected {
            evidence: decrypted.evidence,
            error,
        },
    }
}

fn classify_parse_error(error: DecryptError, evidence: AgeEnvelopeEvidence) -> DecryptionFailure {
    match error {
        DecryptError::UnknownFormat => DecryptionFailure::Unsupported(evidence),
        _ => DecryptionFailure::Corrupt(evidence),
    }
}

/// age's error surface deliberately cannot say "tampered". A header whose
/// stanza no longer authenticates unwraps no file key, which arrives as
/// `NoMatchingKeys` — the same error a wrong key gives — and a tampered
/// payload surfaces later as a stream io error, which `read_plaintext`
/// already reports as `Corrupt`. So `Locked` here means "this key did not
/// open this file", and the operator-facing copy must not claim to know why.
fn classify_decrypt_error(error: DecryptError, evidence: AgeEnvelopeEvidence) -> DecryptionFailure {
    match error {
        DecryptError::NoMatchingKeys
        | DecryptError::DecryptionFailed
        | DecryptError::KeyDecryptionFailed => DecryptionFailure::Locked(evidence),
        DecryptError::UnknownFormat | DecryptError::ExcessiveWork { .. } => {
            DecryptionFailure::Unsupported(evidence)
        }
        _ => DecryptionFailure::Corrupt(evidence),
    }
}

impl DecryptionFailure {
    fn into_outcome(self) -> AgeInspectionOutcome {
        match self {
            Self::Locked(evidence) => AgeInspectionOutcome::Locked(evidence),
            Self::Corrupt(evidence) => AgeInspectionOutcome::Corrupt(evidence),
            Self::Unsupported(evidence) => AgeInspectionOutcome::Unsupported(evidence),
        }
    }
}
