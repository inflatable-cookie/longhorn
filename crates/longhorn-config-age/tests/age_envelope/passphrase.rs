use age::secrecy::SecretString;
use longhorn_config::BackupKind;
use longhorn_config_age::{
    AgeEncryptionMode, AgeEnvelopeLimits, AgeInspectionOutcome, AgePassphrase,
    encrypt_export_with_passphrase, inspect_with_passphrase,
};

use super::support::encoded_archive;

const PASSPHRASE: &str = "correct horse battery staple 009 secret marker";

#[test]
fn passphrase_export_interoperates_without_serializing_secret_material() {
    let archive = encoded_archive("passphrase-round-trip", BackupKind::UserExport);
    let passphrase = AgePassphrase::new(PASSPHRASE.to_owned()).unwrap();
    let encrypted =
        encrypt_export_with_passphrase(&archive, &passphrase, AgeEnvelopeLimits::default())
            .unwrap();

    let raw_identity = age::scrypt::Identity::new(SecretString::from(PASSPHRASE.to_owned()));
    assert_eq!(
        age::decrypt(&raw_identity, encrypted.bytes()).unwrap(),
        archive.bytes()
    );

    let raw_recipient = age::scrypt::Recipient::new(SecretString::from(PASSPHRASE.to_owned()));
    let raw_ciphertext = age::encrypt(&raw_recipient, archive.bytes()).unwrap();
    let outcome =
        inspect_with_passphrase(&passphrase, &raw_ciphertext, AgeEnvelopeLimits::default());
    let AgeInspectionOutcome::Verified(inspection) = outcome else {
        panic!("raw passphrase envelope did not verify");
    };
    assert_eq!(inspection.receipt().mode(), AgeEncryptionMode::Passphrase);

    let wrong = AgePassphrase::new("wrong passphrase".to_owned()).unwrap();
    assert!(matches!(
        inspect_with_passphrase(&wrong, encrypted.bytes(), AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Locked(_)
    ));

    for evidence in [
        format!("{passphrase:?}"),
        format!("{encrypted:?}"),
        serde_json::to_string(encrypted.receipt()).unwrap(),
    ] {
        assert!(!evidence.contains(PASSPHRASE));
    }
}
