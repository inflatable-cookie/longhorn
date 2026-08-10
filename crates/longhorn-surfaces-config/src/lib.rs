//! Registered configuration persistence for authoritative Surface documents.

mod domain;
mod layout_debounce;
mod layout_digest;
mod layout_domain;
mod layout_migration;
mod layout_mutation;
mod migration;
mod mutation;

pub use domain::{
    PersistedSurfaceDocument, RegisteredSurfaceDomain, RegisteredSurfaceDomainError,
    SurfaceBackupPolicy,
};
pub use migration::{NoSurfaceMigration, SurfaceMigration, SurfaceMigrationTarget};
pub use mutation::{
    SurfaceConfigMutationError, SurfaceConfigPublicationReceipt, publish_surface_mutation,
};

// Absorbed from the former `longhorn-layout-config` by Card 179. One document
// means one config crate; the layout-side domain descriptor, migration hook and
// publication path persist the same SurfaceDocument the Surface side does.
pub use layout_debounce::{
    LayoutPresentationIntent, LayoutPresentationIntentError, LayoutPresentationStrategy,
};
pub use layout_digest::{LayoutRegistryDigest, compute_layout_registry_digest};
pub use layout_domain::{
    LayoutBackupPolicy, PersistedLayoutDocument, RegisteredLayoutDomain,
    RegisteredLayoutDomainError,
};
pub use layout_migration::{LayoutMigration, LayoutMigrationTarget, NoLayoutMigration};
pub use layout_mutation::{
    LayoutConfigMutationError, LayoutConfigPublicationReceipt, publish_layout_mutation,
};
