//! Pure Surface-independent layout definitions, state, normalization, and visibility.

/// Compatibility version of the serialized layout protocol.
pub const LAYOUT_PROTOCOL_VERSION: u32 = 1;

mod definition;
mod limits;
mod model;
mod mutation;
mod ratio;
mod validation;
mod visibility;

pub use definition::{
    DefinitionErrorCode, DefinitionLookupError, DefinitionRegistryError, EmptyRegionPolicy,
    LayoutDefinitionRegistry, LayoutSchemaDefinition, PanelDefinition, PanelInstancePolicy,
    PlacementSelector, RegionDefinition, SizingSlotDefinition,
};
pub use limits::{LayoutLimits, LayoutLimitsError};
pub use model::{LayoutContainer, LayoutDocument, PanelInstance, RegionState, SizingSlotState};
pub use mutation::{
    BoundedLayoutReplayStore, LayoutMutationCommand, LayoutMutationEngine, LayoutMutationOutcome,
    LayoutMutationReceipt, LayoutMutationRejection, LayoutMutationRejectionCode,
    LayoutMutationRequest, LayoutReplayStoreError,
};
pub use ratio::{LayoutRatio, LayoutRatioError, RATIO_ONE_MILLIONTHS};
pub use validation::{
    LayoutValidationCode, LayoutValidationError, normalize_document, validate_document,
    validate_normalized_document,
};
pub use visibility::{
    RegionVisibility, RegionVisibilityState, VisibilityProjectionError, project_region_visibility,
};
