use std::{error::Error, fmt, path::PathBuf};

use longhorn_core::DomainId;

use crate::CoordinationFailure;

/// Planning refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageTransitionPlanError {
    /// Preview contains conflicts which must not be merged silently.
    Conflicts {
        /// Number of blocking conflicts.
        count: usize,
    },
}

impl fmt::Display for StorageTransitionPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflicts { count } => write!(formatter, "{count} transition conflicts remain"),
        }
    }
}

impl Error for StorageTransitionPlanError {}

/// Inspection, execution, or recovery failure.
#[derive(Debug)]
pub enum StorageTransitionError {
    /// Inventory bounds are zero or inverted.
    InvalidLimits,
    /// Transition id is not a stable portable identifier.
    InvalidTransitionId,
    /// Legacy candidate id is empty or too long.
    InvalidLegacyCandidate,
    /// Source and target canonical application ids differ.
    LayoutIdentityMismatch,
    /// A store does not use the roots declared by its layout.
    LayoutStoreMismatch,
    /// Persisted target selection does not describe the target layout.
    TargetSelectionMismatch,
    /// Registered source and target descriptors differ.
    DescriptorMismatch {
        /// Affected domain.
        domain: DomainId,
    },
    /// A migratable domain has no explicit policy.
    MissingPolicy {
        /// Affected domain.
        domain: DomainId,
    },
    /// Domain authority cannot participate.
    UnavailableDomain {
        /// Affected domain.
        domain: DomainId,
    },
    /// File, total, or unknown inventory bound was exceeded.
    BoundExceeded {
        /// Path observed at the bound.
        path: PathBuf,
    },
    /// Filesystem operation failed.
    Filesystem {
        /// Affected path.
        path: PathBuf,
        /// Diagnostic detail.
        detail: String,
    },
    /// Adapter authority failed.
    Adapter {
        /// Affected domain.
        domain: DomainId,
        /// Stable adapter detail.
        detail: String,
    },
    /// Current evidence no longer matches confirmation.
    StalePlan,
    /// Receipt-bound source cleanup no longer has safe authority.
    CleanupRefused(String),
    /// A deterministic authority lock failed.
    Coordination(CoordinationFailure),
    /// Durable transition journal is invalid or unavailable.
    Journal(String),
    /// Fixed bootstrap locator is invalid or unavailable.
    Locator(String),
    /// Terminal source or target authority could not be verified.
    RecoveryRequired(String),
}

impl fmt::Display for StorageTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => {
                formatter.write_str("storage transition inventory bounds are zero or inverted")
            }
            Self::InvalidTransitionId => {
                formatter.write_str("storage transition id is not a stable portable identifier")
            }
            Self::InvalidLegacyCandidate => {
                formatter.write_str("legacy storage candidate id is empty or too long")
            }
            Self::LayoutIdentityMismatch => {
                formatter.write_str("source and target canonical application ids differ")
            }
            Self::LayoutStoreMismatch => {
                formatter.write_str("a store does not use the roots declared by its layout")
            }
            Self::TargetSelectionMismatch => formatter
                .write_str("persisted target selection does not describe the target layout"),
            Self::DescriptorMismatch { domain } => write!(
                formatter,
                "source and target descriptors for domain {domain} differ"
            ),
            Self::MissingPolicy { domain } => {
                write!(
                    formatter,
                    "domain {domain} has no explicit transition policy"
                )
            }
            Self::UnavailableDomain { domain } => {
                write!(formatter, "domain {domain} authority cannot participate")
            }
            Self::BoundExceeded { path } => write!(
                formatter,
                "storage transition inventory bound exceeded at {}",
                path.display()
            ),
            Self::Filesystem { path, detail } => write!(
                formatter,
                "filesystem operation on {} failed: {detail}",
                path.display()
            ),
            Self::Adapter { domain, detail } => {
                write!(
                    formatter,
                    "adapter authority for domain {domain} failed: {detail}"
                )
            }
            Self::StalePlan => formatter
                .write_str("current evidence no longer matches the confirmed transition plan"),
            Self::CleanupRefused(detail) => write!(
                formatter,
                "source cleanup no longer has safe authority: {detail}"
            ),
            Self::Coordination(error) => {
                write!(formatter, "transition coordination failed: {error}")
            }
            Self::Journal(detail) => {
                write!(
                    formatter,
                    "transition journal is invalid or unavailable: {detail}"
                )
            }
            Self::Locator(detail) => {
                write!(
                    formatter,
                    "bootstrap locator is invalid or unavailable: {detail}"
                )
            }
            Self::RecoveryRequired(detail) => write!(
                formatter,
                "terminal source or target authority could not be verified: {detail}"
            ),
        }
    }
}

impl Error for StorageTransitionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Coordination(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoordinationFailureKind;

    fn domain(value: &str) -> DomainId {
        DomainId::new(value).expect("fixture domain id")
    }

    #[test]
    fn storage_transition_error_messages_are_hand_written() {
        let cases: [(StorageTransitionError, &str); 18] = [
            (
                StorageTransitionError::InvalidLimits,
                "storage transition inventory bounds are zero or inverted",
            ),
            (
                StorageTransitionError::InvalidTransitionId,
                "storage transition id is not a stable portable identifier",
            ),
            (
                StorageTransitionError::InvalidLegacyCandidate,
                "legacy storage candidate id is empty or too long",
            ),
            (
                StorageTransitionError::LayoutIdentityMismatch,
                "source and target canonical application ids differ",
            ),
            (
                StorageTransitionError::LayoutStoreMismatch,
                "a store does not use the roots declared by its layout",
            ),
            (
                StorageTransitionError::TargetSelectionMismatch,
                "persisted target selection does not describe the target layout",
            ),
            (
                StorageTransitionError::DescriptorMismatch {
                    domain: domain("settings"),
                },
                "source and target descriptors for domain settings differ",
            ),
            (
                StorageTransitionError::MissingPolicy {
                    domain: domain("settings"),
                },
                "domain settings has no explicit transition policy",
            ),
            (
                StorageTransitionError::UnavailableDomain {
                    domain: domain("settings"),
                },
                "domain settings authority cannot participate",
            ),
            (
                StorageTransitionError::BoundExceeded {
                    path: PathBuf::from("/tmp/bound"),
                },
                "storage transition inventory bound exceeded at /tmp/bound",
            ),
            (
                StorageTransitionError::Filesystem {
                    path: PathBuf::from("/tmp/store"),
                    detail: "permission denied".to_owned(),
                },
                "filesystem operation on /tmp/store failed: permission denied",
            ),
            (
                StorageTransitionError::Adapter {
                    domain: domain("settings"),
                    detail: "refused".to_owned(),
                },
                "adapter authority for domain settings failed: refused",
            ),
            (
                StorageTransitionError::StalePlan,
                "current evidence no longer matches the confirmed transition plan",
            ),
            (
                StorageTransitionError::CleanupRefused("receipt mismatch".to_owned()),
                "source cleanup no longer has safe authority: receipt mismatch",
            ),
            (
                StorageTransitionError::Coordination(CoordinationFailure {
                    kind: CoordinationFailureKind::Busy,
                    lock_path: PathBuf::from("/tmp/lock"),
                    detail: "another writer".to_owned(),
                }),
                "transition coordination failed: Busy coordination failure at /tmp/lock: another writer",
            ),
            (
                StorageTransitionError::Journal("truncated".to_owned()),
                "transition journal is invalid or unavailable: truncated",
            ),
            (
                StorageTransitionError::Locator("missing".to_owned()),
                "bootstrap locator is invalid or unavailable: missing",
            ),
            (
                StorageTransitionError::RecoveryRequired("unverified".to_owned()),
                "terminal source or target authority could not be verified: unverified",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
