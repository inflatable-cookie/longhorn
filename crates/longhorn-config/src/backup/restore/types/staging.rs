use std::{error::Error, fmt, path::PathBuf, time::Duration};

use longhorn_core::{DomainId, SchemaVersion};

use crate::{CoordinationFailure, Sha256Digest};

use super::planning::{RestoreAction, RestoreCurrentEvidence};

/// Finite coordinator wait for private restore staging.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestorePrepareOptions {
    /// Maximum time spent acquiring the store coordinator.
    pub lock_timeout: Duration,
}

impl RestorePrepareOptions {
    /// Constructs explicit preparation options.
    #[must_use]
    pub const fn new(lock_timeout: Duration) -> Self {
        Self { lock_timeout }
    }
}

/// Failure before a complete private restore staging set exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestorePrepareError {
    /// Plan and supplied inspection name different archive bytes.
    ArchiveChanged,
    /// The mutation coordinator could not be acquired.
    Coordination(CoordinationFailure),
    /// Current state changed after confirmation.
    StaleCurrent {
        /// First changed selected domain.
        domain: DomainId,
        /// Evidence bound into the plan.
        planned: RestoreCurrentEvidence,
        /// Evidence observed under the coordinator.
        observed: RestoreCurrentEvidence,
    },
    /// Current target evidence could not be reread.
    CurrentReadFailed {
        /// Affected domain.
        domain: DomainId,
        /// Target path.
        path: PathBuf,
        /// Filesystem detail.
        detail: String,
    },
    /// Selected domain code or policy no longer matches inspection.
    DomainCapabilityChanged {
        /// Affected domain.
        domain: DomainId,
    },
    /// Migration, decoding, encoding, or validation failed during staging.
    DomainStagingFailed {
        /// Affected domain.
        domain: DomainId,
        /// Typed source or target-preparation detail.
        detail: String,
    },
    /// Consumer migration or encoding produced a different target after confirmation.
    TargetChanged {
        /// Affected domain.
        domain: DomainId,
    },
}

impl fmt::Display for RestorePrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveChanged => formatter.write_str("restore archive changed after planning"),
            Self::Coordination(error) => error.fmt(formatter),
            Self::StaleCurrent { domain, .. } => {
                write!(
                    formatter,
                    "current state for {domain} changed after confirmation"
                )
            }
            Self::CurrentReadFailed {
                domain,
                path,
                detail,
            } => write!(
                formatter,
                "cannot recheck current evidence for {domain} at {}: {detail}",
                path.display()
            ),
            Self::DomainCapabilityChanged { domain } => {
                write!(
                    formatter,
                    "restore capability for {domain} changed after inspection"
                )
            }
            Self::DomainStagingFailed { domain, detail } => {
                write!(formatter, "cannot stage {domain}: {detail}")
            }
            Self::TargetChanged { domain } => {
                write!(
                    formatter,
                    "staged target for {domain} changed after confirmation"
                )
            }
        }
    }
}

impl Error for RestorePrepareError {}

/// Machine-readable result of complete private staging.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreStagingReceipt {
    pub(crate) selected: usize,
    pub(crate) documents: usize,
    pub(crate) deletions: usize,
    pub(crate) unchanged: usize,
    pub(crate) total_document_bytes: u64,
}

impl RestoreStagingReceipt {
    /// Returns selected domains.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// Returns current-schema documents retained in private memory.
    #[must_use]
    pub const fn documents(&self) -> usize {
        self.documents
    }

    /// Returns staged deletions.
    #[must_use]
    pub const fn deletions(&self) -> usize {
        self.deletions
    }

    /// Returns selected domains requiring no publication.
    #[must_use]
    pub const fn unchanged(&self) -> usize {
        self.unchanged
    }

    /// Returns exact staged current-schema document bytes.
    #[must_use]
    pub const fn total_document_bytes(&self) -> u64 {
        self.total_document_bytes
    }
}

/// Complete all-or-nothing private target staging.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreStaging {
    pub(crate) archive_sha256: Sha256Digest,
    pub(crate) plan_digest: Sha256Digest,
    pub(crate) domains: Vec<StagedDomain>,
    pub(crate) receipt: RestoreStagingReceipt,
}

impl RestoreStaging {
    /// Returns the confirmation digest this staging set satisfies.
    #[must_use]
    pub fn plan_digest(&self) -> &Sha256Digest {
        &self.plan_digest
    }

    /// Returns the bound archive digest.
    #[must_use]
    pub fn archive_sha256(&self) -> &Sha256Digest {
        &self.archive_sha256
    }

    /// Returns complete staging counts.
    #[must_use]
    pub fn receipt(&self) -> &RestoreStagingReceipt {
        &self.receipt
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StagedDomain {
    pub(crate) domain: DomainId,
    pub(crate) action: RestoreAction,
    pub(crate) path: PathBuf,
    pub(crate) current: RestoreCurrentEvidence,
    pub(crate) schema_version: Option<SchemaVersion>,
    pub(crate) bytes: Option<Vec<u8>>,
}
