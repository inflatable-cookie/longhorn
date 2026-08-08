use std::path::{Path, PathBuf};

use crate::{BackupKind, Sha256Digest, backup::types::UtcTimestamp};

/// One successfully inspected same-app archive eligible for retention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupOperationalCandidate {
    pub(crate) path: PathBuf,
    pub(crate) archive_id: String,
    pub(crate) created_at: String,
    pub(crate) created_timestamp: UtcTimestamp,
    pub(crate) kind: BackupKind,
    pub(crate) archive_sha256: Sha256Digest,
}

impl BackupOperationalCandidate {
    /// Returns the root-level archive path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the manifest archive id.
    #[must_use]
    pub fn archive_id(&self) -> &str {
        &self.archive_id
    }

    /// Returns the strict manifest UTC creation time.
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Returns the manifest backup kind.
    #[must_use]
    pub const fn kind(&self) -> BackupKind {
        self.kind
    }

    /// Returns SHA-256 over the complete archive.
    #[must_use]
    pub fn archive_sha256(&self) -> &Sha256Digest {
        &self.archive_sha256
    }
}

/// Why one root entry was preserved outside automatic retention.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BackupRetentionDiagnosticKind {
    /// Entry is not a plaintext Longhorn archive.
    Unmanaged,
    /// Encrypted archive cannot be inspected by the plaintext layer.
    Locked,
    /// Candidate could not be read.
    Unreadable,
    /// Candidate is not a regular file.
    NonRegular,
    /// Plaintext archive is malformed or damaged.
    Corrupt,
    /// Archive uses a future or otherwise unsupported format.
    UnknownFormat,
    /// Archive belongs to another application.
    ForeignApplication,
    /// User-export archive was placed in the operational root.
    UserExport,
    /// More than one valid candidate claims the same archive id.
    DuplicateArchiveId,
    /// Root enumeration exceeded its explicit bound.
    ScanLimit,
    /// Reading the root itself failed.
    RootRead,
    /// The just-published archive predates another valid manifest.
    ClockRegression,
    /// A requested pin was not present in the complete listing.
    MissingPin,
    /// The just-published archive was not present in the complete listing.
    MissingNewArchive,
}

/// Non-fatal listing or retention evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionDiagnostic {
    /// Diagnostic class.
    pub kind: BackupRetentionDiagnosticKind,
    /// Affected root entry when one exists.
    pub path: Option<PathBuf>,
    /// Stable human-readable detail.
    pub detail: String,
}

/// Bounded operational-root inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupOperationalListing {
    pub(crate) root: PathBuf,
    pub(crate) candidates: Vec<BackupOperationalCandidate>,
    pub(crate) diagnostics: Vec<BackupRetentionDiagnostic>,
    pub(crate) complete: bool,
}

impl BackupOperationalListing {
    /// Returns the exact operational root that was inspected.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns proven same-app retention candidates in newest-first order.
    #[must_use]
    pub fn candidates(&self) -> &[BackupOperationalCandidate] {
        &self.candidates
    }

    /// Returns preserved-entry and scan diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[BackupRetentionDiagnostic] {
        &self.diagnostics
    }

    /// Reports whether the root was enumerated completely.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }
}

pub(crate) fn diagnostic(
    kind: BackupRetentionDiagnosticKind,
    path: Option<PathBuf>,
    detail: impl Into<String>,
) -> BackupRetentionDiagnostic {
    BackupRetentionDiagnostic {
        kind,
        path,
        detail: detail.into(),
    }
}
