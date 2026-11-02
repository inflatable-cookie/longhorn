use std::{collections::BTreeSet, fs};

use longhorn_config::{
    BackupApplication, BackupArchiveLimits, BackupKind, BackupOperationalRoot,
    BackupRetentionDiagnosticKind, BackupRetentionPolicy, Sha256Digest, list_operational_backups,
    plan_backup_retention,
};
use longhorn_config_age::{
    AgeEncryptionMode, AgeEnvelopeLimits, AgeIdentity, AgeIdentityRing, AgeInspectionOutcome,
    AgePassphrase, encrypt_export_to_recipients, inspect_with_identities, inspect_with_passphrase,
    inspect_with_provider, reencrypt_operational_backup,
};
use serde_json::Value;
use tempfile::TempDir;

use super::support::{APP_ID, encoded_archive};

/// Retained ciphertext captured on the pre-repair dependency line (age
/// 0.12.1, the line Longhorn 0.4.3 shipped), frozen under
/// `fixtures/config-age-compat`. These cases are the stored-data
/// compatibility proof for the RUSTSEC-2026-0387 repair: the format
/// contract is age v1, not a crate line, so every restoration below must
/// keep working when the crate line moves.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/config-age-compat"
);
const PASSPHRASE: &str = "retained-fixture-passphrase-line-0.4.3";

fn read_fixture(name: &str) -> Vec<u8> {
    fs::read(format!("{FIXTURE_DIR}/{name}")).expect("retained fixture is present")
}

fn retained_manifest() -> Value {
    serde_json::from_slice(&read_fixture("manifest.json")).expect("retained manifest parses")
}

fn retained_identity() -> AgeIdentity {
    let secret = fs::read_to_string(format!("{FIXTURE_DIR}/recipient-identity.txt"))
        .expect("retained identity is present");
    AgeIdentity::parse(secret.trim()).expect("retained identity parses")
}

fn expected_digest(manifest: &Value, envelope: &str, field: &str) -> String {
    manifest[envelope][field]
        .as_str()
        .expect("manifest carries the digest")
        .to_owned()
}

fn assert_verified(
    outcome: AgeInspectionOutcome,
    inner_sha256: &str,
) -> Box<longhorn_config_age::AgeBackupInspection> {
    let AgeInspectionOutcome::Verified(inspection) = outcome else {
        panic!("retained envelope did not verify: {outcome:?}");
    };
    assert_eq!(
        inspection.receipt().inner_archive_sha256().as_str(),
        inner_sha256
    );
    inspection
}

#[test]
fn retained_recipient_capture_restores_under_the_current_line() {
    let manifest = retained_manifest();
    let inner = expected_digest(&manifest, "recipient", "innerArchiveSha256");
    let ciphertext = read_fixture("recipient.age");

    let inspection = assert_verified(
        inspect_with_identities(
            &[retained_identity()],
            &ciphertext,
            AgeEnvelopeLimits::default(),
        ),
        &inner,
    );
    assert_eq!(
        inspection.receipt().evidence().ciphertext_sha256().as_str(),
        expected_digest(&manifest, "recipient", "ciphertextSha256")
    );
    assert_eq!(
        inspection.receipt().mode(),
        AgeEncryptionMode::RecipientKeys
    );
    assert_eq!(
        inspection.receipt().evidence().format_id(),
        longhorn_config_age::AGE_V1_FORMAT_ID
    );
    // The restored inner archive is the same valid capture the old line
    // froze, down to its application id.
    assert_eq!(inspection.archive().manifest().application().id(), APP_ID);
}

#[test]
fn retained_passphrase_capture_restores_under_the_current_line() {
    let manifest = retained_manifest();
    let inner = expected_digest(&manifest, "passphrase", "innerArchiveSha256");
    let ciphertext = read_fixture("passphrase.age");
    let passphrase = AgePassphrase::new(PASSPHRASE.to_owned()).unwrap();

    let inspection = assert_verified(
        inspect_with_passphrase(&passphrase, &ciphertext, AgeEnvelopeLimits::default()),
        &inner,
    );
    assert_eq!(inspection.receipt().mode(), AgeEncryptionMode::Passphrase);
    assert_eq!(
        inspection.receipt().evidence().ciphertext_sha256().as_str(),
        expected_digest(&manifest, "passphrase", "ciphertextSha256")
    );
}

#[test]
fn retained_capture_keeps_old_data_in_new_key_rotation() {
    let manifest = retained_manifest();
    let inner = expected_digest(&manifest, "recipient", "innerArchiveSha256");
    let old_ring = AgeIdentityRing::new(retained_identity());
    let ciphertext = read_fixture("recipient.age");

    // The historical identity still reads the retained envelope through the
    // provider seam, which is how rotated hosts keep restoring old archives.
    assert!(matches!(
        inspect_with_provider(&old_ring, &ciphertext, AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Verified(_)
    ));

    // Rotation: decrypt the retained envelope and write a fresh envelope to a
    // new key. The inner archive digest must survive the crossing unchanged,
    // and the new ring must accept the replacement.
    let target = AgeIdentityRing::new(AgeIdentity::generate());
    let reencrypted = reencrypt_operational_backup(
        &old_ring,
        &target,
        &ciphertext,
        AgeEnvelopeLimits::default(),
    )
    .unwrap();
    assert_eq!(reencrypted.receipt().inner_archive_sha256().as_str(), inner);
    assert!(matches!(
        inspect_with_provider(&target, reencrypted.bytes(), AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Verified(_)
    ));
}

#[test]
fn retained_capture_fails_cleanly_without_the_right_key() {
    let recipient_age = read_fixture("recipient.age");
    let passphrase_age = read_fixture("passphrase.age");

    // A fresh identity is not the retained identity: Locked, never a
    // diagnostic that names key material.
    assert!(matches!(
        inspect_with_identities(
            &[AgeIdentity::generate()],
            &recipient_age,
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Locked(_)
    ));
    // The wrong passphrase on the retained passphrase envelope: Locked too.
    let wrong = AgePassphrase::new("not-the-retained-passphrase".to_owned()).unwrap();
    assert!(matches!(
        inspect_with_passphrase(&wrong, &passphrase_age, AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Locked(_)
    ));
    // The passphrase cannot open the recipient envelope.
    let passphrase_only = AgePassphrase::new(PASSPHRASE.to_owned()).unwrap();
    assert!(matches!(
        inspect_with_passphrase(
            &passphrase_only,
            &recipient_age,
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Locked(_)
    ));
}

#[test]
fn retained_capture_reports_tampering_as_corrupt() {
    let mut tampered = read_fixture("recipient.age");
    // Flip one payload byte: the envelope authenticates the header, then the
    // stream fails. That is Corrupt, never a silent partial restore.
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    assert!(matches!(
        inspect_with_identities(
            &[retained_identity()],
            &tampered,
            AgeEnvelopeLimits::default()
        ),
        AgeInspectionOutcome::Corrupt(_)
    ));
}

#[test]
fn retention_preserves_the_retained_envelope_in_storage() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("retained.longhorn-backup.age");
    fs::write(&path, read_fixture("recipient.age")).unwrap();
    let root = BackupOperationalRoot::new(temp.path()).unwrap();

    // Storage recognizes the retained envelope and refuses to prune what it
    // cannot authenticate.
    let listing = list_operational_backups(
        &root,
        &BackupApplication::new(APP_ID, "1.0.0").unwrap(),
        BackupArchiveLimits::default(),
        10,
    );
    assert!(listing.candidates().is_empty());
    assert!(listing.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind == BackupRetentionDiagnosticKind::Locked
            && diagnostic.path.as_deref() == Some(path.as_path())
    }));

    let policy = BackupRetentionPolicy::new(0, None, None, 10).unwrap();
    let plan = plan_backup_retention(
        &listing,
        policy,
        &BTreeSet::new(),
        &Sha256Digest::from_bytes(&read_fixture("recipient.age")),
    )
    .unwrap();
    assert!(plan.deletions().is_empty());
    assert!(path.exists());
}

#[test]
fn new_line_output_still_matches_the_frozen_format_identifier() {
    // Whatever line produces the ciphertext, the outer format id the product
    // documents must stay age-encryption.org/v1, and one key ring must
    // reach across lines.
    let archive = encoded_archive("format-id-after-repair", BackupKind::Operational);
    let identity = retained_identity();
    let envelope = encrypt_export_to_recipients(
        &archive,
        &[identity.recipient()],
        AgeEnvelopeLimits::default(),
    )
    .unwrap();
    assert_eq!(
        envelope.receipt().evidence().format_id(),
        longhorn_config_age::AGE_V1_FORMAT_ID
    );
    assert!(matches!(
        inspect_with_identities(&[identity], envelope.bytes(), AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Verified(_)
    ));
}
