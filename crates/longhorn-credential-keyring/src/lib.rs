//! The platform keychain backend for [`CredentialStore`].
//!
//! Opt-in and host-agnostic, on the precedent `longhorn-browser` set: neither
//! backend supplies this, so Longhorn implements it once and both hosts
//! compose the same crate. The trait in `longhorn-licence` stays bound to no
//! keychain crate; a consumer that prefers its own backend, or none, composes
//! that instead — `MemoryCredentialStore` is the deliberate no-persistence
//! answer.
//!
//! # The rule this crate exists to keep
//!
//! **Locked is not absent.** `retrieve` answers `Ok(None)` only when the
//! platform store was reached and the slot is empty. A keychain that is
//! locked, denied, or unreachable answers `Err(Unavailable)` — because a
//! locked keychain read as "no credential" would tell the caller the machine
//! is not activated, and every locked-screen lease renewal would become
//! re-authentication and seat churn.
//!
//! # Platforms
//!
//! macOS (Security.framework) and Windows (the credential manager). Linux is
//! deliberately not composed: the persistent backend is secret-service over
//! D-Bus, a heavy dependency nothing here ships against yet, and the cheap
//! alternative — kernel keyutils — does not survive a reboot, which silently
//! violates the persistence claim this store exists to make. On a platform
//! with no composed backend every call is `Unavailable`, which is honest:
//! pretending to store a credential that will not be there tomorrow is the
//! worst available behaviour.

use longhorn_licence::{CredentialError, CredentialSlot, CredentialStore};

/// A [`CredentialStore`] over the operating system's keychain.
///
/// The service name is **host-supplied** — the application identifier, such as
/// `com.inflatablecookie.soundcheck`. This crate hard-coding one would put
/// every consumer's secrets under a single identity, and consumer identity
/// belongs to the consumer. The keyring user is [`CredentialSlot::as_str`].
#[derive(Clone, Debug)]
pub struct KeyringCredentialStore {
    service: String,
}

impl KeyringCredentialStore {
    /// Records a store for one application's credentials.
    #[must_use]
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// The service name entries are stored under.
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod platform {
    use super::{CredentialError, CredentialSlot, KeyringCredentialStore};

    fn entry(
        store: &KeyringCredentialStore,
        slot: CredentialSlot,
    ) -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new(store.service(), slot.as_str()).map_err(unavailable)
    }

    /// Every platform failure is `Unavailable`, never an empty slot.
    ///
    /// The detail is the error's own description and never the secret — the
    /// keyring crate does not echo values into its errors, and nothing here
    /// adds one.
    fn unavailable(error: keyring::Error) -> CredentialError {
        CredentialError::Unavailable {
            detail: error.to_string(),
        }
    }

    impl crate::CredentialStore for KeyringCredentialStore {
        fn store(&self, slot: CredentialSlot, secret: &str) -> Result<(), CredentialError> {
            entry(self, slot)?.set_password(secret).map_err(unavailable)
        }

        fn retrieve(&self, slot: CredentialSlot) -> Result<Option<String>, CredentialError> {
            match entry(self, slot)?.get_password() {
                Ok(secret) => Ok(Some(secret)),
                // The one case that is honestly "empty": the store answered
                // and the slot has nothing in it.
                Err(keyring::Error::NoEntry) => Ok(None),
                // Everything else -- locked, denied, unreachable -- is
                // `Unavailable`. The credential may exist and cannot be read,
                // which must never be reported as "not activated".
                Err(error) => Err(unavailable(error)),
            }
        }

        fn remove(&self, slot: CredentialSlot) -> Result<(), CredentialError> {
            match entry(self, slot)?.delete_credential() {
                // Removing an empty slot succeeds, per the trait's contract.
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(unavailable(error)),
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl CredentialStore for KeyringCredentialStore {
    fn store(&self, _slot: CredentialSlot, _secret: &str) -> Result<(), CredentialError> {
        Err(no_backend())
    }

    fn retrieve(&self, _slot: CredentialSlot) -> Result<Option<String>, CredentialError> {
        Err(no_backend())
    }

    fn remove(&self, _slot: CredentialSlot) -> Result<(), CredentialError> {
        Err(no_backend())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn no_backend() -> CredentialError {
    CredentialError::Unavailable {
        detail: "no persistent credential backend is composed for this platform".to_owned(),
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
mod tests {
    use super::*;

    /// A service name no real application uses, so these tests can never
    /// touch a consumer's entries. One service per test rather than one
    /// shared: `cargo test` runs tests in parallel, and two tests writing the
    /// same (service, slot) entry race — the platform reports "already
    /// exists" for a concurrent create, which is a real answer about a
    /// contention this crate's callers do not have.
    fn test_service(name: &str) -> String {
        format!("audio.example.longhorn-credential-keyring.test.{name}")
    }

    /// The whole contract against the real platform store: store, retrieve,
    /// replace, remove, and remove-when-empty. One test rather than five
    /// because they share cleanup, and a failure mid-way must still remove
    /// the entry it wrote.
    #[test]
    fn the_contract_holds_against_the_real_platform_store() {
        let store = KeyringCredentialStore::new(test_service("contract"));
        let slot = CredentialSlot::RefreshToken;
        // Whatever an earlier failed run left behind.
        store.remove(slot).expect("cleanup");

        assert_eq!(store.retrieve(slot).expect("empty read"), None);
        store.store(slot, "first-secret").expect("store");
        assert_eq!(
            store.retrieve(slot).expect("read back"),
            Some("first-secret".to_owned())
        );
        store.store(slot, "replaced-secret").expect("replace");
        assert_eq!(
            store.retrieve(slot).expect("read replacement"),
            Some("replaced-secret".to_owned())
        );
        store.remove(slot).expect("remove");
        assert_eq!(store.retrieve(slot).expect("read after remove"), None);
        // Removing an empty slot succeeds, per the trait.
        store.remove(slot).expect("remove empty");
    }

    /// The slots are distinct entries, not one value with two names.
    #[test]
    fn slots_do_not_alias() {
        let store = KeyringCredentialStore::new(test_service("aliasing"));
        store.remove(CredentialSlot::RefreshToken).expect("cleanup");
        store.remove(CredentialSlot::LicenceKey).expect("cleanup");

        store
            .store(CredentialSlot::RefreshToken, "token")
            .expect("store token");
        assert_eq!(
            store
                .retrieve(CredentialSlot::LicenceKey)
                .expect("other slot"),
            None
        );

        store.remove(CredentialSlot::RefreshToken).expect("cleanup");
    }
}
