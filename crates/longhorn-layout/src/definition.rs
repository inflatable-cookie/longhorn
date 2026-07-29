use longhorn_core::{LayoutSchemaId, PanelDefinitionId, RegionFamilyId, RegionId, SizingSlotId};
use serde::{Deserialize, Serialize};

use crate::{LayoutLimits, LayoutRatio};

mod error;
mod registry;
mod validation;

use error::definition_error;
pub use error::{DefinitionErrorCode, DefinitionLookupError, DefinitionRegistryError};
pub use registry::LayoutDefinitionRegistry;
use validation::{validate_panel, validate_schema};

/// Durable visibility policy for an empty semantic region.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EmptyRegionPolicy {
    /// The region remains part of the ordinary presentation when empty.
    KeepVisible,
    /// The region is hidden when empty unless transiently revealed.
    HideWhenEmpty,
}

/// Registered definition of one semantic region.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct RegionDefinition {
    id: RegionId,
    family_id: RegionFamilyId,
    order: u32,
    empty_policy: EmptyRegionPolicy,
    collapsible: bool,
}

impl RegionDefinition {
    /// Constructs a region definition.
    #[must_use]
    pub const fn new(
        id: RegionId,
        family_id: RegionFamilyId,
        order: u32,
        empty_policy: EmptyRegionPolicy,
        collapsible: bool,
    ) -> Self {
        Self {
            id,
            family_id,
            order,
            empty_policy,
            collapsible,
        }
    }

    /// Returns stable region identity.
    #[must_use]
    pub const fn id(&self) -> &RegionId {
        &self.id
    }

    /// Returns consumer-defined region family identity.
    #[must_use]
    pub const fn family_id(&self) -> &RegionFamilyId {
        &self.family_id
    }

    /// Returns stable schema order.
    #[must_use]
    pub const fn order(&self) -> u32 {
        self.order
    }

    /// Returns empty-region presentation policy.
    #[must_use]
    pub const fn empty_policy(&self) -> EmptyRegionPolicy {
        self.empty_policy
    }

    /// Returns whether durable collapse state is supported.
    #[must_use]
    pub const fn is_collapsible(&self) -> bool {
        self.collapsible
    }
}

/// Registered fixed-point sizing control mapped by the consumer.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SizingSlotDefinition {
    id: SizingSlotId,
    order: u32,
    minimum: LayoutRatio,
    default: LayoutRatio,
    maximum: LayoutRatio,
}

impl SizingSlotDefinition {
    /// Constructs a sizing-slot definition.
    #[must_use]
    pub const fn new(
        id: SizingSlotId,
        order: u32,
        minimum: LayoutRatio,
        default: LayoutRatio,
        maximum: LayoutRatio,
    ) -> Self {
        Self {
            id,
            order,
            minimum,
            default,
            maximum,
        }
    }

    /// Returns stable sizing-slot identity.
    #[must_use]
    pub const fn id(&self) -> &SizingSlotId {
        &self.id
    }

    /// Returns stable schema order.
    #[must_use]
    pub const fn order(&self) -> u32 {
        self.order
    }

    /// Returns the minimum allowed ratio.
    #[must_use]
    pub const fn minimum(&self) -> LayoutRatio {
        self.minimum
    }

    /// Returns the initial ratio for a new container.
    #[must_use]
    pub const fn default(&self) -> LayoutRatio {
        self.default
    }

    /// Returns the maximum allowed ratio.
    #[must_use]
    pub const fn maximum(&self) -> LayoutRatio {
        self.maximum
    }

    pub(crate) const fn contains(&self, value: LayoutRatio) -> bool {
        value.millionths() >= self.minimum.millionths()
            && value.millionths() <= self.maximum.millionths()
    }
}

/// Complete flat semantic-region schema for one layout-container shape.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutSchemaDefinition {
    id: LayoutSchemaId,
    regions: Vec<RegionDefinition>,
    sizing_slots: Vec<SizingSlotDefinition>,
}

impl LayoutSchemaDefinition {
    /// Constructs a schema from complete region and sizing-slot definitions.
    #[must_use]
    pub fn new(
        id: LayoutSchemaId,
        regions: impl IntoIterator<Item = RegionDefinition>,
        sizing_slots: impl IntoIterator<Item = SizingSlotDefinition>,
    ) -> Self {
        Self {
            id,
            regions: regions.into_iter().collect(),
            sizing_slots: sizing_slots.into_iter().collect(),
        }
    }

    /// Returns stable schema identity.
    #[must_use]
    pub const fn id(&self) -> &LayoutSchemaId {
        &self.id
    }

    /// Returns regions in registered order.
    #[must_use]
    pub fn regions(&self) -> &[RegionDefinition] {
        self.regions.as_slice()
    }

    /// Returns sizing slots in registered order.
    #[must_use]
    pub fn sizing_slots(&self) -> &[SizingSlotDefinition] {
        self.sizing_slots.as_slice()
    }

    /// Returns one registered region.
    #[must_use]
    pub fn region(&self, id: &RegionId) -> Option<&RegionDefinition> {
        self.regions.iter().find(|region| region.id() == id)
    }

    /// Returns one registered sizing slot.
    #[must_use]
    pub fn sizing_slot(&self, id: &SizingSlotId) -> Option<&SizingSlotDefinition> {
        self.sizing_slots.iter().find(|slot| slot.id() == id)
    }
}

/// Region selector used by panel placement policy.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum PlacementSelector {
    /// Select one exact semantic region.
    Region(RegionId),
    /// Select every region in one consumer-defined family.
    Family(RegionFamilyId),
}

impl PlacementSelector {
    fn matches(&self, region: &RegionDefinition) -> bool {
        match self {
            Self::Region(id) => region.id() == id,
            Self::Family(id) => region.family_id() == id,
        }
    }
}

/// Explicit panel-instance count policy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PanelInstancePolicy {
    /// At most one instance may exist in the complete document.
    Singleton,
    /// At most one instance may exist in each container.
    OnePerContainer,
    /// Explicit document and per-container maxima.
    Bounded {
        /// Maximum instances in the complete document.
        maximum_per_document: u32,
        /// Maximum instances in one container.
        maximum_per_container: u32,
    },
    /// Multiple instances are permitted within document limits.
    Multiple,
}

/// Registered placement and lifecycle policy for one panel definition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PanelDefinition {
    id: PanelDefinitionId,
    default_placements: Vec<PlacementSelector>,
    allowed_placements: Vec<PlacementSelector>,
    instance_policy: PanelInstancePolicy,
    movable: bool,
    closeable: bool,
}

impl PanelDefinition {
    /// Constructs a panel definition.
    #[must_use]
    pub fn new(
        id: PanelDefinitionId,
        default_placements: impl IntoIterator<Item = PlacementSelector>,
        allowed_placements: impl IntoIterator<Item = PlacementSelector>,
        instance_policy: PanelInstancePolicy,
        movable: bool,
        closeable: bool,
    ) -> Self {
        Self {
            id,
            default_placements: default_placements.into_iter().collect(),
            allowed_placements: allowed_placements.into_iter().collect(),
            instance_policy,
            movable,
            closeable,
        }
    }

    /// Returns stable definition identity.
    #[must_use]
    pub const fn id(&self) -> &PanelDefinitionId {
        &self.id
    }

    /// Returns ordered default placement selectors.
    #[must_use]
    pub fn default_placements(&self) -> &[PlacementSelector] {
        self.default_placements.as_slice()
    }

    /// Returns explicit allowed placement selectors.
    #[must_use]
    pub fn allowed_placements(&self) -> &[PlacementSelector] {
        self.allowed_placements.as_slice()
    }

    /// Returns explicit instance-count policy.
    #[must_use]
    pub const fn instance_policy(&self) -> PanelInstancePolicy {
        self.instance_policy
    }

    /// Returns whether instances may move after creation.
    #[must_use]
    pub const fn is_movable(&self) -> bool {
        self.movable
    }

    /// Returns whether instances may close.
    #[must_use]
    pub const fn is_closeable(&self) -> bool {
        self.closeable
    }
}
