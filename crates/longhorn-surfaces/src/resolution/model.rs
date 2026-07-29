use longhorn_core::{LayoutContainerId, SurfaceId, SurfaceRevision, WindowId};
use serde::{Deserialize, Serialize};

/// Current consumer-resolved presence and participating-window availability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceResolutionInput {
    admitted_surface_ids: Vec<SurfaceId>,
    available_window_ids: Vec<WindowId>,
}

impl SurfaceResolutionInput {
    /// Constructs current external resolution inputs without product predicates.
    #[must_use]
    pub fn new(
        admitted_surface_ids: impl IntoIterator<Item = SurfaceId>,
        available_window_ids: impl IntoIterator<Item = WindowId>,
    ) -> Self {
        Self {
            admitted_surface_ids: admitted_surface_ids.into_iter().collect(),
            available_window_ids: available_window_ids.into_iter().collect(),
        }
    }

    /// Returns current consumer-admitted Surface ids.
    #[must_use]
    pub fn admitted_surface_ids(&self) -> &[SurfaceId] {
        self.admitted_surface_ids.as_slice()
    }

    /// Returns currently available participating windows.
    #[must_use]
    pub fn available_window_ids(&self) -> &[WindowId] {
        self.available_window_ids.as_slice()
    }
}

/// One Surface assigned to an available participating window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedSurface {
    pub(super) surface_id: SurfaceId,
    pub(super) layout_container_id: LayoutContainerId,
    pub(super) label: Option<String>,
    pub(super) host_preference_index: u32,
}

impl ResolvedSurface {
    /// Returns resolved Surface identity.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    /// Returns the external Surface-to-layout binding.
    #[must_use]
    pub const fn layout_container_id(&self) -> &LayoutContainerId {
        &self.layout_container_id
    }

    /// Returns the optional display label.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the selected zero-based candidate index.
    #[must_use]
    pub const fn host_preference_index(&self) -> u32 {
        self.host_preference_index
    }
}

/// Ordered resolved Surface state for one available participating window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedSurfaceWindow {
    pub(super) window_id: WindowId,
    pub(super) surfaces: Vec<ResolvedSurface>,
    pub(super) active_surface_id: Option<SurfaceId>,
}

impl ResolvedSurfaceWindow {
    /// Returns participating-window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns resolved Surfaces in declared order.
    #[must_use]
    pub fn surfaces(&self) -> &[ResolvedSurface] {
        self.surfaces.as_slice()
    }

    /// Returns the resolved active Surface, absent for an empty window.
    #[must_use]
    pub const fn active_surface_id(&self) -> Option<&SurfaceId> {
        self.active_surface_id.as_ref()
    }
}

/// Typed reason one durable Surface has no current host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceUnresolvedReason {
    /// The consumer did not admit this Surface under current product state.
    NotAdmitted,
    /// None of the declared candidate windows is currently available.
    NoAvailableWindow,
}

/// One current unresolved Surface and its unchanged external layout binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnresolvedSurface {
    pub(super) surface_id: SurfaceId,
    pub(super) layout_container_id: LayoutContainerId,
    pub(super) reason: SurfaceUnresolvedReason,
}

impl UnresolvedSurface {
    /// Returns unresolved Surface identity.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    /// Returns the unchanged external layout-container binding.
    #[must_use]
    pub const fn layout_container_id(&self) -> &LayoutContainerId {
        &self.layout_container_id
    }

    /// Returns why no current host was selected.
    #[must_use]
    pub const fn reason(&self) -> SurfaceUnresolvedReason {
        self.reason
    }
}

/// Deterministic current projection of one valid Surface document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceResolution {
    pub(super) revision: SurfaceRevision,
    pub(super) windows: Vec<ResolvedSurfaceWindow>,
    pub(super) unresolved_surfaces: Vec<UnresolvedSurface>,
}

impl SurfaceResolution {
    /// Returns the projected durable revision.
    #[must_use]
    pub const fn revision(&self) -> SurfaceRevision {
        self.revision
    }

    /// Returns available participating windows in canonical id order.
    #[must_use]
    pub fn windows(&self) -> &[ResolvedSurfaceWindow] {
        self.windows.as_slice()
    }

    /// Returns unresolved Surfaces in canonical id order.
    #[must_use]
    pub fn unresolved_surfaces(&self) -> &[UnresolvedSurface] {
        self.unresolved_surfaces.as_slice()
    }
}
