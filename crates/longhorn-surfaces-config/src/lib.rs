//! Registered configuration persistence for authoritative Surface documents.

mod domain;
mod layout_debounce;
mod layout_digest;
mod migration;
mod mutation;
mod recovery;

pub use domain::{
    PersistedSurfaceDocument, RegisteredSurfaceDomain, RegisteredSurfaceDomainError,
    SurfaceBackupPolicy,
};
pub use layout_debounce::{
    LayoutPresentationIntent, LayoutPresentationIntentError, LayoutPresentationRequest,
    LayoutPresentationStrategy, debounced_layout_presentation,
};
pub use layout_digest::{LayoutRegistryDigest, compute_layout_registry_digest};
pub use migration::{
    LegacySurfaceFormatMigration, NoSurfaceMigration, SurfaceMigration, SurfaceMigrationTarget,
};
pub use mutation::{
    LayoutConfigMutationError, LayoutConfigPublicationReceipt, SurfaceConfigMutationError,
    SurfaceConfigPublicationReceipt, publish_layout_mutation, publish_surface_mutation,
};
pub use recovery::{LayoutFallback, LayoutLoadError, load_or_default};
