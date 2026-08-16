//! Credential storage vocabulary: the rules, not the backend.
//!
//! This vocabulary lives in core because it is host plumbing, not licence
//! domain — it sat in `longhorn-licence` only because licensing needed it
//! first (moved on 2026-08-15, Card 210). Licensing, backup, and consumer
//! systems share the one store shape.

use core::fmt;
use std::{collections::BTreeMap, error::Error, sync::Mutex};

const MAX_SEGMENT_BYTES: usize = 64;
const MAX_PERSISTED_NAME_BYTES: usize = 255;

/// One consumer-owned credential identity segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialSlotSegment {
    /// The consumer or integration namespace.
    Namespace,
    /// The stable non-secret scope discriminator.
    Scope,
    /// The consumer-owned credential purpose.
    Purpose,
}

impl fmt::Display for CredentialSlotSegment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Namespace => "namespace",
            Self::Scope => "scope",
            Self::Purpose => "purpose",
        })
    }
}

/// Credential-slot validation failure.
///
/// Errors identify the rejected segment category and lengths, never the
/// supplied value. A consumer must not put a secret in slot identity, but a
/// diagnostic still does not echo input if that rule is violated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialSlotError {
    /// A required segment was empty.
    Empty {
        /// Which segment was empty.
        segment: CredentialSlotSegment,
    },
    /// A segment exceeded its byte bound.
    SegmentTooLong {
        /// Which segment exceeded the bound.
        segment: CredentialSlotSegment,
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// A segment did not match the lowercase ASCII grammar.
    InvalidSegment {
        /// Which segment was invalid.
        segment: CredentialSlotSegment,
    },
    /// The complete canonical name exceeded the backend-safe bound.
    NameTooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Canonical byte length.
        actual: usize,
    },
}

impl fmt::Display for CredentialSlotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { segment } => write!(formatter, "credential {segment} cannot be empty"),
            Self::SegmentTooLong {
                segment,
                maximum,
                actual,
            } => write!(
                formatter,
                "credential {segment} is {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidSegment { segment } => {
                write!(formatter, "credential {segment} is invalid")
            }
            Self::NameTooLong { maximum, actual } => write!(
                formatter,
                "credential slot name is {actual} bytes; maximum is {maximum}"
            ),
        }
    }
}

impl Error for CredentialSlotError {}

/// Stable identity for one secret in a credential store.
///
/// Built-in constructors retain Longhorn's exact persisted names. Consumer
/// slots use `consumer:<namespace>:<scope>:<purpose>` and keep the meaning of
/// all three segments outside Longhorn.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CredentialSlot(String);

impl CredentialSlot {
    /// The long-lived token used to renew a licence lease.
    #[must_use]
    pub fn refresh_token() -> Self {
        Self("refresh-token".to_owned())
    }

    /// The redemption key retained for licence renewal.
    #[must_use]
    pub fn licence_key() -> Self {
        Self("licence-key".to_owned())
    }

    /// The operational age identity used by automatic encrypted backup.
    #[must_use]
    pub fn backup_identity() -> Self {
        Self("backup-identity".to_owned())
    }

    /// Validates and constructs a consumer-scoped credential slot.
    pub fn consumer_scoped(
        namespace: &str,
        scope: &str,
        purpose: &str,
    ) -> Result<Self, CredentialSlotError> {
        validate_segment(namespace, CredentialSlotSegment::Namespace)?;
        validate_segment(scope, CredentialSlotSegment::Scope)?;
        validate_segment(purpose, CredentialSlotSegment::Purpose)?;

        let name = format!("consumer:{namespace}:{scope}:{purpose}");
        validate_persisted_name(&name)?;
        Ok(Self(name))
    }

    /// Returns the canonical persisted name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CredentialSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn validate_segment(
    value: &str,
    segment: CredentialSlotSegment,
) -> Result<(), CredentialSlotError> {
    if value.is_empty() {
        return Err(CredentialSlotError::Empty { segment });
    }
    if value.len() > MAX_SEGMENT_BYTES {
        return Err(CredentialSlotError::SegmentTooLong {
            segment,
            maximum: MAX_SEGMENT_BYTES,
            actual: value.len(),
        });
    }

    let bytes = value.as_bytes();
    let valid_endpoint = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    let valid_character = |byte: &u8| valid_endpoint(*byte) || *byte == b'-';
    if !valid_endpoint(bytes[0])
        || !valid_endpoint(bytes[bytes.len() - 1])
        || !bytes.iter().all(valid_character)
    {
        return Err(CredentialSlotError::InvalidSegment { segment });
    }

    Ok(())
}

fn validate_persisted_name(name: &str) -> Result<(), CredentialSlotError> {
    if name.len() > MAX_PERSISTED_NAME_BYTES {
        return Err(CredentialSlotError::NameTooLong {
            maximum: MAX_PERSISTED_NAME_BYTES,
            actual: name.len(),
        });
    }
    Ok(())
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
///
/// # There is deliberately no conditional write
///
/// No `store_if_absent`, no compare-and-swap. It was investigated for the
/// age-identity first-run race (Card 224) and refused, because no backend can
/// honour it: `keyring-core` 1.0's `CredentialApi` — the surface behind
/// `longhorn-credential-keyring` — exposes only unconditional `set_secret`,
/// and Windows' `CredWrite` has no create-only flag, so the primitive does
/// not exist cross-platform. A method here whose default was
/// retrieve-then-store would read like mutual exclusion while providing none,
/// and the backend Longhorn ships would be the one breaking it.
///
/// A caller that needs to survive a lost race reads the slot back after
/// writing and adopts what it finds, the way `StoreBackupEncryption` does.
/// That converges rather than excludes, which is what unconditional writes
/// can actually support.
pub trait CredentialStore {
    /// Stores a secret, replacing any existing value in the slot.
    ///
    /// Unconditional: a concurrent writer's value is overwritten without
    /// signal, and there is no way to ask for the write only if the slot is
    /// empty. See the trait note above.
    fn store(&self, slot: &CredentialSlot, secret: &str) -> Result<(), CredentialError>;

    /// Retrieves a secret, or `None` when the slot is empty.
    fn retrieve(&self, slot: &CredentialSlot) -> Result<Option<String>, CredentialError>;

    /// Removes a secret. Removing an empty slot succeeds.
    fn remove(&self, slot: &CredentialSlot) -> Result<(), CredentialError>;
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
    fn store(&self, slot: &CredentialSlot, secret: &str) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(slot.clone(), secret.to_owned());
        Ok(())
    }

    fn retrieve(&self, slot: &CredentialSlot) -> Result<Option<String>, CredentialError> {
        Ok(self
            .secrets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(slot)
            .cloned())
    }

    fn remove(&self, slot: &CredentialSlot) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(slot);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_persisted_names_are_exact() {
        assert_eq!(CredentialSlot::refresh_token().as_str(), "refresh-token");
        assert_eq!(CredentialSlot::licence_key().as_str(), "licence-key");
        assert_eq!(
            CredentialSlot::backup_identity().as_str(),
            "backup-identity"
        );
    }

    #[test]
    fn consumer_segments_accept_the_complete_grammar_and_bounds() {
        let maximum = "a".repeat(MAX_SEGMENT_BYTES);
        let slot = CredentialSlot::consumer_scoped(&maximum, "0-a-9", &maximum).unwrap();

        assert_eq!(slot.as_str(), format!("consumer:{maximum}:0-a-9:{maximum}"));
        assert!(slot.as_str().len() <= MAX_PERSISTED_NAME_BYTES);
    }

    #[test]
    fn each_consumer_segment_rejects_empty_overlong_and_malformed_values() {
        let cases = [
            (CredentialSlotSegment::Namespace, "", "scope", "purpose"),
            (CredentialSlotSegment::Scope, "namespace", "", "purpose"),
            (CredentialSlotSegment::Purpose, "namespace", "scope", ""),
        ];
        for (segment, namespace, scope, purpose) in cases {
            assert_eq!(
                CredentialSlot::consumer_scoped(namespace, scope, purpose),
                Err(CredentialSlotError::Empty { segment })
            );
        }

        let overlong = "a".repeat(MAX_SEGMENT_BYTES + 1);
        let cases = [
            (
                CredentialSlotSegment::Namespace,
                overlong.as_str(),
                "scope",
                "purpose",
            ),
            (
                CredentialSlotSegment::Scope,
                "namespace",
                overlong.as_str(),
                "purpose",
            ),
            (
                CredentialSlotSegment::Purpose,
                "namespace",
                "scope",
                overlong.as_str(),
            ),
        ];
        for (segment, namespace, scope, purpose) in cases {
            assert_eq!(
                CredentialSlot::consumer_scoped(namespace, scope, purpose),
                Err(CredentialSlotError::SegmentTooLong {
                    segment,
                    maximum: MAX_SEGMENT_BYTES,
                    actual: MAX_SEGMENT_BYTES + 1,
                })
            );
        }

        for malformed in [
            "Uppercase",
            "white space",
            "control\ncharacter",
            "separator:value",
            "path/value",
            "..",
            "-leading",
            "trailing-",
            "nonascii-é",
        ] {
            for (segment, namespace, scope, purpose) in [
                (
                    CredentialSlotSegment::Namespace,
                    malformed,
                    "scope",
                    "purpose",
                ),
                (
                    CredentialSlotSegment::Scope,
                    "namespace",
                    malformed,
                    "purpose",
                ),
                (
                    CredentialSlotSegment::Purpose,
                    "namespace",
                    "scope",
                    malformed,
                ),
            ] {
                assert_eq!(
                    CredentialSlot::consumer_scoped(namespace, scope, purpose),
                    Err(CredentialSlotError::InvalidSegment { segment })
                );
            }
        }
    }

    #[test]
    fn complete_persisted_names_are_bounded() {
        assert_eq!(
            validate_persisted_name(&"a".repeat(MAX_PERSISTED_NAME_BYTES + 1)),
            Err(CredentialSlotError::NameTooLong {
                maximum: MAX_PERSISTED_NAME_BYTES,
                actual: MAX_PERSISTED_NAME_BYTES + 1,
            })
        );
    }

    #[test]
    fn validation_errors_never_echo_rejected_input() {
        let rejected = "Token-Shaped-Secret";
        let error = CredentialSlot::consumer_scoped("namespace", "scope", rejected)
            .expect_err("uppercase input must be rejected");

        assert_eq!(
            error,
            CredentialSlotError::InvalidSegment {
                segment: CredentialSlotSegment::Purpose,
            }
        );
        assert!(!error.to_string().contains(rejected));
    }

    #[test]
    fn scoped_slots_round_trip_replace_and_remove_idempotently() {
        let store = MemoryCredentialStore::new();
        let slot = CredentialSlot::consumer_scoped("publisher", "source-1", "signing").unwrap();

        store.store(&slot, "secret").unwrap();
        assert_eq!(store.retrieve(&slot).unwrap(), Some("secret".to_owned()));
        store.store(&slot, "replacement").unwrap();
        assert_eq!(
            store.retrieve(&slot).unwrap(),
            Some("replacement".to_owned())
        );

        store.remove(&slot).unwrap();
        assert_eq!(store.retrieve(&slot).unwrap(), None);
        store
            .remove(&slot)
            .expect("removing an empty slot must succeed");
    }

    #[test]
    fn namespace_scope_and_purpose_each_isolate_slots() {
        let store = MemoryCredentialStore::new();
        let slots = [
            CredentialSlot::consumer_scoped("publisher-a", "source", "signing").unwrap(),
            CredentialSlot::consumer_scoped("publisher-b", "source", "signing").unwrap(),
            CredentialSlot::consumer_scoped("publisher-a", "other-source", "signing").unwrap(),
            CredentialSlot::consumer_scoped("publisher-a", "source", "submit").unwrap(),
        ];

        for (index, slot) in slots.iter().enumerate() {
            store.store(slot, &format!("secret-{index}")).unwrap();
        }
        for (index, slot) in slots.iter().enumerate() {
            assert_eq!(
                store.retrieve(slot).unwrap(),
                Some(format!("secret-{index}"))
            );
        }
    }

    #[test]
    fn a_storage_failure_never_carries_the_secret() {
        let error = CredentialError::Unavailable {
            detail: "keychain locked".into(),
        };

        assert_eq!(
            error.to_string(),
            "credential store unavailable: keychain locked"
        );
    }
}
