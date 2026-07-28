use longhorn_config::BackupKind;
use longhorn_config_age::{
    AgeEncryptionError, AgeEnvelopeLimits, AgeIdentity, AgeIdentityRing, AgeInspectionOutcome,
    AgeProviderError, AgeRecipient, AgeReencryptionError, BackupEncryptionProvider,
    encrypt_operational_backup, inspect_with_provider, reencrypt_operational_backup,
};

use super::support::encoded_archive;

struct FailingTarget;

impl BackupEncryptionProvider for FailingTarget {
    fn active_recipients(&self) -> Result<Vec<AgeRecipient>, AgeProviderError> {
        Err(AgeProviderError::InteractionRequired)
    }

    fn decryption_identities(&self) -> Result<Vec<AgeIdentity>, AgeProviderError> {
        Err(AgeProviderError::Unavailable)
    }
}

#[test]
fn active_and_historical_ring_reads_old_archives_and_writes_new_key() {
    let archive = encoded_archive("rotation-ring", BackupKind::Operational);
    let old_identity = AgeIdentity::generate();
    let old_ring = AgeIdentityRing::new(old_identity.clone());
    let old_envelope =
        encrypt_operational_backup(&old_ring, &archive, AgeEnvelopeLimits::default()).unwrap();

    let new_identity = AgeIdentity::generate();
    let active_only = AgeIdentityRing::new(new_identity.clone());
    assert!(matches!(
        inspect_with_provider(
            &active_only,
            old_envelope.bytes(),
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Locked(_)
    ));

    let rotated_ring = AgeIdentityRing::new(new_identity).with_historical(old_identity);
    assert!(matches!(
        inspect_with_provider(
            &rotated_ring,
            old_envelope.bytes(),
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Verified(_)
    ));
    let new_envelope =
        encrypt_operational_backup(&rotated_ring, &archive, AgeEnvelopeLimits::default()).unwrap();
    assert!(matches!(
        inspect_with_provider(
            &active_only,
            new_envelope.bytes(),
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Verified(_)
    ));
}

#[test]
fn explicit_reencryption_uses_fresh_envelope_and_cannot_modify_source_on_failure() {
    let archive = encoded_archive("explicit-reencryption", BackupKind::Operational);
    let old_ring = AgeIdentityRing::new(AgeIdentity::generate());
    let source =
        encrypt_operational_backup(&old_ring, &archive, AgeEnvelopeLimits::default()).unwrap();
    let original = source.bytes().to_vec();
    let new_ring = AgeIdentityRing::new(AgeIdentity::generate());

    let replacement = reencrypt_operational_backup(
        &old_ring,
        &new_ring,
        source.bytes(),
        AgeEnvelopeLimits::default(),
    )
    .unwrap();
    assert_ne!(replacement.bytes(), source.bytes());
    assert_eq!(
        replacement.receipt().inner_archive_sha256(),
        source.receipt().inner_archive_sha256()
    );
    assert!(matches!(
        inspect_with_provider(&new_ring, replacement.bytes(), AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Verified(_)
    ));

    assert_eq!(
        reencrypt_operational_backup(
            &old_ring,
            &FailingTarget,
            source.bytes(),
            AgeEnvelopeLimits::default()
        ),
        Err(AgeReencryptionError::Target(AgeEncryptionError::Provider(
            AgeProviderError::InteractionRequired
        )))
    );
    assert_eq!(source.bytes(), original);
}
