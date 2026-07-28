use std::str::FromStr;

use longhorn_config::BackupKind;
use longhorn_config_age::{
    AgeEncryptionError, AgeEnvelopeLimits, AgeIdentity, AgeInspectionOutcome, AgeProviderError,
    AgeRecipient, BackupEncryptionProvider, encrypt_operational_backup, inspect_with_identities,
    inspect_with_provider,
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
    let tight = AgeEnvelopeLimits::new(1, longhorn_config::BackupArchiveLimits::default()).unwrap();
    assert_eq!(
        longhorn_config_age::encrypt_export_to_recipients(&archive, &[identity.recipient()], tight),
        Err(AgeEncryptionError::CiphertextTooLarge { limit: 1 })
    );
}
