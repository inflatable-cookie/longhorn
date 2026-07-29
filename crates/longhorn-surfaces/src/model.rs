use longhorn_core::{LayoutContainerId, SurfaceId, SurfaceRevision, WindowId};
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

/// Durable generic metadata and hosting policy for one Surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceRecord {
    id: SurfaceId,
    layout_container_id: LayoutContainerId,
    label: Option<String>,
    host_preferences: Vec<SurfaceHostPreference>,
}

impl SurfaceRecord {
    /// Constructs one Surface without product or layout payload.
    #[must_use]
    pub fn new(
        id: SurfaceId,
        layout_container_id: LayoutContainerId,
        label: Option<String>,
        host_preferences: impl IntoIterator<Item = SurfaceHostPreference>,
    ) -> Self {
        Self {
            id,
            layout_container_id,
            label,
            host_preferences: host_preferences.into_iter().collect(),
        }
    }

    /// Returns stable Surface identity.
    #[must_use]
    pub const fn id(&self) -> &SurfaceId {
        &self.id
    }

    /// Returns the distinct external layout-container binding.
    #[must_use]
    pub const fn layout_container_id(&self) -> &LayoutContainerId {
        &self.layout_container_id
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
    windows: Vec<ParticipatingWindow>,
}

impl SurfaceDocument {
    /// Constructs a complete Surface document.
    #[must_use]
    pub fn new(
        revision: SurfaceRevision,
        surfaces: impl IntoIterator<Item = SurfaceRecord>,
        windows: impl IntoIterator<Item = ParticipatingWindow>,
    ) -> Self {
        Self {
            revision,
            surfaces: surfaces.into_iter().collect(),
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
