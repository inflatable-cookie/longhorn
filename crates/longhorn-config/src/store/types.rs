use std::{error::Error, fmt, path::PathBuf, time::Duration};

use longhorn_core::{DomainId, SchemaVersion};

use crate::{CoordinationFailure, DomainIssue, DomainLocation};

/// Result of loading a registered configuration domain.
#[derive(Debug, PartialEq)]
pub enum LoadOutcome<T> {
    /// A validated value is ready.
    Ready(LoadedConfig<T>),
    /// Source data was preserved for explicit recovery.
    Recovery(RecoveryState),
    /// Another authority or explicit root is required.
    Unavailable(UnavailableState),
}

/// Validated loaded configuration.
#[derive(Debug, PartialEq)]
pub struct LoadedConfig<T> {
    /// Decoded value.
    pub value: T,
    /// Schema version of the returned value.
    pub schema_version: SchemaVersion,
    /// Source and migration state.
    pub origin: LoadedOrigin,
    /// Non-fatal load diagnostics.
    pub diagnostics: Vec<LoadDiagnostic>,
    /// Original file bytes when a file was read.
    pub source: Option<SourceDocument>,
}

/// Source of a ready configuration value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadedOrigin {
    /// Compiled default.
    Default,
    /// Current on-disk document.
    File,
    /// Older document migrated without rewriting its source.
    MigratedInMemory {
        /// Original schema.
        from: SchemaVersion,
        /// Returned schema.
        to: SchemaVersion,
    },
}

/// Non-fatal load diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadDiagnostic {
    /// Stable diagnostic code.
    pub code: LoadDiagnosticCode,
    /// Human-readable diagnostic.
    pub message: String,
}

/// Stable non-fatal diagnostic code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadDiagnosticCode {
    /// The ordinary domain file did not exist.
    Missing,
}

/// Original file material preserved by a load.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDocument {
    /// Diagnostic source path.
    pub path: PathBuf,
    /// Exact source bytes.
    pub bytes: Vec<u8>,
}

/// Explicit recovery state for unreadable or invalid source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryState {
    /// Stable recovery category.
    pub kind: RecoveryKind,
    /// Diagnostic source path, when file-backed.
    pub path: Option<PathBuf>,
    /// Exact source bytes, when the file could be read.
    pub source: Option<SourceDocument>,
    /// Human-readable detail.
    pub detail: String,
}

/// Stable recovery category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryKind {
    /// A capability-scoped read failed.
    ReadFailed,
    /// The compiled default failed validation.
    InvalidDefault,
    /// The JSON envelope was malformed.
    CorruptDocument,
    /// The envelope named another domain.
    DomainMismatch,
    /// The document schema is newer than this package supports.
    FutureSchema,
    /// Raw or decoded data failed domain validation.
    InvalidValue,
    /// The domain did not supply the next migration.
    MissingMigration,
    /// A migration returned the wrong target version.
    InvalidMigrationStep,
    /// Consumer migration code failed.
    MigrationFailed,
    /// Current raw JSON could not be decoded.
    DecodeFailed,
}

/// A load requiring another storage authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnavailableState {
    /// Required location or authority.
    pub location: DomainLocation,
}

/// Store use that violates registration authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// The domain was not registered.
    NotRegistered {
        /// Requested domain.
        id: DomainId,
    },
    /// The caller changed a descriptor after registration.
    DescriptorChanged {
        /// Changed domain.
        id: DomainId,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRegistered { id } => write!(formatter, "domain {id} is not registered"),
            Self::DescriptorChanged { id } => {
                write!(formatter, "domain {id} changed after registration")
            }
        }
    }
}

impl Error for StoreError {}

/// Required lock wait and publication durability for one mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MutationOptions {
    /// Maximum time spent acquiring process and file locks.
    pub lock_timeout: Duration,
    /// Minimum accepted publication durability.
    pub durability: DurabilityRequirement,
}

impl MutationOptions {
    /// Constructs explicit finite mutation options.
    #[must_use]
    pub const fn new(lock_timeout: Duration, durability: DurabilityRequirement) -> Self {
        Self {
            lock_timeout,
            durability,
        }
    }
}

/// Minimum durability accepted by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurabilityRequirement {
    /// Require atomic old-or-new visibility and a synced file.
    Atomic,
    /// Also require verified parent-directory synchronization.
    Durable,
}

/// Durability established for a published mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Durability {
    /// The replacement is atomic and the new file was synced.
    FileSynced,
    /// The file and containing directory were synced.
    FileAndDirectorySynced,
}

/// Successful configuration mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationReceipt {
    /// Mutated domain.
    pub domain: DomainId,
    /// Published domain file.
    pub path: PathBuf,
    /// Published schema version.
    pub schema_version: SchemaVersion,
    /// Durability established by publication.
    pub durability: Durability,
}

/// Safe refusal to mutate a load outcome or authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationRefusal {
    /// The domain has no ordinary writable file.
    Unavailable {
        /// Resolved non-file authority.
        location: DomainLocation,
    },
    /// The registered file is read-only.
    ReadOnly {
        /// Read-only file.
        path: PathBuf,
    },
    /// Project-shared mutation needs a separately proven authority.
    ProjectSharedRequiresExternalAuthority {
        /// Project-shared file.
        path: PathBuf,
    },
    /// Source is invalid and was preserved for recovery.
    Recovery(RecoveryState),
    /// Destructive migration rewrite requires the later backup batch.
    MigrationBackupRequired {
        /// Original schema.
        from: SchemaVersion,
        /// Current schema.
        to: SchemaVersion,
    },
}

/// Filesystem publication phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationStage {
    /// Open the registered root capability.
    OpenRoot,
    /// Create the registered target parent.
    CreateParent,
    /// Open the target parent capability.
    OpenParent,
    /// Exclusively create a unique temporary file.
    CreateTemporary,
    /// Write the encoded envelope.
    WriteTemporary,
    /// Sync the encoded temporary file.
    SyncTemporary,
    /// Atomically replace the target.
    Rename,
    /// Sync the target parent directory.
    SyncDirectory,
}

/// Typed atomic-publication failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationFailure {
    /// Failed phase.
    pub stage: PublicationStage,
    /// Intended target.
    pub path: PathBuf,
    /// Whether atomic replacement already succeeded.
    pub published: bool,
    /// Human-readable detail, including cleanup failure when relevant.
    pub detail: String,
}

/// Configuration mutation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationError {
    /// Store registration authority failed.
    Store(StoreError),
    /// The mutation coordinator could not be acquired.
    Coordination(CoordinationFailure),
    /// The current location or load outcome is not safely writable.
    Refused(MutationRefusal),
    /// Consumer patch code rejected the change.
    Patch(DomainIssue),
    /// The patched typed value failed validation.
    Validation(DomainIssue),
    /// The domain codec could not encode the current value.
    Encode(DomainIssue),
    /// Encoded current-version JSON failed raw validation.
    EncodedValueInvalid(DomainIssue),
    /// The versioned envelope could not be serialized.
    Serialization {
        /// Serializer detail.
        detail: String,
    },
    /// Atomic publication failed.
    Publication(PublicationFailure),
}

impl fmt::Display for MutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => error.fmt(formatter),
            Self::Coordination(error) => error.fmt(formatter),
            Self::Refused(refusal) => write!(formatter, "mutation refused: {refusal:?}"),
            Self::Patch(issue) => write!(formatter, "patch failed: {}", issue.message),
            Self::Validation(issue) => {
                write!(formatter, "patched value is invalid: {}", issue.message)
            }
            Self::Encode(issue) => write!(formatter, "encode failed: {}", issue.message),
            Self::EncodedValueInvalid(issue) => {
                write!(formatter, "encoded value is invalid: {}", issue.message)
            }
            Self::Serialization { detail } => {
                write!(formatter, "cannot serialize domain envelope: {detail}")
            }
            Self::Publication(failure) => write!(
                formatter,
                "publication failed at {:?} for {}: {}",
                failure.stage,
                failure.path.display(),
                failure.detail
            ),
        }
    }
}

impl Error for MutationError {}
