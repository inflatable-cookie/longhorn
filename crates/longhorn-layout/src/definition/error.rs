use std::{error::Error, fmt};

use longhorn_core::{LayoutSchemaId, PanelDefinitionId, RegionId};

/// Stable category for invalid definition-registry input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionErrorCode {
    /// No layout schema was supplied.
    EmptyRegistry,
    /// Registered schema count exceeded configured limits.
    TooManySchemas,
    /// Registered panel-definition count exceeded configured limits.
    TooManyPanelDefinitions,
    /// A schema id appeared more than once.
    DuplicateSchema,
    /// A schema contained no semantic region.
    EmptySchema,
    /// Region count exceeded configured limits.
    TooManyRegions,
    /// A region id appeared more than once in one schema.
    DuplicateRegion,
    /// Two regions declared the same stable order.
    DuplicateRegionOrder,
    /// Sizing-slot count exceeded configured limits.
    TooManySizingSlots,
    /// A sizing-slot id appeared more than once in one schema.
    DuplicateSizingSlot,
    /// Two sizing slots declared the same stable order.
    DuplicateSizingSlotOrder,
    /// Sizing minimum, default, and maximum were not ordered.
    InvalidSizingBounds,
    /// A panel definition id appeared more than once.
    DuplicatePanelDefinition,
    /// A panel definition had no default placement.
    EmptyDefaultPlacement,
    /// A panel definition allowed no placement.
    EmptyAllowedPlacement,
    /// A default selector was repeated.
    DuplicateDefaultPlacement,
    /// An allowed selector was repeated.
    DuplicateAllowedPlacement,
    /// A default selector matched no registered region.
    UnknownDefaultSelector,
    /// An allowed selector matched no registered region.
    UnknownAllowedSelector,
    /// A default selector resolved outside allowed placement.
    DefaultPlacementNotAllowed,
    /// Bounded instance policy used zero or inverted limits.
    InvalidInstanceLimit,
}

/// Invalid layout definition registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionRegistryError {
    pub(super) code: DefinitionErrorCode,
    pub(super) detail: String,
}

impl DefinitionRegistryError {
    /// Returns the stable error category.
    #[must_use]
    pub const fn code(&self) -> DefinitionErrorCode {
        self.code
    }

    /// Returns the bounded human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for DefinitionRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for DefinitionRegistryError {}

pub(super) fn definition_error(
    code: DefinitionErrorCode,
    detail: impl Into<String>,
) -> DefinitionRegistryError {
    DefinitionRegistryError {
        code,
        detail: detail.into(),
    }
}

/// Failed lookup in a validated definition registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefinitionLookupError {
    /// Requested schema is not registered.
    UnknownSchema(LayoutSchemaId),
    /// Requested panel definition is not registered.
    UnknownPanelDefinition(PanelDefinitionId),
    /// Requested region is not part of the selected schema.
    UnknownRegion(RegionId),
}

impl fmt::Display for DefinitionLookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSchema(id) => write!(formatter, "unknown layout schema {id}"),
            Self::UnknownPanelDefinition(id) => {
                write!(formatter, "unknown panel definition {id}")
            }
            Self::UnknownRegion(id) => write!(formatter, "unknown region {id}"),
        }
    }
}

impl Error for DefinitionLookupError {}
