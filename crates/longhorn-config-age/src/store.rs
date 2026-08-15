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
    ///
    /// # First-run race
    ///
    /// Read-then-generate-then-store is not atomic across processes. Two
    /// instances launched together, on a machine where the slot is still
    /// empty, can both find `None` and both generate.
    ///
    /// Generation therefore reads the slot back and adopts whatever the store
    /// names, rather than keeping what it made. A process whose write was
    /// overwritten adopts the winner's identity and encrypts to that, so the
    /// two converge instead of one of them writing archives the surviving
    /// identity cannot open. The remaining window is the read-back itself:
    /// two writes landing between one process's write and its read-back still
    /// leave that process holding a superseded identity.
    ///
    /// Closing it completely needs a conditional write, and there is none to
    /// call. `keyring-core` 1.0's `CredentialApi` — the surface behind
    /// `longhorn-credential-keyring` — offers only unconditional `set_secret`;
    /// Windows' `CredWrite` has no create-only flag either, so a
    /// compare-and-swap is not available as a cross-platform primitive.
    /// Putting one on [`CredentialStore`] would declare a guarantee
    /// Longhorn's own backend cannot keep, which is worse than the narrowed
    /// window. Recorded on Card 224.
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
                // Read back rather than trusting the write. Another process
                // that generated between this one's read and its write has
                // left *its* identity in the slot; encrypting to the one
                // generated here would write archives that the identity the
                // store actually names cannot open. Adopt what the store
                // says, not what this process made.
                match self.store.retrieve(CredentialSlot::AgeIdentity) {
                    Ok(Some(secret)) => {
                        AgeIdentity::parse(&secret).map_err(|_| AgeProviderError::Failed)?
                    }
                    // Empty immediately after a write the store reported as
                    // successful: it took the secret and kept nothing, so it
                    // is not a store. Refusing beats encrypting a backup to an
                    // identity that will never be there to open it.
                    Ok(None) | Err(CredentialError::Unavailable { .. }) => {
                        return Err(AgeProviderError::Unavailable);
                    }
                }
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

    /// A store that loses the first-run race: the slot reads empty, and by the
    /// time the caller reads back, a rival process has written its own
    /// identity over the caller's. Two `retrieve` calls, two different
    /// answers — which is exactly what the losing process sees.
    struct RivalWonStore {
        rival: String,
        reads: Mutex<usize>,
        written: Mutex<Option<String>>,
    }

    impl RivalWonStore {
        fn new(rival: &str) -> Self {
            Self {
                rival: rival.to_owned(),
                reads: Mutex::new(0),
                written: Mutex::new(None),
            }
        }
    }

    impl CredentialStore for RivalWonStore {
        fn store(&self, _slot: CredentialSlot, secret: &str) -> Result<(), CredentialError> {
            *self.written.lock().unwrap() = Some(secret.to_owned());
            Ok(())
        }

        fn retrieve(&self, _slot: CredentialSlot) -> Result<Option<String>, CredentialError> {
            let mut reads = self.reads.lock().unwrap();
            *reads += 1;
            // First read: the slot is genuinely empty, so the caller
            // generates. Read-back: the rival's write landed in between.
            if *reads == 1 {
                Ok(None)
            } else {
                Ok(Some(self.rival.clone()))
            }
        }

        fn remove(&self, _slot: CredentialSlot) -> Result<(), CredentialError> {
            Ok(())
        }
    }

    /// A store that accepts a write and keeps nothing.
    struct ForgetfulStore;

    impl CredentialStore for ForgetfulStore {
        fn store(&self, _slot: CredentialSlot, _secret: &str) -> Result<(), CredentialError> {
            Ok(())
        }

        fn retrieve(&self, _slot: CredentialSlot) -> Result<Option<String>, CredentialError> {
            Ok(None)
        }

        fn remove(&self, _slot: CredentialSlot) -> Result<(), CredentialError> {
            Ok(())
        }
    }

    #[test]
    fn the_loser_of_a_first_run_race_adopts_the_surviving_identity() {
        let rival = AgeIdentity::generate();
        let provider =
            StoreBackupEncryption::new(RivalWonStore::new(rival.to_secret().expose_secret()));

        let adopted = provider.decryption_identities().unwrap();

        // The identity in use is the one the store names, not the one this
        // provider generated — otherwise every archive it writes would be
        // unreadable by the identity that survived.
        assert_eq!(adopted[0].recipient(), rival.recipient());
        assert_eq!(
            provider.active_recipients().unwrap()[0],
            rival.recipient(),
            "encryption must target the surviving identity too"
        );
        let written = provider.store.written.lock().unwrap().clone();
        assert!(
            written.is_some_and(|secret| secret != rival.to_secret().expose_secret()),
            "the provider must have generated and written its own identity first, \
             or this test is not exercising the race"
        );
    }

    #[test]
    fn a_store_that_keeps_nothing_is_refused_rather_than_encrypted_to() {
        // A successful write followed by an empty slot means the secret went
        // nowhere. Encrypting to it would produce archives nothing can open.
        let provider = StoreBackupEncryption::new(ForgetfulStore);

        assert!(matches!(
            provider.decryption_identities(),
            Err(AgeProviderError::Unavailable)
        ));
    }

    #[test]
    fn the_identity_is_generated_once_and_read_back() {
        let provider = StoreBackupEncryption::new(MemoryCredentialStore::new());

        let first = provider.decryption_identities().unwrap();

        // Generation must have *persisted*, not just returned. Seeding the
        // second store from `first` instead would pass even if `identity`
        // never wrote to the store at all.
        let persisted = provider
            .store
            .retrieve(CredentialSlot::AgeIdentity)
            .unwrap()
            .expect("generation must persist the identity, not just return it");

        // A fresh provider over that stored secret reads the same identity —
        // the process boundary is the point.
        let next = MemoryCredentialStore::new();
        next.store(CredentialSlot::AgeIdentity, &persisted).unwrap();
        let second = StoreBackupEncryption::new(next)
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
