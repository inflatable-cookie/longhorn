use core::fmt;
use std::{collections::BTreeMap, error::Error, sync::Mutex};

use serde::{Deserialize, Serialize};

/// A random per-installation identifier.
///
/// Random, and derived from nothing. Not a MAC address, not a hardware
/// serial, not anything about the user: those are privacy-hostile, unstable
/// under virtual machines and adapter churn, and would turn seat accounting
/// into tracking. Seat counting needs a value that is stable and unique per
/// installation, which is all this is.
///
/// Generating it belongs to the host; this crate is pure.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct MachineId(String);

impl From<MachineId> for String {
    fn from(value: MachineId) -> Self {
        value.0
    }
}

impl MachineId {
    /// Shortest value accepted, in bytes.
    ///
    /// Enough that a host cannot accidentally supply something guessable or
    /// enumerable, such as a counter or a hostname.
    pub const MINIMUM_BYTES: usize = 16;

    /// Validates and records an identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, MachineIdError> {
        let value = value.into();
        if value.len() < Self::MINIMUM_BYTES {
            return Err(MachineIdError::TooShort {
                minimum: Self::MINIMUM_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for MachineId {
    type Error = MachineIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for MachineId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Machine identifier validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineIdError {
    /// The identifier was too short to be unguessable.
    TooShort {
        /// Shortest accepted.
        minimum: usize,
        /// Supplied length.
        actual: usize,
    },
}

impl fmt::Display for MachineIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { minimum, actual } => write!(
                formatter,
                "machine id is {actual} bytes; at least {minimum} are needed"
            ),
        }
    }
}

impl Error for MachineIdError {}

/// Which secret is being stored.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CredentialSlot {
    /// A long-lived token used to renew a lease.
    RefreshToken,
    /// A redemption key, kept so renewal does not re-prompt.
    LicenceKey,
}

impl CredentialSlot {
    /// Returns the stable storage name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RefreshToken => "refresh-token",
            Self::LicenceKey => "licence-key",
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
    fn a_machine_id_must_be_long_enough_to_be_unguessable() {
        assert!(matches!(
            MachineId::new("short"),
            Err(MachineIdError::TooShort { actual: 5, .. })
        ));
        assert!(MachineId::new("0123456789abcdef").is_ok());
    }

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

        assert_eq!(
            store.retrieve(CredentialSlot::RefreshToken).unwrap(),
            Some("token".to_owned())
        );
        assert_eq!(
            store.retrieve(CredentialSlot::LicenceKey).unwrap(),
            Some("key".to_owned())
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
