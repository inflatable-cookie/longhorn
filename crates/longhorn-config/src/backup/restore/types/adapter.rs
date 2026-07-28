use std::{error::Error, fmt};

use longhorn_core::DomainId;

use crate::{
    BackupAdapterError, BackupAdapterId, BackupAdapterRestoreOutcome,
    BackupAdapterRestoreParticipation, Sha256Digest,
};

/// Minimum guarantee required by an explicit custom restore call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreAdapterRequirement {
    /// Refuse any adapter that cannot prove exact rollback.
    FailureAtomic,
    /// Permit a separately receipted nontransactional operation.
    AllowSeparate,
}

/// Machine-readable result of one explicit adapter-owned restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterReceipt {
    domain: DomainId,
    adapter: BackupAdapterId,
    participation: BackupAdapterRestoreParticipation,
    confirmation_digest: Sha256Digest,
    outcome: BackupAdapterRestoreOutcome,
}

impl RestoreAdapterReceipt {
    pub(crate) fn new(
        domain: DomainId,
        adapter: BackupAdapterId,
        participation: BackupAdapterRestoreParticipation,
        confirmation_digest: Sha256Digest,
        outcome: BackupAdapterRestoreOutcome,
    ) -> Self {
        Self {
            domain,
            adapter,
            participation,
            confirmation_digest,
            outcome,
        }
    }

    /// Returns the restored custom domain.
    #[must_use]
    pub const fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the stable adapter id.
    #[must_use]
    pub const fn adapter(&self) -> &BackupAdapterId {
        &self.adapter
    }

    /// Returns the adapter-owned transaction guarantee.
    #[must_use]
    pub const fn participation(&self) -> &BackupAdapterRestoreParticipation {
        &self.participation
    }

    /// Returns the digest explicitly confirmed by the caller.
    #[must_use]
    pub const fn confirmation_digest(&self) -> &Sha256Digest {
        &self.confirmation_digest
    }

    /// Returns the terminal state reported by the adapter authority.
    #[must_use]
    pub const fn outcome(&self) -> &BackupAdapterRestoreOutcome {
        &self.outcome
    }
}

/// Failure before a truthful custom restore receipt exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreAdapterError {
    /// Stable application or producer identity does not match.
    IdentityMismatch,
    /// Archive bytes differ from the inspected source.
    ArchiveChanged,
    /// Domain is absent from the inspected manifest.
    UnknownDomain {
        /// Requested domain.
        domain: DomainId,
    },
    /// Domain is not currently backed by the inspected adapter.
    AdapterChanged {
        /// Requested domain.
        domain: DomainId,
    },
    /// The caller supplied a stale or unrelated confirmation digest.
    ConfirmationMismatch {
        /// Requested domain.
        domain: DomainId,
    },
    /// Caller required failure atomicity but the adapter declared less.
    FailureAtomicRequired {
        /// Requested domain.
        domain: DomainId,
        /// Stable adapter id.
        adapter: BackupAdapterId,
    },
    /// Restore is explicitly excluded.
    Excluded {
        /// Requested domain.
        domain: DomainId,
        /// Stable reason.
        reason: String,
    },
    /// Side-effect-free reinspection no longer matches the confirmed preview.
    PreviewChanged {
        /// Requested domain.
        domain: DomainId,
    },
    /// Adapter terminal evidence contradicted its confirmed capability claim.
    OutcomeEvidenceMismatch {
        /// Requested domain.
        domain: DomainId,
    },
    /// Adapter execution failed before a terminal receipt was available.
    AdapterFailed {
        /// Requested domain.
        domain: DomainId,
        /// Stable adapter id.
        adapter: BackupAdapterId,
        /// Stable adapter failure.
        error: BackupAdapterError,
    },
}

impl fmt::Display for RestoreAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityMismatch => {
                formatter.write_str("restore application or producer identity does not match")
            }
            Self::ArchiveChanged => {
                formatter.write_str("custom restore archive changed after inspection")
            }
            Self::UnknownDomain { domain } => {
                write!(
                    formatter,
                    "custom restore domain {domain} was not inspected"
                )
            }
            Self::AdapterChanged { domain } => {
                write!(formatter, "custom restore adapter for {domain} changed")
            }
            Self::ConfirmationMismatch { domain } => {
                write!(
                    formatter,
                    "custom restore confirmation for {domain} does not match"
                )
            }
            Self::FailureAtomicRequired { domain, adapter } => write!(
                formatter,
                "custom restore adapter {} for {domain} cannot prove exact rollback",
                adapter.as_str()
            ),
            Self::Excluded { domain, reason } => {
                write!(
                    formatter,
                    "custom restore domain {domain} is excluded: {reason}"
                )
            }
            Self::PreviewChanged { domain } => {
                write!(formatter, "custom restore preview for {domain} changed")
            }
            Self::OutcomeEvidenceMismatch { domain } => {
                write!(
                    formatter,
                    "custom restore outcome evidence for {domain} does not match its confirmed preview"
                )
            }
            Self::AdapterFailed {
                domain,
                adapter,
                error,
            } => write!(
                formatter,
                "custom restore adapter {} failed for {domain}: {error}",
                adapter.as_str()
            ),
        }
    }
}

impl Error for RestoreAdapterError {}
