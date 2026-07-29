use longhorn_core::{
    LayoutContainerId, LayoutRevision, LayoutSchemaId, PanelDefinitionId, PanelInstanceId,
    RegionId, SizingSlotId,
};
use serde::{Deserialize, Serialize};

use crate::LayoutRatio;

/// Durable product-neutral panel instance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PanelInstance {
    id: PanelInstanceId,
    definition_id: PanelDefinitionId,
}

impl PanelInstance {
    /// Constructs one panel instance without product payload.
    #[must_use]
    pub const fn new(id: PanelInstanceId, definition_id: PanelDefinitionId) -> Self {
        Self { id, definition_id }
    }

    /// Returns stable panel-instance identity.
    #[must_use]
    pub const fn id(&self) -> &PanelInstanceId {
        &self.id
    }

    /// Returns registered panel-definition identity.
    #[must_use]
    pub const fn definition_id(&self) -> &PanelDefinitionId {
        &self.definition_id
    }
}

/// Durable state for one semantic region.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct RegionState {
    region_id: RegionId,
    panel_instance_ids: Vec<PanelInstanceId>,
    active_panel_instance_id: Option<PanelInstanceId>,
    collapsed: Option<bool>,
}

impl RegionState {
    /// Constructs region state.
    #[must_use]
    pub fn new(
        region_id: RegionId,
        panel_instance_ids: impl IntoIterator<Item = PanelInstanceId>,
        active_panel_instance_id: Option<PanelInstanceId>,
        collapsed: Option<bool>,
    ) -> Self {
        Self {
            region_id,
            panel_instance_ids: panel_instance_ids.into_iter().collect(),
            active_panel_instance_id,
            collapsed,
        }
    }

    /// Returns stable semantic region identity.
    #[must_use]
    pub const fn region_id(&self) -> &RegionId {
        &self.region_id
    }

    /// Returns panel instances in durable tab order.
    #[must_use]
    pub fn panel_instance_ids(&self) -> &[PanelInstanceId] {
        self.panel_instance_ids.as_slice()
    }

    /// Returns the durable active panel, when selected.
    #[must_use]
    pub const fn active_panel_instance_id(&self) -> Option<&PanelInstanceId> {
        self.active_panel_instance_id.as_ref()
    }

    /// Returns collapse state only when the schema supports it.
    #[must_use]
    pub const fn collapsed(&self) -> Option<bool> {
        self.collapsed
    }

    pub(crate) fn normalize_active(&mut self) {
        if self.panel_instance_ids.is_empty() {
            self.active_panel_instance_id = None;
        } else if self.active_panel_instance_id.is_none() {
            self.active_panel_instance_id = self.panel_instance_ids.first().cloned();
        }
    }

    pub(crate) fn normalize_collapse(&mut self, collapsible: bool) {
        self.collapsed = collapsible.then_some(self.collapsed.unwrap_or(false));
    }

    pub(crate) fn panel_instance_ids_mut(&mut self) -> &mut Vec<PanelInstanceId> {
        &mut self.panel_instance_ids
    }

    pub(crate) fn set_active_panel_instance_id(&mut self, id: Option<PanelInstanceId>) {
        self.active_panel_instance_id = id;
    }

    pub(crate) fn set_collapsed(&mut self, collapsed: bool) {
        self.collapsed = Some(collapsed);
    }
}

/// Durable value for one consumer-mapped sizing slot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SizingSlotState {
    sizing_slot_id: SizingSlotId,
    ratio: LayoutRatio,
}

impl SizingSlotState {
    /// Constructs sizing-slot state.
    #[must_use]
    pub const fn new(sizing_slot_id: SizingSlotId, ratio: LayoutRatio) -> Self {
        Self {
            sizing_slot_id,
            ratio,
        }
    }

    /// Returns stable sizing-slot identity.
    #[must_use]
    pub const fn sizing_slot_id(&self) -> &SizingSlotId {
        &self.sizing_slot_id
    }

    /// Returns fixed-point sizing ratio.
    #[must_use]
    pub const fn ratio(&self) -> LayoutRatio {
        self.ratio
    }

    pub(crate) fn set_ratio(&mut self, ratio: LayoutRatio) {
        self.ratio = ratio;
    }
}

/// Durable Surface-independent layout container.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutContainer {
    id: LayoutContainerId,
    schema_id: LayoutSchemaId,
    regions: Vec<RegionState>,
    sizing_slots: Vec<SizingSlotState>,
}

impl LayoutContainer {
    /// Constructs a container from complete region and sizing state.
    #[must_use]
    pub fn new(
        id: LayoutContainerId,
        schema_id: LayoutSchemaId,
        regions: impl IntoIterator<Item = RegionState>,
        sizing_slots: impl IntoIterator<Item = SizingSlotState>,
    ) -> Self {
        Self {
            id,
            schema_id,
            regions: regions.into_iter().collect(),
            sizing_slots: sizing_slots.into_iter().collect(),
        }
    }

    /// Returns stable container identity.
    #[must_use]
    pub const fn id(&self) -> &LayoutContainerId {
        &self.id
    }

    /// Returns the registered schema used by this container.
    #[must_use]
    pub const fn schema_id(&self) -> &LayoutSchemaId {
        &self.schema_id
    }

    /// Returns complete region state.
    #[must_use]
    pub fn regions(&self) -> &[RegionState] {
        self.regions.as_slice()
    }

    /// Returns complete sizing-slot state.
    #[must_use]
    pub fn sizing_slots(&self) -> &[SizingSlotState] {
        self.sizing_slots.as_slice()
    }

    /// Returns one region state.
    #[must_use]
    pub fn region(&self, id: &RegionId) -> Option<&RegionState> {
        self.regions.iter().find(|region| region.region_id() == id)
    }

    /// Returns one sizing-slot state.
    #[must_use]
    pub fn sizing_slot(&self, id: &SizingSlotId) -> Option<&SizingSlotState> {
        self.sizing_slots
            .iter()
            .find(|slot| slot.sizing_slot_id() == id)
    }

    pub(crate) fn regions_mut(&mut self) -> &mut Vec<RegionState> {
        &mut self.regions
    }

    pub(crate) fn sizing_slots_mut(&mut self) -> &mut Vec<SizingSlotState> {
        &mut self.sizing_slots
    }

    pub(crate) fn region_mut(&mut self, id: &RegionId) -> Option<&mut RegionState> {
        self.regions
            .iter_mut()
            .find(|region| region.region_id() == id)
    }

    pub(crate) fn sizing_slot_mut(&mut self, id: &SizingSlotId) -> Option<&mut SizingSlotState> {
        self.sizing_slots
            .iter_mut()
            .find(|slot| slot.sizing_slot_id() == id)
    }
}

/// Complete durable layout document for one consumer-selected scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutDocument {
    revision: LayoutRevision,
    containers: Vec<LayoutContainer>,
    panel_instances: Vec<PanelInstance>,
}

impl LayoutDocument {
    /// Constructs a document from complete container and instance state.
    #[must_use]
    pub fn new(
        revision: LayoutRevision,
        containers: impl IntoIterator<Item = LayoutContainer>,
        panel_instances: impl IntoIterator<Item = PanelInstance>,
    ) -> Self {
        Self {
            revision,
            containers: containers.into_iter().collect(),
            panel_instances: panel_instances.into_iter().collect(),
        }
    }

    /// Returns the monotonic durable revision.
    #[must_use]
    pub const fn revision(&self) -> LayoutRevision {
        self.revision
    }

    /// Returns containers in canonical id order after normalization.
    #[must_use]
    pub fn containers(&self) -> &[LayoutContainer] {
        self.containers.as_slice()
    }

    /// Returns panel instances in canonical id order after normalization.
    #[must_use]
    pub fn panel_instances(&self) -> &[PanelInstance] {
        self.panel_instances.as_slice()
    }

    /// Returns one layout container.
    #[must_use]
    pub fn container(&self, id: &LayoutContainerId) -> Option<&LayoutContainer> {
        self.containers
            .iter()
            .find(|container| container.id() == id)
    }

    /// Returns one panel instance.
    #[must_use]
    pub fn panel_instance(&self, id: &PanelInstanceId) -> Option<&PanelInstance> {
        self.panel_instances
            .iter()
            .find(|instance| instance.id() == id)
    }

    pub(crate) fn containers_mut(&mut self) -> &mut Vec<LayoutContainer> {
        &mut self.containers
    }

    pub(crate) fn panel_instances_mut(&mut self) -> &mut Vec<PanelInstance> {
        &mut self.panel_instances
    }

    pub(crate) fn container_mut(&mut self, id: &LayoutContainerId) -> Option<&mut LayoutContainer> {
        self.containers
            .iter_mut()
            .find(|container| container.id() == id)
    }

    pub(crate) fn set_revision(&mut self, revision: LayoutRevision) {
        self.revision = revision;
    }
}
