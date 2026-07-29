//! Registered configuration persistence for authoritative layout documents.
//!
//! The consumer supplies the exact domain descriptor, default document,
//! definition registry, migration hook, and backup participation. This crate
//! adds no storage scope, filename, host, or renderer policy.

mod debounce;
mod digest;
mod domain;
mod migration;
mod mutation;

pub use debounce::{
    LayoutPresentationIntent, LayoutPresentationIntentError, LayoutPresentationStrategy,
};
pub use digest::{LayoutRegistryDigest, compute_layout_registry_digest};
pub use domain::{
    LayoutBackupPolicy, PersistedLayoutDocument, RegisteredLayoutDomain,
    RegisteredLayoutDomainError,
};
pub use migration::{LayoutMigration, LayoutMigrationTarget, NoLayoutMigration};
pub use mutation::{
    LayoutConfigMutationError, LayoutConfigPublicationReceipt, publish_layout_mutation,
};
