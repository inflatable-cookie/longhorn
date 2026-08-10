use longhorn_core::{PanelDefinitionId, PanelInstanceId, RegionId, SizingSlotId};
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
