//! The store-backed noninteractive authority, end to end.

use longhorn_config::BackupKind;
use longhorn_config_age::{
    AgeEnvelopeLimits, AgeInspectionOutcome, StoreBackupEncryption, encrypt_operational_backup,
    inspect_with_provider,
};
use longhorn_core::MemoryCredentialStore;

use super::support::encoded_archive;

#[test]
fn automatic_backup_never_prompts_and_the_backup_reads_back() {
    // Contract 004's noninteractive authority: no operator present, the
    // identity resolves from the store, the envelope encrypts and inspects.
    let store = MemoryCredentialStore::new();
    let provider = StoreBackupEncryption::new(store);
    let archive = encoded_archive("store-authority", BackupKind::Operational);

    let encrypted =
        encrypt_operational_backup(&provider, &archive, AgeEnvelopeLimits::default()).unwrap();

    assert!(matches!(
        inspect_with_provider(&provider, encrypted.bytes(), AgeEnvelopeLimits::default()),
        AgeInspectionOutcome::Verified(_)
    ));
}
