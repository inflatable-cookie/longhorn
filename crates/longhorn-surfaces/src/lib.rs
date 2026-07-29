//! Pure optional Surface identity, topology, validation, lifecycle, and host resolution.

/// Compatibility version of the serialized Surface protocol.
pub const SURFACE_PROTOCOL_VERSION: u32 = 1;

mod limits;
mod model;
mod mutation;
mod resolution;
mod validation;

pub use limits::{SurfaceLimits, SurfaceLimitsError};
pub use model::{ParticipatingWindow, SurfaceDocument, SurfaceHostPreference, SurfaceRecord};
pub use mutation::{
    EmptyWindowPolicy, LayoutContainerCleanupIntent, LayoutContainerInventory,
    SurfaceMutationCommand, SurfaceMutationEngine, SurfaceMutationOutcome, SurfaceMutationReceipt,
    SurfaceMutationRejection, SurfaceMutationRejectionCode, SurfaceMutationRequest,
};
pub use resolution::{
    ResolvedSurface, ResolvedSurfaceWindow, SurfaceResolution, SurfaceResolutionError,
    SurfaceResolutionErrorCode, SurfaceResolutionInput, SurfaceUnresolvedReason, UnresolvedSurface,
    resolve_surfaces,
};
pub use validation::{
    SurfaceValidationCode, SurfaceValidationError, normalize_document, validate_document,
    validate_normalized_document,
};
