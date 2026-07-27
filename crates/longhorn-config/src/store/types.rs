use std::{error::Error, fmt, path::PathBuf};

use longhorn_core::{DomainId, SchemaVersion};

use crate::DomainLocation;

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
