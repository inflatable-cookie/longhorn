use std::{error::Error, fmt, path::PathBuf};

use crate::{RootKind, StorageRootError};

use super::{PlatformDirectoryFact, StorageProfile};

/// Storage layout resolution failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageLayoutError {
    /// Required platform fact was not supplied.
    MissingPlatformFact {
        /// Missing fact.
        fact: PlatformDirectoryFact,
    },
    /// Platform fact was not absolute on the executing host.
    InvalidPlatformFact {
        /// Invalid fact.
        fact: PlatformDirectoryFact,
        /// Rejected path.
        path: PathBuf,
    },
    /// Portable profile did not receive its one explicit root.
    PortableRootRequired,
    /// A portable root was supplied to another profile.
    PortableRootForbidden {
        /// Profile that rejected it.
        profile: StorageProfile,
    },
    /// Portable root was not absolute.
    InvalidPortableRoot {
        /// Rejected path.
        path: PathBuf,
    },
    /// Per-purpose override was not absolute.
    InvalidOverride {
        /// Root being overridden.
        kind: RootKind,
        /// Rejected path.
        path: PathBuf,
    },
    /// Resolved roots failed the store root contract.
    StorageRoots(StorageRootError),
}

impl fmt::Display for StorageLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPlatformFact { fact } => {
                write!(formatter, "missing platform directory fact {fact:?}")
            }
            Self::InvalidPlatformFact { fact, path } => {
                write!(
                    formatter,
                    "platform directory fact {fact:?} must be absolute: {}",
                    path.display()
                )
            }
            Self::PortableRootRequired => {
                formatter.write_str("portable-v1 requires one explicit absolute root")
            }
            Self::PortableRootForbidden { profile } => {
                write!(
                    formatter,
                    "{} does not accept a portable root",
                    profile.id()
                )
            }
            Self::InvalidPortableRoot { path } => {
                write!(
                    formatter,
                    "portable root must be absolute: {}",
                    path.display()
                )
            }
            Self::InvalidOverride { kind, path } => {
                write!(
                    formatter,
                    "{kind:?} override must be absolute: {}",
                    path.display()
                )
            }
            Self::StorageRoots(error) => error.fmt(formatter),
        }
    }
}

impl Error for StorageLayoutError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::StorageRoots(error) => Some(error),
            _ => None,
        }
    }
}

impl From<StorageRootError> for StorageLayoutError {
    fn from(value: StorageRootError) -> Self {
        Self::StorageRoots(value)
    }
}
