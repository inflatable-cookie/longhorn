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
        write!(formatter, "{self:?}")
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
