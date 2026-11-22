use std::str::FromStr;

use longhorn_config::{BackupArchiveLimits, BackupKind};
use longhorn_config_age::{
    AgeEncryptionError, AgeEnvelopeLimits, AgeEnvelopeLimitsError, AgeIdentity, AgeIdentityRing,
    AgeInspectionOutcome, AgePassphrase, AgePassphraseError, AgeProviderError, AgeRecipient,
    AgeReencryptionError, BackupEncryptionProvider, encrypt_export_to_recipients,
    encrypt_operational_backup, inspect_with_identities, inspect_with_provider,
    reencrypt_operational_backup,
};

use super::support::encoded_archive;

struct UnavailableProvider;

impl BackupEncryptionProvider for UnavailableProvider {
    fn active_recipients(&self) -> Result<Vec<AgeRecipient>, AgeProviderError> {
        Err(AgeProviderError::InteractionRequired)
    }

    fn decryption_identities(&self) -> Result<Vec<AgeIdentity>, AgeProviderError> {
        Err(AgeProviderError::Unavailable)
    }
}

#[test]
fn operational_automation_refuses_interactive_authority_and_locks_unavailable_keys() {
    let archive = encoded_archive("provider-refusal", BackupKind::Operational);
    assert_eq!(
        encrypt_operational_backup(&UnavailableProvider, &archive, AgeEnvelopeLimits::default()),
        Err(AgeEncryptionError::Provider(
            AgeProviderError::InteractionRequired
        ))
    );

    let identity = AgeIdentity::generate();
    let encrypted = longhorn_config_age::encrypt_export_to_recipients(
        &archive,
        &[identity.recipient()],
        AgeEnvelopeLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        inspect_with_provider(
            &UnavailableProvider,
            encrypted.bytes(),
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Locked(_)
    ));
}

#[test]
fn wrong_key_tampering_truncation_and_future_format_have_distinct_states() {
    let archive = encoded_archive("failure-states", BackupKind::Operational);
    let identity = AgeIdentity::generate();
    let encrypted = longhorn_config_age::encrypt_export_to_recipients(
        &archive,
        &[identity.recipient()],
        AgeEnvelopeLimits::default(),
    )
    .unwrap();

    let wrong = AgeIdentity::generate();
    assert!(matches!(
        inspect_with_identities(&[wrong], encrypted.bytes(), AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Locked(_)
    ));

    let truncated = &encrypted.bytes()[..encrypted.bytes().len() - 8];
    assert!(matches!(
        inspect_with_identities(
            std::slice::from_ref(&identity),
            truncated,
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Corrupt(_)
    ));

    let mut modified = encrypted.bytes().to_vec();
    *modified.last_mut().unwrap() ^= 1;
    assert!(matches!(
        inspect_with_identities(
            std::slice::from_ref(&identity),
            &modified,
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Corrupt(_)
    ));

    assert!(matches!(
        inspect_with_identities(
            std::slice::from_ref(&identity),
            b"age-encryption.org/v2\n",
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Unsupported(_)
    ));
}

#[test]
fn authenticated_non_archive_never_reaches_inner_trust() {
    let identity = AgeIdentity::generate();
    let raw_recipient =
        age::x25519::Recipient::from_str(&identity.recipient().as_string()).unwrap();
    let raw = age::encrypt(&raw_recipient, b"not a ZIP archive").unwrap();
    assert!(matches!(
        inspect_with_identities(&[identity], &raw, AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::InnerArchiveRejected { .. }
    ));
}

#[test]
fn configured_ciphertext_bounds_fail_closed() {
    let archive = encoded_archive("ciphertext-bounds", BackupKind::Operational);
    let identity = AgeIdentity::generate();
    let tight = AgeEnvelopeLimits::new(1, BackupArchiveLimits::default()).unwrap();
    assert_eq!(
        encrypt_export_to_recipients(&archive, &[identity.recipient()], tight),
        Err(AgeEncryptionError::CiphertextTooLarge { limit: 1 })
    );

    // Encrypt under default limits, then inspect under a ciphertext bound smaller than the envelope.
    let encrypted = encrypt_export_to_recipients(
        &archive,
        &[identity.recipient()],
        AgeEnvelopeLimits::default(),
    )
    .unwrap();
    let too_small_ciphertext =
        AgeEnvelopeLimits::new(encrypted.bytes().len() - 1, BackupArchiveLimits::default())
            .unwrap();
    assert!(matches!(
        inspect_with_identities(
            std::slice::from_ref(&identity),
            encrypted.bytes(),
            too_small_ciphertext
        ),
        AgeInspectionOutcome::Unsupported(_)
    ));

    // Inner archive bound enforced on both encryption (envelope.rs:96) and decryption (inspection.rs:146).
    let tiny_inner = BackupArchiveLimits::new(16, 8, 256, 16, 16, 200).unwrap();
    let tight_inner = AgeEnvelopeLimits::new(1024 * 1024, tiny_inner).unwrap();
    assert!(matches!(
        encrypt_export_to_recipients(&archive, &[identity.recipient()], tight_inner),
        Err(AgeEncryptionError::InnerArchiveTooLarge { limit: 16, .. })
    ));
    assert!(matches!(
        inspect_with_identities(&[identity], encrypted.bytes(), tight_inner),
        AgeInspectionOutcome::Unsupported(_)
    ));
}

#[test]
fn envelope_limits_and_passphrase_validate_bounds() {
    assert_eq!(
        AgeEnvelopeLimits::new(0, BackupArchiveLimits::default()),
        Err(AgeEnvelopeLimitsError::Zero)
    );
    assert_eq!(
        AgeEnvelopeLimits::new(544 * 1024 * 1024 + 1, BackupArchiveLimits::default()),
        Err(AgeEnvelopeLimitsError::HardCeiling)
    );
    let valid = AgeEnvelopeLimits::new(1024, BackupArchiveLimits::default()).unwrap();
    assert_eq!(valid.max_ciphertext_bytes(), 1024);

    assert_eq!(
        AgePassphrase::new(String::new()).unwrap_err(),
        AgePassphraseError::Empty
    );
    assert_eq!(
        AgePassphrase::new("x".repeat(4097)).unwrap_err(),
        AgePassphraseError::TooLong
    );
    let passphrase = AgePassphrase::new("correct-horse-battery-staple".into()).unwrap();
    assert!(!format!("{passphrase:?}").contains("correct-horse-battery-staple"));
}

struct FailingProvider(AgeProviderError);

impl BackupEncryptionProvider for FailingProvider {
    fn active_recipients(&self) -> Result<Vec<AgeRecipient>, AgeProviderError> {
        Err(self.0)
    }

    fn decryption_identities(&self) -> Result<Vec<AgeIdentity>, AgeProviderError> {
        Err(self.0)
    }
}

#[test]
fn provider_failed_stays_distinct_from_locked() {
    let archive = encoded_archive("provider-failed", BackupKind::Operational);
    let identity = AgeIdentity::generate();
    let encrypted = encrypt_export_to_recipients(
        &archive,
        &[identity.recipient()],
        AgeEnvelopeLimits::default(),
    )
    .unwrap();
    let target = AgeIdentityRing::new(AgeIdentity::generate());

    for locked_error in [
        AgeProviderError::Unavailable,
        AgeProviderError::InteractionRequired,
    ] {
        let provider = FailingProvider(locked_error);
        assert!(matches!(
            inspect_with_provider(&provider, encrypted.bytes(), AgeEnvelopeLimits::default()),
            AgeInspectionOutcome::Locked(_)
        ));
        assert!(matches!(
            reencrypt_operational_backup(
                &provider,
                &target,
                encrypted.bytes(),
                AgeEnvelopeLimits::default()
            ),
            Err(AgeReencryptionError::SourceLocked(_))
        ));
    }

    let failed = FailingProvider(AgeProviderError::Failed);
    assert!(matches!(
        inspect_with_provider(&failed, encrypted.bytes(), AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::ProviderFailed(_)
    ));
    assert!(matches!(
        reencrypt_operational_backup(
            &failed,
            &target,
            encrypted.bytes(),
            AgeEnvelopeLimits::default()
        ),
        Err(AgeReencryptionError::SourceProviderFailed(_))
    ));
}
