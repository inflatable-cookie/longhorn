//! Registered configuration persistence for authoritative Surface documents.

mod domain;
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
