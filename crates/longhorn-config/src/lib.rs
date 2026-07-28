//! Versioned, root-confined configuration domains and atomic local mutation.
//!
//! This crate performs read, validation, in-memory migration, and coordinated
//! atomic patch mutation with optional bounded debounce and explicit flush. It
//! does not repair, back up, or restore files.

mod coordination;
mod debounce;
mod domain;
mod location;
mod registry;
mod store;

pub use coordination::{
    CoordinationAuthority, CoordinationAuthorityError, CoordinationFailure, CoordinationFailureKind,
};
pub use debounce::{
    DebounceClock, DebounceFlushSet, DebouncePolicy, DebouncePolicyError, DebounceSnapshot,
    DebounceStrategy, DebounceTerminal, DebouncedMutation, FlushOutcome, FlushSetError,
    PendingSnapshot, RetryDisposition, StageDisposition, StageError, StageReceipt, SystemClock,
};
pub use domain::{
    ConfigDomain, DomainDescriptor, DomainDescriptorError, DomainFilePath, DomainFilePathError,
    DomainIssue, MigrationStep, StorageClass,
};
pub use location::{
    AccessMode, DomainLocation, ResolvedFile, RootKind, StorageRootError, StorageRoots,
};
pub use registry::RegistrationError;
pub use store::{
    ConfigStore, Durability, DurabilityRequirement, LoadDiagnostic, LoadDiagnosticCode,
    LoadOutcome, LoadedConfig, LoadedOrigin, MutationError, MutationOptions, MutationReceipt,
    MutationRefusal, PublicationFailure, PublicationStage, RecoveryKind, RecoveryState,
    SourceDocument, StoreError, UnavailableState,
};
