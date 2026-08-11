use std::{error::Error, fmt};

/// Stable category for invalid current layout state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutValidationCode {
    /// Container count exceeded configured limits.
    TooManyContainers,
    /// Panel-instance count exceeded configured limits.
    TooManyPanelInstances,
    /// A surface id appeared more than once.
    DuplicateContainer,
    /// A panel-instance id appeared more than once.
    DuplicatePanelInstance,
    /// A surface referenced an unknown schema.
    UnknownSchema,
    /// A panel instance referenced an unknown definition.
    UnknownPanelDefinition,
    /// A region id appeared more than once in one surface.
    DuplicateRegionState,
    /// A region was not part of its surface schema.
    UnknownRegion,
    /// Complete schema region state was missing.
    IncompleteRegionState,
    /// A region exceeded its panel-instance limit.
    TooManyPanelInstancesInRegion,
    /// Collapse state appeared on an unsupported region.
    UnsupportedCollapseState,
    /// The active panel did not belong to its region.
    ActivePanelNotInRegion,
    /// One panel instance appeared in more than one placement.
    DuplicatePanelPlacement,
    /// A region referenced an unknown panel instance.
    UnknownPanelInstance,
    /// A validated registry lookup unexpectedly failed.
    UnknownDefinitionReference,
    /// Panel policy rejected its current region.
    PanelPlacementNotAllowed,
    /// A sizing-slot id appeared more than once in one surface.
    DuplicateSizingSlotState,
    /// A sizing slot was not part of its surface schema.
    UnknownSizingSlot,
    /// Complete schema sizing-slot state was missing.
    IncompleteSizingSlotState,
    /// A sizing ratio fell outside registered bounds.
    SizingRatioOutOfBounds,
    /// A panel instance had no placement.
    UnplacedPanelInstance,
    /// Registered panel instance-count policy was exceeded.
    InstancePolicyExceeded,
    /// Valid state was not in canonical normalized form.
    NonCanonicalDocument,
}

/// Invalid current layout document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutValidationError {
    code: LayoutValidationCode,
    detail: String,
}

impl LayoutValidationError {
    /// Returns the stable validation category.
    #[must_use]
    pub const fn code(&self) -> LayoutValidationCode {
        self.code
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for LayoutValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for LayoutValidationError {}

pub(super) fn validation_error(
    code: LayoutValidationCode,
    detail: impl Into<String>,
) -> LayoutValidationError {
    LayoutValidationError {
        code,
        detail: detail.into(),
    }
}
