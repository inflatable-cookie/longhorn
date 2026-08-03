use std::{error::Error, fmt, path::PathBuf, time::Duration};

use longhorn_core::DomainId;

use crate::{BackupAdapterId, BackupAdapterStateEvidence, BackupLimits, Sha256Digest};

use super::super::RestoreFailureTerminal;

/// One confirmation-bound member of a grouped custom-adapter restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupPlanEntry {
    pub(crate) domain: DomainId,
    pub(crate) adapter: BackupAdapterId,
    pub(crate) adapter_confirmation: Sha256Digest,
    pub(crate) target_evidence: BackupAdapterStateEvidence,
    pub(crate) rollback_evidence: BackupAdapterStateEvidence,
}

impl RestoreAdapterGroupPlanEntry {
    /// Returns the selected domain.
    #[must_use]
    pub const fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the inspected adapter id.
    #[must_use]
    pub const fn adapter(&self) -> &BackupAdapterId {
        &self.adapter
    }

    /// Returns the per-domain confirmation included in the group digest.
    #[must_use]
    pub const fn adapter_confirmation(&self) -> &Sha256Digest {
        &self.adapter_confirmation
    }

    /// Returns the expected target semantic evidence.
    #[must_use]
    pub const fn target_evidence(&self) -> &BackupAdapterStateEvidence {
        &self.target_evidence
    }

    /// Returns exact rollback semantic state.
    #[must_use]
    pub const fn rollback_evidence(&self) -> &BackupAdapterStateEvidence {
        &self.rollback_evidence
    }
}

/// Exact sorted selection and one confirmation for a grouped restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupPlan {
    pub(crate) archive_sha256: Sha256Digest,
    pub(crate) entries: Vec<RestoreAdapterGroupPlanEntry>,
    pub(crate) confirmation_digest: Sha256Digest,
}

impl RestoreAdapterGroupPlan {
    /// Returns the bound archive digest.
    #[must_use]
    pub const fn archive_sha256(&self) -> &Sha256Digest {
        &self.archive_sha256
    }

    /// Returns the complete sorted selection.
    #[must_use]
    pub fn entries(&self) -> &[RestoreAdapterGroupPlanEntry] {
        &self.entries
    }

    /// Returns the digest required to confirm group execution.
    #[must_use]
    pub const fn confirmation_digest(&self) -> &Sha256Digest {
        &self.confirmation_digest
    }
}

/// Invalid grouped custom-adapter selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreAdapterGroupPlanError {
    /// Application or producer identity is incompatible.
    IdentityMismatch,
    /// No custom domains were selected.
    Empty,
    /// One domain appeared more than once.
    DuplicateDomain {
        /// Repeated domain.
        domain: DomainId,
    },
    /// One selected domain was not adapter-ready in the inspection.
    UnknownDomain {
        /// Unknown domain.
        domain: DomainId,
    },
    /// One selected adapter did not declare grouped participation.
    GroupedParticipationRequired {
        /// Rejected domain.
        domain: DomainId,
        /// Inspected adapter.
        adapter: BackupAdapterId,
    },
}

impl fmt::Display for RestoreAdapterGroupPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityMismatch => formatter
                .write_str("grouped restore application or producer identity does not match"),
            Self::Empty => formatter.write_str("grouped restore selection cannot be empty"),
            Self::DuplicateDomain { domain } => {
                write!(formatter, "grouped restore repeats domain {domain}")
            }
            Self::UnknownDomain { domain } => {
                write!(
                    formatter,
                    "grouped restore domain {domain} was not inspected"
                )
            }
            Self::GroupedParticipationRequired { domain, adapter } => write!(
                formatter,
                "custom restore adapter {} for {domain} does not declare grouped failure atomicity",
                adapter.as_str()
            ),
        }
    }
}

impl Error for RestoreAdapterGroupPlanError {}

/// Bounds and coordination timeout for one grouped execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupExecutionOptions {
    pub(crate) lock_timeout: Duration,
    pub(crate) limits: BackupLimits,
}

impl RestoreAdapterGroupExecutionOptions {
    /// Constructs grouped execution bounds.
    #[must_use]
    pub const fn new(lock_timeout: Duration, limits: BackupLimits) -> Self {
        Self {
            lock_timeout,
            limits,
        }
    }

    /// Returns the store coordinator timeout.
    #[must_use]
    pub const fn lock_timeout(self) -> Duration {
        self.lock_timeout
    }

    /// Returns grouped target and rollback payload limits.
    #[must_use]
    pub const fn limits(self) -> BackupLimits {
        self.limits
    }
}

/// Durable stage reached by a grouped execution failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreAdapterGroupExecutionStage {
    /// Earlier restore recovery or coordinator acquisition.
    RecoverPrevious,
    /// Archive, selection, or confirmation validation.
    ValidatePlan,
    /// Fresh descriptor, adapter, or semantic inspection.
    Reinspect,
    /// Side-effect-free target and rollback staging.
    Stage,
    /// Durable payload or journal publication.
    PublishJournal,
    /// Adapter target publication.
    ApplyTarget,
    /// Complete target semantic verification.
    VerifyTarget,
    /// Complete old-state rollback.
    Rollback,
    /// Terminal private-material cleanup.
    Cleanup,
}

/// Grouped execution failure with exact terminal classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupError {
    pub(crate) stage: RestoreAdapterGroupExecutionStage,
    pub(crate) domain: Option<DomainId>,
    pub(crate) terminal: RestoreFailureTerminal,
    pub(crate) detail: String,
}

impl RestoreAdapterGroupError {
    /// Returns the failed transaction stage.
    #[must_use]
    pub const fn stage(&self) -> RestoreAdapterGroupExecutionStage {
        self.stage
    }

    /// Returns the affected domain when one exists.
    #[must_use]
    pub const fn domain(&self) -> Option<&DomainId> {
        self.domain.as_ref()
    }

    /// Returns the exact terminal classification.
    #[must_use]
    pub const fn terminal(&self) -> RestoreFailureTerminal {
        self.terminal
    }

    /// Returns bounded safe failure detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for RestoreAdapterGroupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "grouped adapter restore failed at {:?}",
            self.stage
        )?;
        if let Some(domain) = &self.domain {
            write!(formatter, " for {domain}")?;
        }
        write!(formatter, ": {}", self.detail)
    }
}

impl Error for RestoreAdapterGroupError {}

/// Target and rollback evidence retained for one grouped receipt member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupReceiptEntry {
    pub(crate) domain: DomainId,
    pub(crate) target_evidence: BackupAdapterStateEvidence,
    pub(crate) rollback_evidence: BackupAdapterStateEvidence,
}

impl RestoreAdapterGroupReceiptEntry {
    /// Returns the grouped domain.
    #[must_use]
    pub const fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the exact archive target state.
    #[must_use]
    pub const fn target_evidence(&self) -> &BackupAdapterStateEvidence {
        &self.target_evidence
    }

    /// Returns the exact pre-transaction state.
    #[must_use]
    pub const fn rollback_evidence(&self) -> &BackupAdapterStateEvidence {
        &self.rollback_evidence
    }
}

impl From<&RestoreAdapterGroupPlanEntry> for RestoreAdapterGroupReceiptEntry {
    fn from(entry: &RestoreAdapterGroupPlanEntry) -> Self {
        Self {
            domain: entry.domain.clone(),
            target_evidence: entry.target_evidence.clone(),
            rollback_evidence: entry.rollback_evidence.clone(),
        }
    }
}

/// Successful complete grouped restore receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupExecutionReceipt {
    pub(crate) confirmation_digest: Sha256Digest,
    pub(crate) entries: Vec<RestoreAdapterGroupReceiptEntry>,
}

impl RestoreAdapterGroupExecutionReceipt {
    /// Returns the executed group confirmation.
    #[must_use]
    pub const fn confirmation_digest(&self) -> &Sha256Digest {
        &self.confirmation_digest
    }

    /// Returns every target and rollback evidence pair in stable domain order.
    #[must_use]
    pub fn entries(&self) -> &[RestoreAdapterGroupReceiptEntry] {
        &self.entries
    }
}

/// Terminal boot recovery outcome for one grouped restore journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreAdapterGroupRecoveryOutcome {
    /// No grouped journal existed.
    NoRecoveryNeeded,
    /// The complete old generation was restored and verified.
    RolledBack,
    /// A durable terminal journal only required cleanup.
    TerminalCleanup,
}

/// Machine-readable grouped boot recovery receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupRecoveryReceipt {
    pub(crate) outcome: RestoreAdapterGroupRecoveryOutcome,
    pub(crate) entries: Vec<RestoreAdapterGroupReceiptEntry>,
}

impl RestoreAdapterGroupRecoveryReceipt {
    /// Returns the terminal recovery outcome.
    #[must_use]
    pub const fn outcome(&self) -> RestoreAdapterGroupRecoveryOutcome {
        self.outcome
    }

    /// Returns every journalled target and rollback evidence pair.
    #[must_use]
    pub fn entries(&self) -> &[RestoreAdapterGroupReceiptEntry] {
        &self.entries
    }
}

/// Grouped boot recovery could not verify exact rollback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreAdapterGroupRecoveryError {
    pub(crate) path: PathBuf,
    pub(crate) domain: Option<DomainId>,
    pub(crate) detail: String,
}

impl RestoreAdapterGroupRecoveryError {
    /// Returns the grouped journal path.
    #[must_use]
    pub const fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns the affected domain when one exists.
    #[must_use]
    pub const fn domain(&self) -> Option<&DomainId> {
        self.domain.as_ref()
    }

    /// Returns bounded safe failure detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for RestoreAdapterGroupRecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "grouped adapter recovery failed at {}",
            self.path.display()
        )?;
        if let Some(domain) = &self.domain {
            write!(formatter, " for {domain}")?;
        }
        write!(formatter, ": {}", self.detail)
    }
}

impl Error for RestoreAdapterGroupRecoveryError {}
