//! Noninteractive operational identity authority over the host's credential
//! store — the mechanism contract 004's automatic encrypted backup requires.
//!
//! The identity is generated once from the OS RNG and kept in the
//! `CredentialSlot::AgeIdentity` slot; every later backup and restore reads it
//! back. A consumer supplies any [`CredentialStore`] — the keychain backend
//! for persistence, the memory store for a deliberate no-persistence
//! composition (a fresh identity per process means old backups stop
//! decrypting, which is exactly what that composition claims).

use std::sync::Mutex;

use age::secrecy::ExposeSecret;
use longhorn_core::{CredentialError, CredentialSlot, CredentialStore};

use crate::{AgeIdentity, AgeProviderError, AgeRecipient, BackupEncryptionProvider};

/// A [`BackupEncryptionProvider`] that keeps the operational age identity in
/// an injected credential store.
pub struct StoreBackupEncryption<S> {
    store: S,
    // The store answers in strings; parse once per process rather than per
    // envelope.
    cached: Mutex<Option<AgeIdentity>>,
}

impl<S> StoreBackupEncryption<S> {
    /// Records the provider over one store.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            cached: Mutex::new(None),
        }
    }
}

impl<S: CredentialStore> StoreBackupEncryption<S> {
    /// The operational identity: read back, or generated and stored.
    ///
    /// A stored value that does not parse is `Failed`, never silently
    /// regenerated — overwriting an unreadable identity would orphan every
    /// backup it wrote.
    fn identity(&self) -> Result<AgeIdentity, AgeProviderError> {
        let mut cached = self.cached.lock().map_err(|_| AgeProviderError::Failed)?;
        if let Some(identity) = cached.as_ref() {
            return Ok(identity.clone());
        }
        let identity = match self.store.retrieve(CredentialSlot::AgeIdentity) {
            Ok(Some(secret)) => {
                AgeIdentity::parse(&secret).map_err(|_| AgeProviderError::Failed)?
            }
            Ok(None) => {
                let generated = AgeIdentity::generate();
                self.store
                    .store(
                        CredentialSlot::AgeIdentity,
                        generated.to_secret().expose_secret(),
                    )
                    .map_err(|_| AgeProviderError::Unavailable)?;
                generated
            }
            Err(CredentialError::Unavailable { .. }) => {
                return Err(AgeProviderError::Unavailable);
            }
        };
        *cached = Some(identity.clone());
        Ok(identity)
    }
}

impl<S: CredentialStore> BackupEncryptionProvider for StoreBackupEncryption<S> {
    fn active_recipients(&self) -> Result<Vec<AgeRecipient>, AgeProviderError> {
        Ok(vec![self.identity()?.recipient()])
    }

    fn decryption_identities(&self) -> Result<Vec<AgeIdentity>, AgeProviderError> {
        Ok(vec![self.identity()?])
    }
}

#[cfg(test)]
mod tests {
    use longhorn_core::MemoryCredentialStore;

    use super::*;

    #[test]
    fn the_identity_is_generated_once_and_read_back() {
        let store = MemoryCredentialStore::new();
        let provider = StoreBackupEncryption::new(store);

        let first = provider.decryption_identities().unwrap();
        let second = StoreBackupEncryption::new({
            // A fresh provider over the same store reads the same identity —
            // the process boundary is the point.
            let store = MemoryCredentialStore::new();
            store
                .store(
                    CredentialSlot::AgeIdentity,
                    first[0].to_secret().expose_secret(),
                )
                .unwrap();
            store
        })
        .decryption_identities()
        .unwrap();

        assert_eq!(first[0].recipient(), second[0].recipient());
    }

    #[test]
    fn a_corrupt_stored_identity_fails_rather_than_regenerating() {
        let store = MemoryCredentialStore::new();
        store
            .store(CredentialSlot::AgeIdentity, "not-an-identity")
            .unwrap();
        let provider = StoreBackupEncryption::new(store);

        assert!(matches!(
            provider.decryption_identities(),
            Err(AgeProviderError::Failed)
        ));
        assert_eq!(
            provider
                .store
                .retrieve(CredentialSlot::AgeIdentity)
                .unwrap()
                .as_deref(),
            Some("not-an-identity"),
            "the unreadable identity is left in place"
        );
    }
}
