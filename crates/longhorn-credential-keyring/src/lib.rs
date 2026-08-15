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

#[cfg(any(test, target_os = "macos", target_os = "windows"))]
mod mapping {
    use super::CredentialError;

    /// What one backend call answered, restated in this crate's terms so the
    /// contract mapping compiles — and is tested — on every platform, not
    /// only where a keychain is composed.
    ///
    /// The one distinction the whole crate exists to keep: `NoEntry` is the
    /// store answering "the slot is empty"; `Unavailable` is the store not
    /// answering at all. Locked is not absent.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(super) enum BackendFailure {
        /// The store was reached and the slot has nothing in it.
        NoEntry,
        /// Locked, denied, unreachable — anything that is not "empty".
        Unavailable(String),
    }

    /// Maps one backend write to the contract: a store call has no honest
    /// "empty" answer, so every failure is `Unavailable`.
    pub(super) fn map_store(result: Result<(), String>) -> Result<(), CredentialError> {
        result.map_err(unavailable)
    }

    /// Maps one backend read to the contract.
    pub(super) fn map_retrieve(
        result: Result<String, BackendFailure>,
    ) -> Result<Option<String>, CredentialError> {
        match result {
            Ok(secret) => Ok(Some(secret)),
            // The one case that is honestly "empty": the store answered and
            // the slot has nothing in it.
            Err(BackendFailure::NoEntry) => Ok(None),
            // Everything else — locked, denied, unreachable — is
            // `Unavailable`. The credential may exist and cannot be read,
            // which must never be reported as "not activated".
            Err(BackendFailure::Unavailable(detail)) => Err(unavailable(detail)),
        }
    }

    /// Maps one backend removal to the contract.
    pub(super) fn map_remove(result: Result<(), BackendFailure>) -> Result<(), CredentialError> {
        match result {
            // Removing an empty slot succeeds, per the trait's contract.
            Ok(()) | Err(BackendFailure::NoEntry) => Ok(()),
            Err(BackendFailure::Unavailable(detail)) => Err(unavailable(detail)),
        }
    }

    /// The detail is the backend's own description and never the secret.
    fn unavailable(detail: String) -> CredentialError {
        CredentialError::Unavailable { detail }
    }
}

#[cfg(any(test, target_os = "macos", target_os = "windows"))]
use mapping::{BackendFailure, map_remove, map_retrieve, map_store};

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod platform {
    use super::{BackendFailure, CredentialError, CredentialSlot, KeyringCredentialStore};
    use super::{map_remove, map_retrieve, map_store};

    fn entry(
        store: &KeyringCredentialStore,
        slot: CredentialSlot,
    ) -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new(store.service(), slot.as_str()).map_err(|error| {
            CredentialError::Unavailable {
                detail: error.to_string(),
            }
        })
    }

    /// `NoEntry` is the store answering "empty"; everything else is the store
    /// not answering. The detail is the error's own description and never the
    /// secret — the keyring crate does not echo values into its errors, and
    /// nothing here adds one.
    fn classify(error: keyring::Error) -> BackendFailure {
        match error {
            keyring::Error::NoEntry => BackendFailure::NoEntry,
            other => BackendFailure::Unavailable(other.to_string()),
        }
    }

    impl crate::CredentialStore for KeyringCredentialStore {
        fn store(&self, slot: CredentialSlot, secret: &str) -> Result<(), CredentialError> {
            map_store(
                entry(self, slot)?
                    .set_password(secret)
                    .map_err(|error| error.to_string()),
            )
        }

        fn retrieve(&self, slot: CredentialSlot) -> Result<Option<String>, CredentialError> {
            map_retrieve(entry(self, slot)?.get_password().map_err(classify))
        }

        fn remove(&self, slot: CredentialSlot) -> Result<(), CredentialError> {
            map_remove(entry(self, slot)?.delete_credential().map_err(classify))
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

/// The contract suite against a mock backend, on every platform.
///
/// The platform-gated suite above reaches a real keychain; this one drives
/// the same mapping the platform code routes through, so CI Linux exercises
/// the contract — above all that a locked store is `Unavailable`, never
/// `None`.
#[cfg(test)]
mod mock_tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// A backend answering in `BackendFailure` terms: an in-memory slot map
    /// plus a switch that makes every call fail the way a locked or
    /// unreachable store would.
    #[derive(Default)]
    struct MockBackend {
        secrets: Mutex<BTreeMap<CredentialSlot, String>>,
        failure: Mutex<Option<String>>,
    }

    impl MockBackend {
        fn lock(&self, detail: &str) {
            *self.failure.lock().expect("failure") = Some(detail.to_owned());
        }

        fn failure(&self) -> Option<String> {
            self.failure.lock().expect("failure").clone()
        }
    }

    /// A `CredentialStore` over the mock, routing through the crate's own
    /// mapping functions exactly as the platform module does.
    struct MockStore {
        backend: MockBackend,
    }

    impl CredentialStore for MockStore {
        fn store(&self, slot: CredentialSlot, secret: &str) -> Result<(), CredentialError> {
            if let Some(detail) = self.backend.failure() {
                return map_store(Err(detail));
            }
            self.backend
                .secrets
                .lock()
                .expect("secrets")
                .insert(slot, secret.to_owned());
            map_store(Ok(()))
        }

        fn retrieve(&self, slot: CredentialSlot) -> Result<Option<String>, CredentialError> {
            if let Some(detail) = self.backend.failure() {
                return map_retrieve(Err(BackendFailure::Unavailable(detail)));
            }
            let secret = self
                .backend
                .secrets
                .lock()
                .expect("secrets")
                .get(&slot)
                .cloned();
            match secret {
                Some(secret) => map_retrieve(Ok(secret)),
                None => map_retrieve(Err(BackendFailure::NoEntry)),
            }
        }

        fn remove(&self, slot: CredentialSlot) -> Result<(), CredentialError> {
            if let Some(detail) = self.backend.failure() {
                return map_remove(Err(BackendFailure::Unavailable(detail)));
            }
            let removed = self.backend.secrets.lock().expect("secrets").remove(&slot);
            match removed {
                Some(_) => map_remove(Ok(())),
                None => map_remove(Err(BackendFailure::NoEntry)),
            }
        }
    }

    /// The whole contract against the mock: store, retrieve, replace, remove,
    /// and remove-when-empty — the same shape the platform suite asserts
    /// against a real keychain.
    #[test]
    fn the_contract_holds_against_the_mock_backend() {
        let store = MockStore {
            backend: MockBackend::default(),
        };
        let slot = CredentialSlot::RefreshToken;

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
    fn slots_do_not_alias_against_the_mock_backend() {
        let store = MockStore {
            backend: MockBackend::default(),
        };

        store
            .store(CredentialSlot::RefreshToken, "token")
            .expect("store token");
        assert_eq!(
            store
                .retrieve(CredentialSlot::LicenceKey)
                .expect("other slot"),
            None
        );
    }

    /// Locked is not absent: a store that does not answer is `Unavailable`,
    /// and no read of it may come back as "the slot is empty".
    #[test]
    fn a_locked_store_is_unavailable_never_empty() {
        let store = MockStore {
            backend: MockBackend::default(),
        };
        store
            .store(CredentialSlot::RefreshToken, "secret")
            .expect("store");

        store.backend.lock("keychain is locked");

        let error = store
            .retrieve(CredentialSlot::RefreshToken)
            .expect_err("a locked store cannot answer None");
        assert_eq!(
            error,
            CredentialError::Unavailable {
                detail: "keychain is locked".to_owned()
            }
        );
        assert!(store.store(CredentialSlot::RefreshToken, "other").is_err());
        assert!(store.remove(CredentialSlot::RefreshToken).is_err());
    }
}
