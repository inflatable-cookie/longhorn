use std::{collections::BTreeMap, error::Error, fmt, path::PathBuf};

use longhorn_core::{DomainId, SchemaVersion};

use crate::Sha256Digest;

use super::inspection::RestoreDomainCompatibility;

/// Explicit resolution for one archive-domain conflict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreConflictChoice {
    /// Apply the archive state, replacing or deleting current state as required.
    UseArchive,
    /// Keep current state and explicitly skip this archive domain.
    KeepCurrent,
}

/// Exact conflict choices for every included manifest domain.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RestoreChoices {
    pub(crate) choices: BTreeMap<DomainId, RestoreConflictChoice>,
}

impl RestoreChoices {
    /// Constructs an empty choice set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds exactly one choice for a manifest domain.
    pub fn choose(
        &mut self,
        domain: DomainId,
        choice: RestoreConflictChoice,
    ) -> Result<(), RestoreChoiceError> {
        if self.choices.insert(domain.clone(), choice).is_some() {
            return Err(RestoreChoiceError::DuplicateDomain { domain });
        }
        Ok(())
    }

    /// Returns the choice for one domain.
    #[must_use]
    pub fn get(&self, domain: &DomainId) -> Option<RestoreConflictChoice> {
        self.choices.get(domain).copied()
    }
}

/// Invalid explicit restore choice set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreChoiceError {
    /// A domain received more than one conflict choice.
    DuplicateDomain {
        /// Duplicate domain.
        domain: DomainId,
    },
}

impl fmt::Display for RestoreChoiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDomain { domain } => {
                write!(
                    formatter,
                    "restore choice for domain {domain} is already declared"
                )
            }
        }
    }
}

impl Error for RestoreChoiceError {}

/// Exact current-file evidence bound into a restore plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreCurrentEvidence {
    /// No current target file exists.
    Absent,
    /// A current target file exists with exact length and digest.
    Present {
        /// Exact current byte length.
        byte_length: u64,
        /// SHA-256 over exact current bytes.
        sha256: Sha256Digest,
    },
}

/// Planned result for one selected archive domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreAction {
    /// Create a missing current file.
    Create,
    /// Replace a different current file.
    Replace,
    /// Delete a current file because the archive records absence.
    Delete,
    /// Migrate older archive state and publish the current-schema result.
    Migrate,
    /// Current state already equals the complete staged target.
    Unchanged,
}

/// One explicit choice and its derived selected action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanEntry {
    pub(crate) domain: DomainId,
    pub(crate) choice: RestoreConflictChoice,
    pub(crate) action: Option<RestoreAction>,
    pub(crate) current: Option<RestoreCurrentEvidence>,
}

impl RestorePlanEntry {
    /// Returns the manifest domain.
    #[must_use]
    pub fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the caller's explicit conflict choice.
    #[must_use]
    pub const fn choice(&self) -> RestoreConflictChoice {
        self.choice
    }

    /// Returns the selected action, or `None` for an explicit skip.
    #[must_use]
    pub const fn action(&self) -> Option<RestoreAction> {
        self.action
    }

    /// Returns current evidence for selected actions.
    #[must_use]
    pub fn current(&self) -> Option<&RestoreCurrentEvidence> {
        self.current.as_ref()
    }
}

/// Machine-readable counts for a complete confirmation-bound plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanReceipt {
    pub(crate) selected: usize,
    pub(crate) skipped: usize,
    pub(crate) creates: usize,
    pub(crate) replaces: usize,
    pub(crate) deletes: usize,
    pub(crate) migrations: usize,
    pub(crate) unchanged: usize,
}

impl RestorePlanReceipt {
    /// Returns selected archive domains.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// Returns explicitly skipped archive domains.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    /// Returns actions of the requested kind.
    #[must_use]
    pub const fn actions(&self, action: RestoreAction) -> usize {
        match action {
            RestoreAction::Create => self.creates,
            RestoreAction::Replace => self.replaces,
            RestoreAction::Delete => self.deletes,
            RestoreAction::Migrate => self.migrations,
            RestoreAction::Unchanged => self.unchanged,
        }
    }
}

/// Complete restore plan bound to archive, choices, actions, and current evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlan {
    pub(crate) archive_sha256: Sha256Digest,
    pub(crate) digest: Sha256Digest,
    pub(crate) entries: Vec<RestorePlanEntry>,
    pub(crate) targets: Vec<PlannedTarget>,
    pub(crate) receipt: RestorePlanReceipt,
}

impl RestorePlan {
    /// Returns the canonical confirmation digest.
    #[must_use]
    pub fn digest(&self) -> &Sha256Digest {
        &self.digest
    }

    /// Returns the bound source archive digest.
    #[must_use]
    pub fn archive_sha256(&self) -> &Sha256Digest {
        &self.archive_sha256
    }

    /// Returns every explicit choice in manifest order.
    #[must_use]
    pub fn entries(&self) -> &[RestorePlanEntry] {
        &self.entries
    }

    /// Returns plan counts.
    #[must_use]
    pub fn receipt(&self) -> &RestorePlanReceipt {
        &self.receipt
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlannedTarget {
    pub(crate) domain: DomainId,
    pub(crate) action: RestoreAction,
    pub(crate) current: RestoreCurrentEvidence,
    pub(crate) path: PathBuf,
    pub(crate) target_schema_version: Option<SchemaVersion>,
    pub(crate) target_sha256: Option<Sha256Digest>,
}

/// Failure to construct an exact confirmation-bound restore plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestorePlanError {
    /// Stable application or producer identity does not match.
    IdentityMismatch,
    /// Choices omitted manifest domains.
    MissingChoices {
        /// Omitted domains.
        domains: Vec<DomainId>,
    },
    /// Choices named domains absent from the manifest.
    UnexpectedChoices {
        /// Unknown choice domains.
        domains: Vec<DomainId>,
    },
    /// No domain selected archive state.
    EmptySelection,
    /// A selected domain is not restorable.
    DomainNotRestorable {
        /// Rejected domain.
        domain: DomainId,
        /// Inspection result causing rejection.
        compatibility: RestoreDomainCompatibility,
    },
    /// Current target evidence could not be read.
    CurrentReadFailed {
        /// Affected domain.
        domain: DomainId,
        /// Target path.
        path: PathBuf,
        /// Filesystem detail.
        detail: String,
    },
}

impl fmt::Display for RestorePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityMismatch => {
                formatter.write_str("restore application or producer identity does not match")
            }
            Self::MissingChoices { domains } => {
                write!(
                    formatter,
                    "restore choices omit manifest domains: {domains:?}"
                )
            }
            Self::UnexpectedChoices { domains } => {
                write!(
                    formatter,
                    "restore choices name unknown domains: {domains:?}"
                )
            }
            Self::EmptySelection => formatter.write_str("restore selection cannot be empty"),
            Self::DomainNotRestorable {
                domain,
                compatibility,
            } => write!(
                formatter,
                "domain {domain} is not restorable: {compatibility:?}"
            ),
            Self::CurrentReadFailed {
                domain,
                path,
                detail,
            } => write!(
                formatter,
                "cannot read current evidence for {domain} at {}: {detail}",
                path.display()
            ),
        }
    }
}

impl Error for RestorePlanError {}
