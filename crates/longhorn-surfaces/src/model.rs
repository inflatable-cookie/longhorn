use longhorn_core::{
    LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionId, SizingSlotId, SurfaceId,
    SurfaceRevision, WindowId,
};

use crate::layout::model::{PanelInstance, RegionState, SizingSlotState};
use serde::{Deserialize, Serialize};

/// One candidate host and its declared tab order for a Surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceHostPreference {
    window_id: WindowId,
    order: u32,
}

impl SurfaceHostPreference {
    /// Constructs one candidate host entry.
    #[must_use]
    pub const fn new(window_id: WindowId, order: u32) -> Self {
        Self { window_id, order }
    }

    /// Returns the participating candidate window.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the declared zero-based order in that candidate window.
    #[must_use]
    pub const fn order(&self) -> u32 {
        self.order
    }

    pub(crate) fn set_order(&mut self, order: u32) {
        self.order = order;
    }
}

/// How one Surface presents its bound layout container.
///
/// `Regional` is the ordinary case: the container's own region tree decides
/// what renders. `FocusedPanel` names one panel to render full-surface, with no
/// regional layout and no panel tabs -- a dedicated console or manager surface.
///
/// The surfaces domain records which panel is focused. It does not verify that
/// the bound container holds that panel and only that panel, because it has no
/// view of container contents; see Card 177 for why that invariant is a
/// consumer obligation rather than a rejection here.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum SurfacePresentation {
    /// The bound container's region tree decides what renders.
    #[default]
    Regional,
    /// One panel renders full-surface without regions or tabs.
    FocusedPanel {
        /// The panel rendered for the whole Surface.
        panel_definition_id: PanelDefinitionId,
    },
}

impl SurfacePresentation {
    /// Returns the focused panel, when this Surface presents exactly one.
    #[must_use]
    pub const fn focused_panel(&self) -> Option<&PanelDefinitionId> {
        match self {
            Self::Regional => None,
            Self::FocusedPanel {
                panel_definition_id,
            } => Some(panel_definition_id),
        }
    }

    /// Returns whether the bound container's regions decide what renders.
    #[must_use]
    pub const fn is_regional(&self) -> bool {
        matches!(self, Self::Regional)
    }
}

/// Durable generic metadata and hosting policy for one Surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceRecord {
    id: SurfaceId,
    schema_id: LayoutSchemaId,
    label: Option<String>,
    // Defaulted so a document written before Card 177 loads unchanged. The
    // stored schema version belongs to the consumer's migration hook, so an
    // additive field with a default is what keeps NoSurfaceMigration correct.
    #[serde(default)]
    presentation: SurfacePresentation,
    regions: Vec<RegionState>,
    sizing_slots: Vec<SizingSlotState>,
    host_preferences: Vec<SurfaceHostPreference>,
}

impl SurfaceRecord {
    /// Constructs one Surface without product or layout payload.
    #[must_use]
    pub fn new(
        id: SurfaceId,
        schema_id: LayoutSchemaId,
        label: Option<String>,
        regions: impl IntoIterator<Item = RegionState>,
        sizing_slots: impl IntoIterator<Item = SizingSlotState>,
        host_preferences: impl IntoIterator<Item = SurfaceHostPreference>,
    ) -> Self {
        Self {
            id,
            schema_id,
            label,
            presentation: SurfacePresentation::Regional,
            regions: regions.into_iter().collect(),
            sizing_slots: sizing_slots.into_iter().collect(),
            host_preferences: host_preferences.into_iter().collect(),
        }
    }

    /// Constructs one Surface with an explicit presentation.
    #[must_use]
    pub fn with_presentation(
        id: SurfaceId,
        schema_id: LayoutSchemaId,
        label: Option<String>,
        presentation: SurfacePresentation,
        regions: impl IntoIterator<Item = RegionState>,
        sizing_slots: impl IntoIterator<Item = SizingSlotState>,
        host_preferences: impl IntoIterator<Item = SurfaceHostPreference>,
    ) -> Self {
        Self {
            id,
            schema_id,
            label,
            presentation,
            regions: regions.into_iter().collect(),
            sizing_slots: sizing_slots.into_iter().collect(),
            host_preferences: host_preferences.into_iter().collect(),
        }
    }

    /// Returns stable Surface identity.
    #[must_use]
    pub const fn id(&self) -> &SurfaceId {
        &self.id
    }

    /// Returns the registered layout schema this Surface is an instance of.
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

    /// Returns the optional mutable display label.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns candidate hosts in declared fallback priority.
    #[must_use]
    pub fn host_preferences(&self) -> &[SurfaceHostPreference] {
        self.host_preferences.as_slice()
    }

    /// Returns how this Surface presents its bound container.
    #[must_use]
    pub const fn presentation(&self) -> &SurfacePresentation {
        &self.presentation
    }

    pub(crate) fn set_presentation(&mut self, presentation: SurfacePresentation) {
        self.presentation = presentation;
    }

    pub(crate) fn set_label(&mut self, label: Option<String>) {
        self.label = label;
    }

    pub(crate) fn host_preferences_mut(&mut self) -> &mut Vec<SurfaceHostPreference> {
        &mut self.host_preferences
    }
}

/// Durable active-Surface preference for one participating window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ParticipatingWindow {
    id: WindowId,
    active_surface_id: Option<SurfaceId>,
}

impl ParticipatingWindow {
    /// Constructs participating-window Surface state.
    #[must_use]
    pub const fn new(id: WindowId, active_surface_id: Option<SurfaceId>) -> Self {
        Self {
            id,
            active_surface_id,
        }
    }

    /// Returns logical participating-window identity.
    #[must_use]
    pub const fn id(&self) -> &WindowId {
        &self.id
    }

    /// Returns the preferred active member, when selected.
    #[must_use]
    pub const fn active_surface_id(&self) -> Option<&SurfaceId> {
        self.active_surface_id.as_ref()
    }

    pub(crate) fn set_active_surface_id(&mut self, surface_id: Option<SurfaceId>) {
        self.active_surface_id = surface_id;
    }
}

/// Complete durable optional Surface document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceDocument {
    revision: SurfaceRevision,
    surfaces: Vec<SurfaceRecord>,
    panel_instances: Vec<PanelInstance>,
    windows: Vec<ParticipatingWindow>,
}

impl SurfaceDocument {
    /// Constructs a complete Surface document.
    #[must_use]
    pub fn new(
        revision: SurfaceRevision,
        surfaces: impl IntoIterator<Item = SurfaceRecord>,
        panel_instances: impl IntoIterator<Item = PanelInstance>,
        windows: impl IntoIterator<Item = ParticipatingWindow>,
    ) -> Self {
        Self {
            revision,
            surfaces: surfaces.into_iter().collect(),
            panel_instances: panel_instances.into_iter().collect(),
            windows: windows.into_iter().collect(),
        }
    }

    /// Returns the monotonic durable revision.
    #[must_use]
    pub const fn revision(&self) -> SurfaceRevision {
        self.revision
    }

    /// Returns Surface records in canonical id order after normalization.
    #[must_use]
    pub fn surfaces(&self) -> &[SurfaceRecord] {
        self.surfaces.as_slice()
    }

    /// Returns panel instances in canonical id order after normalization.
    #[must_use]
    pub fn panel_instances(&self) -> &[PanelInstance] {
        self.panel_instances.as_slice()
    }

    /// Returns one panel instance.
    #[must_use]
    pub fn panel_instance(&self, id: &PanelInstanceId) -> Option<&PanelInstance> {
        self.panel_instances
            .iter()
            .find(|instance| instance.id() == id)
    }

    pub(crate) fn panel_instances_mut(&mut self) -> &mut Vec<PanelInstance> {
        &mut self.panel_instances
    }

    /// Returns participating windows in canonical id order after normalization.
    #[must_use]
    pub fn windows(&self) -> &[ParticipatingWindow] {
        self.windows.as_slice()
    }

    /// Returns one Surface record.
    #[must_use]
    pub fn surface(&self, id: &SurfaceId) -> Option<&SurfaceRecord> {
        self.surfaces.iter().find(|surface| surface.id() == id)
    }

    /// Returns one participating window.
    #[must_use]
    pub fn window(&self, id: &WindowId) -> Option<&ParticipatingWindow> {
        self.windows.iter().find(|window| window.id() == id)
    }

    pub(crate) fn surfaces_mut(&mut self) -> &mut Vec<SurfaceRecord> {
        &mut self.surfaces
    }

    pub(crate) fn windows_mut(&mut self) -> &mut Vec<ParticipatingWindow> {
        &mut self.windows
    }

    pub(crate) fn surface_mut(&mut self, id: &SurfaceId) -> Option<&mut SurfaceRecord> {
        self.surfaces.iter_mut().find(|surface| surface.id() == id)
    }

    pub(crate) fn window_mut(&mut self, id: &WindowId) -> Option<&mut ParticipatingWindow> {
        self.windows.iter_mut().find(|window| window.id() == id)
    }

    pub(crate) fn set_revision(&mut self, revision: SurfaceRevision) {
        self.revision = revision;
    }
}
