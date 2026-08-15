//! Credential storage vocabulary: the rules, not the backend.
//!
//! This vocabulary lives in core because it is host plumbing, not licence
//! domain — it sat in `longhorn-licence` only because licensing needed it
//! first (moved on 2026-08-15, Card 210). Licensing, backup, and any future
//! secret-bearing system share the one store shape.

use core::fmt;
use std::{collections::BTreeMap, error::Error, sync::Mutex};

/// Which secret is being stored.
///
/// The vocabulary covers licence and backup secrets today; a new kind of
/// persisted secret gets a variant here, not a parallel store.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CredentialSlot {
    /// A long-lived token used to renew a lease.
    RefreshToken,
    /// A redemption key, kept so renewal does not re-prompt.
    LicenceKey,
    /// The operational age identity automatic encrypted backup decrypts with.
    ///
    /// Noninteractive by construction — contract 004's automatic backup
    /// cannot prompt, so the identity is generated once and kept here.
    AgeIdentity,
}

impl CredentialSlot {
    /// Returns the stable storage name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RefreshToken => "refresh-token",
            Self::LicenceKey => "licence-key",
            Self::AgeIdentity => "backup-identity",
        }
    }
}

impl fmt::Display for CredentialSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stores secrets outside ordinary configuration.
///
/// Credentials never enter the configuration store: that store is
/// world-readable on disk, is included in backups, and is copied between
/// machines during restore. A refresh token in a backup archive is a
/// credential leak with a long tail.
///
/// The trait is injected rather than bound to one keychain crate because
/// consuming applications differ in what they already depend on, and forcing
/// a platform backend on all of them would contradict the framework's
/// agnostic posture. Longhorn owns the *rules*; the backend is composed.
pub trait CredentialStore {
    /// Stores a secret, replacing any existing value in the slot.
    fn store(&self, slot: CredentialSlot, secret: &str) -> Result<(), CredentialError>;

    /// Retrieves a secret, or `None` when the slot is empty.
    fn retrieve(&self, slot: CredentialSlot) -> Result<Option<String>, CredentialError>;

    /// Removes a secret. Removing an empty slot succeeds.
    fn remove(&self, slot: CredentialSlot) -> Result<(), CredentialError>;
}

/// Credential storage failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialError {
    /// The backing store could not be reached.
    Unavailable {
        /// What went wrong, for diagnostics. Never the secret.
        detail: String,
    },
}

impl fmt::Display for CredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { detail } => {
                write!(formatter, "credential store unavailable: {detail}")
            }
        }
    }
}

impl Error for CredentialError {}

/// A credential store held in memory for the life of the process.
///
/// For tests, and for a deliberate no-persistence composition where
/// re-authenticating on each launch is preferred to storing anything.
#[derive(Debug, Default)]
pub struct MemoryCredentialStore {
    secrets: Mutex<BTreeMap<CredentialSlot, String>>,
}

impl MemoryCredentialStore {
    /// Records an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CredentialStore for MemoryCredentialStore {
    fn store(&self, slot: CredentialSlot, secret: &str) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(slot, secret.to_owned());
        Ok(())
    }

    fn retrieve(&self, slot: CredentialSlot) -> Result<Option<String>, CredentialError> {
        Ok(self
            .secrets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&slot)
            .cloned())
    }

    fn remove(&self, slot: CredentialSlot) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&slot);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_round_trip_and_removal_is_idempotent() {
        let store = MemoryCredentialStore::new();

        store.store(CredentialSlot::RefreshToken, "secret").unwrap();
        assert_eq!(
            store.retrieve(CredentialSlot::RefreshToken).unwrap(),
            Some("secret".to_owned())
        );

        store.remove(CredentialSlot::RefreshToken).unwrap();
        assert_eq!(store.retrieve(CredentialSlot::RefreshToken).unwrap(), None);
        store
            .remove(CredentialSlot::RefreshToken)
            .expect("removing an empty slot must succeed");
    }

    #[test]
    fn slots_do_not_collide() {
        let store = MemoryCredentialStore::new();

        store.store(CredentialSlot::RefreshToken, "token").unwrap();
        store.store(CredentialSlot::LicenceKey, "key").unwrap();
        store
            .store(CredentialSlot::AgeIdentity, "identity")
            .unwrap();

        assert_eq!(
            store.retrieve(CredentialSlot::RefreshToken).unwrap(),
            Some("token".to_owned())
        );
        assert_eq!(
            store.retrieve(CredentialSlot::LicenceKey).unwrap(),
            Some("key".to_owned())
        );
        assert_eq!(
            store.retrieve(CredentialSlot::AgeIdentity).unwrap(),
            Some("identity".to_owned())
        );
    }

    #[test]
    fn a_storage_failure_never_carries_the_secret() {
        // The detail reaches diagnostics and logs. A store that put the
        // secret in its own error message would defeat the point of having
        // a credential store at all.
        let error = CredentialError::Unavailable {
            detail: "keychain locked".into(),
        };

        assert_eq!(
            error.to_string(),
            "credential store unavailable: keychain locked"
        );
    }
}
