//! Versioned, root-confined configuration domain loading.
//!
//! This crate currently performs read, validation, and in-memory migration
//! only. It does not write, repair, back up, or restore files.

mod domain;
mod location;
mod registry;
mod store;

pub use domain::{
    ConfigDomain, DomainDescriptor, DomainDescriptorError, DomainFilePath, DomainFilePathError,
    DomainIssue, MigrationStep, StorageClass,
};
pub use location::{
    AccessMode, DomainLocation, ResolvedFile, RootKind, StorageRootError, StorageRoots,
};
pub use registry::RegistrationError;
pub use store::{
    ConfigStore, LoadDiagnostic, LoadDiagnosticCode, LoadOutcome, LoadedConfig, LoadedOrigin,
    RecoveryKind, RecoveryState, SourceDocument, StoreError, UnavailableState,
};
