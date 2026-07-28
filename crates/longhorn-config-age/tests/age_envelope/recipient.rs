use std::str::FromStr;

use age::secrecy::ExposeSecret;
use longhorn_config::BackupKind;
use longhorn_config_age::{
    AgeEncryptionMode, AgeEnvelopeLimits, AgeIdentity, AgeIdentityRing, AgeInspectionOutcome,
    encrypt_export_to_recipients, inspect_with_identities,
};

use super::support::{APP_ID, DOMAIN_ID, encoded_archive};

#[test]
fn recipient_envelope_interoperates_and_hides_the_complete_inner_archive() {
    let archive = encoded_archive("recipient-round-trip", BackupKind::UserExport);
    let identity = AgeIdentity::generate();
    let recipient = identity.recipient();
    let encrypted = encrypt_export_to_recipients(
        &archive,
        std::slice::from_ref(&recipient),
        AgeEnvelopeLimits::default(),
    )
    .unwrap();

    assert!(encrypted.bytes().starts_with(b"age-encryption.org/v1\n"));
    for hidden in [
        b"longhorn/manifest.json".as_slice(),
        APP_ID.as_bytes(),
        DOMAIN_ID.as_bytes(),
    ] {
        assert!(
            !encrypted
                .bytes()
                .windows(hidden.len())
                .any(|part| part == hidden)
        );
    }

    let raw_identity =
        age::x25519::Identity::from_str(identity.to_secret().expose_secret()).unwrap();
    assert_eq!(
        age::decrypt(&raw_identity, encrypted.bytes()).unwrap(),
        archive.bytes()
    );

    let raw_recipient = age::x25519::Recipient::from_str(&recipient.as_string()).unwrap();
    let raw_ciphertext = age::encrypt(&raw_recipient, archive.bytes()).unwrap();
    let outcome = inspect_with_identities(
        std::slice::from_ref(&identity),
        &raw_ciphertext,
        AgeEnvelopeLimits::default(),
    );
    let AgeInspectionOutcome::Verified(inspection) = outcome else {
        panic!("raw age ciphertext did not verify");
    };
    assert_eq!(inspection.archive().manifest().application().id(), APP_ID);
    assert_eq!(
        inspection.receipt().mode(),
        AgeEncryptionMode::RecipientKeys
    );
    assert_eq!(encrypted.receipt().inner_archive_sha256(), archive.sha256());

    let secret = identity.to_secret();
    for evidence in [
        format!("{identity:?}"),
        format!("{:?}", AgeIdentityRing::new(identity.clone())),
        serde_json::to_string(encrypted.receipt()).unwrap(),
    ] {
        assert!(!evidence.contains(secret.expose_secret()));
    }
}
