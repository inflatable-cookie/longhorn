//! Optional binary age v1 envelopes for Longhorn configuration backups.
//!
//! Plaintext capture, ZIP encoding, restore, and retention remain in
//! `longhorn-config`. This crate owns only the replaceable authenticated
//! envelope and injected key-authority seam.

mod authority;
mod bounded;
mod envelope;
mod error;
mod inspection;
mod limits;
mod rotation;
mod types;

pub use authority::{
    AgeIdentity, AgeIdentityError, AgeIdentityRing, AgePassphrase, AgePassphraseError,
    AgeProviderError, AgeRecipient, AgeRecipientError, BackupEncryptionProvider,
};
pub use envelope::{
    encrypt_export_to_recipients, encrypt_export_with_passphrase, encrypt_operational_backup,
};
pub use error::{AgeEncryptionError, AgeReencryptionError};
pub use inspection::{inspect_with_identities, inspect_with_passphrase, inspect_with_provider};
pub use limits::{AgeEnvelopeLimits, AgeEnvelopeLimitsError};
pub use rotation::reencrypt_operational_backup;
pub use types::{
    AgeBackupInspection, AgeEncryptionMode, AgeEnvelopeEvidence, AgeEnvelopeReceipt,
    AgeInspectionOutcome, EncryptedBackupArchive,
};

/// Stable authenticated-envelope format identifier.
pub const AGE_V1_FORMAT_ID: &str = "age-encryption.org/v1";
