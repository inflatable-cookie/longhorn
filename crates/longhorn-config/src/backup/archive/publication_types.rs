use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use crate::{Durability, DurabilityRequirement, Sha256Digest};

use super::{BackupArchiveError, BackupArchiveLimits};

/// Portable `.longhorn-backup` file name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackupArchiveFileName(String);

impl BackupArchiveFileName {
    /// Validates one portable archive file name.
    pub fn new(value: impl Into<String>) -> Result<Self, BackupArchiveFileNameError> {
        let value = value.into();
        if value.len() > 255 {
            return Err(BackupArchiveFileNameError::TooLong);
        }
        if !value.ends_with(".longhorn-backup") {
            return Err(BackupArchiveFileNameError::Extension);
        }
        if value.starts_with('.')
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(BackupArchiveFileNameError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    /// Returns the validated file name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Invalid portable archive file name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupArchiveFileNameError {
    /// File name exceeds the portable component bound.
    TooLong,
    /// File name does not use `.longhorn-backup`.
    Extension,
    /// File name contains unsafe or non-portable characters.
    InvalidCharacter,
}

impl fmt::Display for BackupArchiveFileNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => formatter.write_str("backup archive file name is too long"),
            Self::Extension => {
                formatter.write_str("backup archive file name must end with .longhorn-backup")
            }
            Self::InvalidCharacter => {
                formatter.write_str("backup archive file name is not a portable path component")
            }
        }
    }
}

impl Error for BackupArchiveFileNameError {}

/// Injected operational backup directory authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupOperationalRoot(PathBuf);

impl BackupOperationalRoot {
    /// Accepts an absolute application-owned operational backup root.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, BackupPublicationError> {
        let path = path.into();
        if !path.is_absolute() {
            return Err(invalid_destination(
                path,
                "operational backup root must be absolute",
            ));
        }
        Ok(Self(path))
    }

    /// Returns the injected root path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Explicit user-selected export target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupExportTarget {
    parent: PathBuf,
    file_name: BackupArchiveFileName,
}

impl BackupExportTarget {
    /// Constructs an export below an existing absolute user-selected parent.
    pub fn new(
        parent: impl Into<PathBuf>,
        file_name: BackupArchiveFileName,
    ) -> Result<Self, BackupPublicationError> {
        let parent = parent.into();
        if !parent.is_absolute() {
            return Err(invalid_destination(
                parent,
                "export parent must be absolute",
            ));
        }
        Ok(Self { parent, file_name })
    }

    /// Returns the selected parent.
    #[must_use]
    pub fn parent(&self) -> &Path {
        &self.parent
    }

    /// Returns the selected portable file name.
    #[must_use]
    pub fn file_name(&self) -> &BackupArchiveFileName {
        &self.file_name
    }

    pub(super) fn path(&self) -> PathBuf {
        self.parent.join(self.file_name.as_str())
    }
}

/// Explicit authority for replacing an existing export.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportOverwrite {
    /// Reject an existing destination.
    Refuse,
    /// Atomically replace an existing destination.
    Replace,
}

/// Archive destination class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupDestinationKind {
    /// Application-owned operational retention root.
    Operational,
    /// Explicit user-selected export destination.
    UserExport,
}

/// Publication durability and verification policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackupPublicationOptions {
    /// Required filesystem durability.
    pub durability: DurabilityRequirement,
    /// Archive verification safety bounds.
    pub archive_limits: BackupArchiveLimits,
}

impl BackupPublicationOptions {
    /// Constructs explicit publication policy.
    #[must_use]
    pub const fn new(
        durability: DurabilityRequirement,
        archive_limits: BackupArchiveLimits,
    ) -> Self {
        Self {
            durability,
            archive_limits,
        }
    }
}

/// Successful verified archive publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPublicationReceipt {
    /// Published target.
    pub path: PathBuf,
    /// Destination class.
    pub destination: BackupDestinationKind,
    /// SHA-256 over exact published archive bytes.
    pub archive_sha256: Sha256Digest,
    /// Established filesystem durability.
    pub durability: Durability,
    /// Whether an existing authorized export was replaced.
    pub replaced_existing: bool,
}

/// Archive publication phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupPublicationStage {
    /// Validate root and overwrite authority.
    ValidateDestination,
    /// Create the operational root when absent.
    CreateRoot,
    /// Open the destination parent.
    OpenParent,
    /// Create a unique private sibling partial.
    CreateTemporary,
    /// Write archive bytes.
    WriteTemporary,
    /// Sync archive bytes.
    SyncTemporary,
    /// Reopen and inspect staged bytes.
    VerifyTemporary,
    /// Atomically rename the staged archive.
    Rename,
    /// Sync the destination directory.
    SyncDirectory,
}

/// Typed staged archive publication failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPublicationError {
    /// Failed publication phase.
    pub stage: BackupPublicationStage,
    /// Intended target or affected root.
    pub path: PathBuf,
    /// Whether rename already made the archive visible.
    pub published: bool,
    /// Machine-readable archive verification failure when applicable.
    pub verification: Option<BackupArchiveError>,
    /// Human-readable I/O or policy detail.
    pub detail: String,
}

impl fmt::Display for BackupPublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "backup publication {:?} failed for {}: {}",
            self.stage,
            self.path.display(),
            self.detail
        )
    }
}

impl Error for BackupPublicationError {}

fn invalid_destination(path: PathBuf, detail: impl Into<String>) -> BackupPublicationError {
    BackupPublicationError {
        stage: BackupPublicationStage::ValidateDestination,
        path,
        published: false,
        verification: None,
        detail: detail.into(),
    }
}
