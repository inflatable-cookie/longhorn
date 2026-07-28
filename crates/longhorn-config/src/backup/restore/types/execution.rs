use std::{error::Error, fmt, path::PathBuf, time::Duration};

use longhorn_core::{DomainId, SchemaVersion};

use crate::{
    BackupArchiveFileName, BackupArchiveLimits, BackupCaptureOptions, BackupMetadata,
    BackupOperationalRoot, BackupPublicationReceipt, RecoveryState, Sha256Digest, StoreError,
    UnavailableState,
};

/// Finite coordinator wait and required pre-migration safety backup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationRewriteOptions {
    /// Maximum time spent acquiring the store coordinator.
    pub lock_timeout: Duration,
    /// Verified plaintext pre-migration safety backup.
    pub safety_backup: RestoreSafetyBackupOptions,
}

impl MigrationRewriteOptions {
    /// Constructs explicit destructive migration policy.
    #[must_use]
    pub const fn new(lock_timeout: Duration, safety_backup: RestoreSafetyBackupOptions) -> Self {
        Self {
            lock_timeout,
            safety_backup,
        }
    }
}

/// Successful verified destructive migration rewrite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationRewriteReceipt {
    pub(crate) domain: DomainId,
    pub(crate) from: SchemaVersion,
    pub(crate) to: SchemaVersion,
    pub(crate) safety_backup: BackupPublicationReceipt,
}

impl MigrationRewriteReceipt {
    /// Returns the rewritten domain.
    #[must_use]
    pub fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the original on-disk schema.
    #[must_use]
    pub const fn from(&self) -> SchemaVersion {
        self.from
    }

    /// Returns the durable current schema.
    #[must_use]
    pub const fn to(&self) -> SchemaVersion {
        self.to
    }

    /// Returns verified pre-migration backup publication evidence.
    #[must_use]
    pub fn safety_backup(&self) -> &BackupPublicationReceipt {
        &self.safety_backup
    }
}

/// Failure before or during destructive migration rewrite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationRewriteError {
    /// Store registration authority failed.
    Store(StoreError),
    /// Current source cannot be safely rewritten.
    Recovery(RecoveryState),
    /// Current storage authority is unavailable.
    Unavailable(UnavailableState),
    /// Current source is absent or already current.
    NotRequired,
    /// Current-schema encoding or validation failed.
    Preparation(String),
    /// Safety backup, journal, publication, verification, or rollback failed.
    Execution(RestoreExecutionError),
}

impl fmt::Display for MigrationRewriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => error.fmt(formatter),
            Self::Recovery(recovery) => {
                write!(
                    formatter,
                    "migration source requires recovery: {}",
                    recovery.detail
                )
            }
            Self::Unavailable(unavailable) => {
                write!(
                    formatter,
                    "migration source is unavailable: {unavailable:?}"
                )
            }
            Self::NotRequired => formatter.write_str("destructive migration is not required"),
            Self::Preparation(detail) => {
                write!(formatter, "cannot prepare migration rewrite: {detail}")
            }
            Self::Execution(error) => error.fmt(formatter),
        }
    }
}

impl Error for MigrationRewriteError {}

/// Verified operational safety-backup policy for one destructive operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreSafetyBackupOptions {
    pub(crate) metadata: BackupMetadata,
    pub(crate) root: BackupOperationalRoot,
    pub(crate) file_name: BackupArchiveFileName,
    pub(crate) capture: BackupCaptureOptions,
    pub(crate) archive_limits: BackupArchiveLimits,
}

impl RestoreSafetyBackupOptions {
    /// Constructs a safety-backup destination and bounded capture policy.
    #[must_use]
    pub const fn new(
        metadata: BackupMetadata,
        root: BackupOperationalRoot,
        file_name: BackupArchiveFileName,
        capture: BackupCaptureOptions,
        archive_limits: BackupArchiveLimits,
    ) -> Self {
        Self {
            metadata,
            root,
            file_name,
            capture,
            archive_limits,
        }
    }

    /// Returns safety-backup metadata.
    #[must_use]
    pub fn metadata(&self) -> &BackupMetadata {
        &self.metadata
    }

    /// Returns the operational backup root.
    #[must_use]
    pub fn root(&self) -> &BackupOperationalRoot {
        &self.root
    }

    /// Returns the portable safety-backup file name.
    #[must_use]
    pub fn file_name(&self) -> &BackupArchiveFileName {
        &self.file_name
    }
}

/// Finite coordinator wait and required safety backup for live restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreExecutionOptions {
    /// Maximum time spent acquiring the store coordinator.
    pub lock_timeout: Duration,
    /// Verified plaintext pre-restore safety backup.
    pub safety_backup: RestoreSafetyBackupOptions,
}

impl RestoreExecutionOptions {
    /// Constructs explicit live restore policy.
    #[must_use]
    pub const fn new(lock_timeout: Duration, safety_backup: RestoreSafetyBackupOptions) -> Self {
        Self {
            lock_timeout,
            safety_backup,
        }
    }
}

/// Durable restore transaction phase that failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreExecutionStage {
    /// Safety metadata did not describe a pre-restore backup.
    ValidateSafetyBackup,
    /// An older unresolved restore could not be recovered first.
    RecoverPrevious,
    /// Current evidence changed after staging.
    RecheckCurrent,
    /// Exact current rollback bytes could not be captured.
    CaptureRollback,
    /// Safety snapshot capture failed.
    CaptureSafetyBackup,
    /// Safety archive encoding failed.
    EncodeSafetyBackup,
    /// Safety archive publication or verification failed.
    PublishSafetyBackup,
    /// Durable rollback material or journal publication failed.
    PublishJournal,
    /// One atomic live-file replacement or deletion failed.
    PublishTarget,
    /// Complete staged-target verification failed.
    VerifyTarget,
    /// Exact rollback publication or verification failed.
    Rollback,
    /// Terminal journal cleanup failed.
    Cleanup,
}

/// Terminal state proven before a failed execution returns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreFailureTerminal {
    /// No live target was touched.
    NoLiveMutation,
    /// Every selected current source was restored and verified.
    RolledBack,
    /// Exact rollback could not be durably established and writes remain blocked.
    RecoveryRequired,
}

/// Failure from journaled live restore execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreExecutionError {
    /// Failed transaction phase.
    pub stage: RestoreExecutionStage,
    /// Affected domain when the failure is domain-specific.
    pub domain: Option<DomainId>,
    /// Terminal state proven before return.
    pub terminal: RestoreFailureTerminal,
    /// Stable human-readable detail.
    pub detail: String,
}

impl fmt::Display for RestoreExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "restore {:?} failed with {:?}: {}",
            self.stage, self.terminal, self.detail
        )
    }
}

impl Error for RestoreExecutionError {}

/// Successful fully verified live restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreExecutionReceipt {
    pub(crate) plan_digest: Sha256Digest,
    pub(crate) safety_backup: BackupPublicationReceipt,
    pub(crate) restored: Vec<DomainId>,
    pub(crate) deleted: Vec<DomainId>,
    pub(crate) migrated: Vec<DomainId>,
    pub(crate) unchanged: Vec<DomainId>,
}

impl RestoreExecutionReceipt {
    /// Returns the confirmed plan digest.
    #[must_use]
    pub fn plan_digest(&self) -> &Sha256Digest {
        &self.plan_digest
    }

    /// Returns verified pre-restore backup publication evidence.
    #[must_use]
    pub fn safety_backup(&self) -> &BackupPublicationReceipt {
        &self.safety_backup
    }

    /// Returns domains whose staged documents were published.
    #[must_use]
    pub fn restored(&self) -> &[DomainId] {
        &self.restored
    }

    /// Returns domains deleted to reproduce archive absence.
    #[must_use]
    pub fn deleted(&self) -> &[DomainId] {
        &self.deleted
    }

    /// Returns restored domains that required archive migration.
    #[must_use]
    pub fn migrated(&self) -> &[DomainId] {
        &self.migrated
    }

    /// Returns selected domains requiring no publication.
    #[must_use]
    pub fn unchanged(&self) -> &[DomainId] {
        &self.unchanged
    }
}

/// Durable restore-journal state visible to loads and retention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreOperationState {
    /// No journal blocks ordinary work.
    Inactive,
    /// A live transaction or recoverable interrupted transaction exists.
    Active,
    /// A rollback was not verified; normal writes remain blocked.
    RecoveryRequired,
}

/// Finite coordinator wait for explicit startup recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestoreRecoveryOptions {
    /// Maximum time spent acquiring the store coordinator.
    pub lock_timeout: Duration,
}

impl RestoreRecoveryOptions {
    /// Constructs explicit recovery policy.
    #[must_use]
    pub const fn new(lock_timeout: Duration) -> Self {
        Self { lock_timeout }
    }
}

/// Completed startup or operation recovery result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreRecoveryOutcome {
    /// No journal existed.
    NoRecoveryNeeded,
    /// An interrupted destructive operation was rolled back and verified.
    RolledBack,
    /// A previously verified terminal journal was cleaned.
    TerminalCleanup,
}

/// Machine-readable recovery receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreRecoveryReceipt {
    pub(crate) outcome: RestoreRecoveryOutcome,
    pub(crate) domains: Vec<DomainId>,
}

impl RestoreRecoveryReceipt {
    /// Returns the terminal recovery result.
    #[must_use]
    pub const fn outcome(&self) -> RestoreRecoveryOutcome {
        self.outcome
    }

    /// Returns journal domains considered by recovery.
    #[must_use]
    pub fn domains(&self) -> &[DomainId] {
        &self.domains
    }
}

/// Failure to verify rollback from a durable restore journal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreRecoveryError {
    /// Journal or rollback path involved.
    pub path: PathBuf,
    /// First affected domain when known.
    pub domain: Option<DomainId>,
    /// Human-readable refusal or I/O detail.
    pub detail: String,
}

impl fmt::Display for RestoreRecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "restore recovery failed at {}: {}",
            self.path.display(),
            self.detail
        )
    }
}

impl Error for RestoreRecoveryError {}
