use std::path::{Path, PathBuf};

use longhorn_core::DomainId;

use crate::{
    BackupAdapterId, BackupAdapterRestoreParticipation, Sha256Digest, StorageClass,
    StorageProfileSelection,
};

/// Exact current ordinary-file state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageFileEvidence {
    /// File does not exist.
    Absent,
    /// Exact regular-file evidence.
    Present {
        /// Exact byte length.
        byte_length: usize,
        /// Exact byte digest.
        sha256: Sha256Digest,
    },
    /// Adapter-owned semantic evidence.
    Semantic {
        /// Adapter-defined semantic digest.
        sha256: Sha256Digest,
    },
}

/// Visible reason a domain does not migrate as an ordinary file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageTransitionExclusion {
    /// Cache is rebuilt in the target layout.
    CacheRebuilt,
    /// Runtime material is never migrated.
    RuntimeDiscarded,
    /// Secret remains in secure-store authority.
    SecretExternal,
    /// Logs were not opted into evidence migration.
    LogsNotSelected,
    /// Defaults or policy retain external authority.
    ExternalAuthority,
    /// Explicit catalogue exclusion.
    Catalog(String),
}

/// Planned domain behavior.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageTransitionAction {
    /// Copy verified ordinary bytes.
    CopyOrdinary,
    /// Source and target are both absent.
    Absent,
    /// Both layouts resolve the same exact file authority.
    SameAuthority,
    /// Separate paths already hold identical state.
    Identical,
    /// Capture and restore through schema-opaque adapters.
    CustomAdapter {
        /// Source capture adapter.
        source_adapter: BackupAdapterId,
        /// Target restore adapter.
        target_adapter: BackupAdapterId,
        /// Target adapter transaction guarantee.
        participation: BackupAdapterRestoreParticipation,
    },
    /// Do not migrate this domain.
    Excluded(StorageTransitionExclusion),
}

/// One registered domain in the transition inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionDomain {
    pub(crate) domain: DomainId,
    pub(crate) storage_class: StorageClass,
    pub(crate) source_path: Option<PathBuf>,
    pub(crate) target_path: Option<PathBuf>,
    pub(crate) source_evidence: Option<StorageFileEvidence>,
    pub(crate) target_evidence: Option<StorageFileEvidence>,
    pub(crate) action: StorageTransitionAction,
}

impl StorageTransitionDomain {
    /// Returns the domain id.
    #[must_use]
    pub fn domain(&self) -> &DomainId {
        &self.domain
    }
    /// Returns the registered storage class.
    #[must_use]
    pub const fn storage_class(&self) -> StorageClass {
        self.storage_class
    }
    /// Returns the ordinary source path.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }
    /// Returns the ordinary target path.
    #[must_use]
    pub fn target_path(&self) -> Option<&Path> {
        self.target_path.as_deref()
    }
    /// Returns current source evidence.
    #[must_use]
    pub fn source_evidence(&self) -> Option<&StorageFileEvidence> {
        self.source_evidence.as_ref()
    }
    /// Returns current target evidence.
    #[must_use]
    pub fn target_evidence(&self) -> Option<&StorageFileEvidence> {
        self.target_evidence.as_ref()
    }
    /// Returns planned behavior.
    #[must_use]
    pub fn action(&self) -> &StorageTransitionAction {
        &self.action
    }
}

/// Destination or authority conflict found without mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageTransitionConflictKind {
    /// Source and target root purposes overlap ambiguously.
    OverlappingRoots,
    /// Target already contains different registered state.
    TargetOccupied,
    /// Target contains an unregistered file.
    UnknownTargetFile,
}

/// One conflict that blocks plan creation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionConflict {
    pub(crate) kind: StorageTransitionConflictKind,
    pub(crate) domain: Option<DomainId>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) detail: String,
}

impl StorageTransitionConflict {
    /// Returns stable conflict kind.
    #[must_use]
    pub const fn kind(&self) -> StorageTransitionConflictKind {
        self.kind
    }
    /// Returns affected domain.
    #[must_use]
    pub fn domain(&self) -> Option<&DomainId> {
        self.domain.as_ref()
    }
    /// Returns affected path.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    /// Returns diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Unregistered source or target file preserved by transition policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionUnknownFile {
    pub(crate) root: crate::RootKind,
    pub(crate) path: PathBuf,
    pub(crate) evidence: StorageFileEvidence,
}

impl StorageTransitionUnknownFile {
    /// Returns root purpose.
    #[must_use]
    pub const fn root(&self) -> crate::RootKind {
        self.root
    }
    /// Returns exact path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Returns current evidence.
    #[must_use]
    pub fn evidence(&self) -> &StorageFileEvidence {
        &self.evidence
    }
}

/// Complete non-mutating transition inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionPreview {
    pub(crate) source_layout_digest: Sha256Digest,
    pub(crate) target_layout_digest: Sha256Digest,
    pub(crate) target_selection: StorageProfileSelection,
    pub(crate) domains: Vec<StorageTransitionDomain>,
    pub(crate) source_unknown: Vec<StorageTransitionUnknownFile>,
    pub(crate) target_unknown: Vec<StorageTransitionUnknownFile>,
    pub(crate) conflicts: Vec<StorageTransitionConflict>,
    pub(crate) evidence_digest: Sha256Digest,
    pub(crate) confirmation_digest: Sha256Digest,
}

impl StorageTransitionPreview {
    /// Returns source layout evidence.
    #[must_use]
    pub fn source_layout_digest(&self) -> &Sha256Digest {
        &self.source_layout_digest
    }
    /// Returns target layout evidence.
    #[must_use]
    pub fn target_layout_digest(&self) -> &Sha256Digest {
        &self.target_layout_digest
    }
    /// Returns registered inventory.
    #[must_use]
    pub fn domains(&self) -> &[StorageTransitionDomain] {
        &self.domains
    }
    /// Returns preserved source unknown files.
    #[must_use]
    pub fn source_unknown(&self) -> &[StorageTransitionUnknownFile] {
        &self.source_unknown
    }
    /// Returns target unknown files which block planning.
    #[must_use]
    pub fn target_unknown(&self) -> &[StorageTransitionUnknownFile] {
        &self.target_unknown
    }
    /// Returns blocking conflicts.
    #[must_use]
    pub fn conflicts(&self) -> &[StorageTransitionConflict] {
        &self.conflicts
    }
    /// Returns current inventory evidence digest.
    #[must_use]
    pub fn evidence_digest(&self) -> &Sha256Digest {
        &self.evidence_digest
    }
    /// Returns confirmation bound to layouts and evidence.
    #[must_use]
    pub fn confirmation_digest(&self) -> &Sha256Digest {
        &self.confirmation_digest
    }
}

/// Immutable executable transition plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionPlan {
    pub(crate) preview: StorageTransitionPreview,
}

impl StorageTransitionPlan {
    /// Returns confirmation digest.
    #[must_use]
    pub fn confirmation_digest(&self) -> &Sha256Digest {
        self.preview.confirmation_digest()
    }
    /// Returns planned domains.
    #[must_use]
    pub fn domains(&self) -> &[StorageTransitionDomain] {
        self.preview.domains()
    }
}
