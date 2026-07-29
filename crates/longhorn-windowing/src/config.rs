use std::collections::BTreeMap;

use longhorn_core::{DisplayId, ScreenSize, WindowId, WindowPlacement};
use serde::{Deserialize, Serialize};

/// Product-neutral placement importance for a logical window.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowRole {
    /// The application requires this primary window whenever a display exists.
    RequiredPrimary,
    /// The window may remain unavailable when configured displays are absent.
    Optional,
}

/// Persistent, product-supplied placement inputs for one logical window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowPlacementConfig {
    window_id: WindowId,
    enabled: bool,
    role: WindowRole,
    home_display_id: Option<DisplayId>,
    fallback_display_ids: Vec<DisplayId>,
    normal_placements: BTreeMap<DisplayId, WindowPlacement>,
    default_normal_placement: WindowPlacement,
    maximized: bool,
}

impl WindowPlacementConfig {
    /// Constructs enabled placement configuration with explicit default geometry.
    #[must_use]
    pub const fn new(
        window_id: WindowId,
        role: WindowRole,
        default_normal_placement: WindowPlacement,
    ) -> Self {
        Self {
            window_id,
            enabled: true,
            role,
            home_display_id: None,
            fallback_display_ids: Vec::new(),
            normal_placements: BTreeMap::new(),
            default_normal_placement,
            maximized: false,
        }
    }

    /// Sets whether the logical window participates in placement.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets or clears the configured home display.
    #[must_use]
    pub fn with_home_display(mut self, display_id: Option<DisplayId>) -> Self {
        self.home_display_id = display_id;
        self
    }

    /// Replaces the ordered configured fallback displays.
    #[must_use]
    pub fn with_fallback_displays(
        mut self,
        display_ids: impl IntoIterator<Item = DisplayId>,
    ) -> Self {
        self.fallback_display_ids = display_ids.into_iter().collect();
        self
    }

    /// Remembers normal geometry independently for one canonical display.
    #[must_use]
    pub fn with_normal_placement(
        mut self,
        display_id: DisplayId,
        placement: WindowPlacement,
    ) -> Self {
        self.normal_placements.insert(display_id, placement);
        self
    }

    /// Sets the desired maximized state without replacing normal geometry.
    #[must_use]
    pub const fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Returns stable logical identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns whether the window participates in placement.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns product-neutral placement importance.
    #[must_use]
    pub const fn role(&self) -> WindowRole {
        self.role
    }

    /// Returns the configured home without applying temporary fallback.
    #[must_use]
    pub const fn home_display_id(&self) -> Option<&DisplayId> {
        self.home_display_id.as_ref()
    }

    /// Returns configured fallbacks in consumer priority order.
    #[must_use]
    pub fn fallback_display_ids(&self) -> &[DisplayId] {
        self.fallback_display_ids.as_slice()
    }

    /// Returns remembered normal geometry for one canonical display.
    #[must_use]
    pub fn normal_placement_for(&self, display_id: &DisplayId) -> Option<WindowPlacement> {
        self.normal_placements.get(display_id).copied()
    }

    /// Returns all per-display normal placements in canonical display order.
    #[must_use]
    pub const fn normal_placements(&self) -> &BTreeMap<DisplayId, WindowPlacement> {
        &self.normal_placements
    }

    /// Returns caller-supplied geometry used when no display memory applies.
    #[must_use]
    pub const fn default_normal_placement(&self) -> WindowPlacement {
        self.default_normal_placement
    }

    /// Returns desired maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    pub(crate) fn reference_normal_placement(&self) -> WindowPlacement {
        self.home_display_id
            .as_ref()
            .and_then(|id| self.normal_placement_for(id))
            .or_else(|| {
                self.fallback_display_ids
                    .iter()
                    .find_map(|id| self.normal_placement_for(id))
            })
            .unwrap_or(self.default_normal_placement)
    }
}

/// Consumer-supplied geometry policy for placement resolution.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlacementPolicy {
    minimum_size: ScreenSize,
    minimum_visible_extent: ScreenSize,
}

impl PlacementPolicy {
    /// Constructs policy without product or platform defaults.
    #[must_use]
    pub const fn new(minimum_size: ScreenSize, minimum_visible_extent: ScreenSize) -> Self {
        Self {
            minimum_size,
            minimum_visible_extent,
        }
    }

    /// Returns minimum resolved content size.
    #[must_use]
    pub const fn minimum_size(&self) -> ScreenSize {
        self.minimum_size
    }

    /// Returns the intersection extent required for a useful display candidate.
    #[must_use]
    pub const fn minimum_visible_extent(&self) -> ScreenSize {
        self.minimum_visible_extent
    }
}
