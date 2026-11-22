//! Pure optional Surface identity, topology, validation, lifecycle, and host resolution.

/// Compatibility version of the serialized Surface protocol.
pub const SURFACE_PROTOCOL_VERSION: u32 = 1;

pub mod layout;
mod limits;
mod model;
mod mutation;
mod resolution;
mod snapshot;
mod validation;

pub use limits::{SurfaceLimits, SurfaceLimitsBuilder, SurfaceLimitsError};
pub use model::{
    ParticipatingWindow, SurfaceDocument, SurfaceHostPreference, SurfacePresentation, SurfaceRecord,
};
pub use mutation::{
    EmptyWindowPolicy, SurfaceMutationCommand, SurfaceMutationEngine, SurfaceMutationOutcome,
    SurfaceMutationReceipt, SurfaceMutationRejection, SurfaceMutationRejectionCode,
    SurfaceMutationRequest, SurfaceMutationResponse,
};
pub use resolution::{
    ResolvedSurface, ResolvedSurfaceWindow, SurfaceResolution, SurfaceResolutionError,
    SurfaceResolutionErrorCode, SurfaceResolutionInput, SurfaceUnresolvedReason, UnresolvedSurface,
    resolve_surfaces,
};
pub use snapshot::{SurfaceChangedEvent, SurfaceProtocolEpoch, SurfaceSnapshot};
pub use validation::{
    SurfaceValidationCode, SurfaceValidationError, normalize_document, validate_document,
    validate_normalized_document,
};

// Former `longhorn-layout` vocabulary, re-exported at the crate root. Card 179.
pub use layout::LAYOUT_PROTOCOL_VERSION;
pub use layout::definition::{
    DefinitionErrorCode, DefinitionLookupError, DefinitionRegistryError, EmptyRegionPolicy,
    LayoutDefinitionRegistry, LayoutSchemaDefinition, PanelDefinition, PanelInstancePolicy,
    PlacementSelector, RegionDefinition, SizingSlotDefinition,
};
pub use layout::model::{PanelInstance, RegionState, SizingSlotState};
pub use layout::mutation::{
    BoundedLayoutReplayStore, LayoutMutationCommand, LayoutMutationEngine, LayoutMutationOutcome,
    LayoutMutationReceipt, LayoutMutationRejection, LayoutMutationRejectionCode,
    LayoutMutationRequest, LayoutReplayStoreError,
};
pub use layout::ratio::{LayoutRatio, LayoutRatioError, RATIO_ONE_MILLIONTHS};
pub use layout::visibility::{
    RegionVisibility, RegionVisibilityState, VisibilityProjectionError, project_region_visibility,
};
