use longhorn_core::{DisplayId, ScaleFactor, ScreenRect, WindowId, WindowPlacement};
use longhorn_display::{DisplayAvailability, DisplayInventory};
use serde::{Deserialize, Serialize};

use crate::{
    PlacementPolicy, PlacementResolutionError, WindowPlacementConfig, WindowPlacementResolution,
    WindowRole, resolve_window_placement,
};

/// Serializable logical display evidence retained with a saved placement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SavedDisplayEvidence {
    full_bounds: ScreenRect,
    work_area: ScreenRect,
    scale: ScaleFactor,
}

impl SavedDisplayEvidence {
    /// Constructs exact logical geometry and scale evidence.
    #[must_use]
    pub const fn new(full_bounds: ScreenRect, work_area: ScreenRect, scale: ScaleFactor) -> Self {
        Self {
            full_bounds,
            work_area,
            scale,
        }
    }

    /// Returns the last observed full display bounds.
    #[must_use]
    pub const fn full_bounds(&self) -> ScreenRect {
        self.full_bounds
    }

    /// Returns the last observed usable work area.
    #[must_use]
    pub const fn work_area(&self) -> ScreenRect {
        self.work_area
    }

    /// Returns the last observed scale.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }
}

/// Serializable canonical identity and observation evidence for a saved display.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SavedDisplayAssociation {
    display_id: Option<DisplayId>,
    evidence: Option<SavedDisplayEvidence>,
}

impl SavedDisplayAssociation {
    /// Constructs an explicitly unresolved association.
    #[must_use]
    pub const fn unresolved() -> Self {
        Self {
            display_id: None,
            evidence: None,
        }
    }

    /// Constructs an association from optional canonical identity and evidence.
    #[must_use]
    pub const fn new(
        display_id: Option<DisplayId>,
        evidence: Option<SavedDisplayEvidence>,
    ) -> Self {
        Self {
            display_id,
            evidence,
        }
    }

    /// Returns canonical machine-local identity when correlation resolved it.
    #[must_use]
    pub const fn display_id(&self) -> Option<&DisplayId> {
        self.display_id.as_ref()
    }

    /// Returns the logical facts available at capture time.
    #[must_use]
    pub const fn evidence(&self) -> Option<&SavedDisplayEvidence> {
        self.evidence.as_ref()
    }
}

/// Serializable placement record accepted directly by restore planning.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SavedWindowPlacement {
    window_id: WindowId,
    normal_placement: WindowPlacement,
    maximized: bool,
    display: SavedDisplayAssociation,
}

impl SavedWindowPlacement {
    /// Constructs a complete durable placement record.
    #[must_use]
    pub const fn new(
        window_id: WindowId,
        normal_placement: WindowPlacement,
        maximized: bool,
        display: SavedDisplayAssociation,
    ) -> Self {
        Self {
            window_id,
            normal_placement,
            maximized,
            display,
        }
    }

    /// Returns stable logical window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns saved normal outer origin and inner size.
    #[must_use]
    pub const fn normal_placement(&self) -> WindowPlacement {
        self.normal_placement
    }

    /// Returns saved maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns saved display identity and observation evidence.
    #[must_use]
    pub const fn display(&self) -> &SavedDisplayAssociation {
        &self.display
    }
}

/// Restores one saved placement through the canonical fallback policy.
pub fn restore_window_placement(
    saved: &SavedWindowPlacement,
    inventory: &DisplayInventory,
    role: WindowRole,
    policy: PlacementPolicy,
) -> Result<WindowPlacementResolution, PlacementResolutionError> {
    let home_display_id = resolve_saved_display_association(saved.display(), inventory);
    let mut config =
        WindowPlacementConfig::new(saved.window_id().clone(), role, saved.normal_placement())
            .with_home_display(home_display_id.clone())
            .with_maximized(saved.is_maximized());
    if let Some(display_id) = home_display_id {
        config = config.with_normal_placement(display_id, saved.normal_placement());
    }
    resolve_window_placement(&config, inventory, policy)
}

/// Resolves saved canonical identity or unique exact logical display evidence.
#[must_use]
pub fn resolve_saved_display_association(
    association: &SavedDisplayAssociation,
    inventory: &DisplayInventory,
) -> Option<DisplayId> {
    if let Some(display_id) = association.display_id() {
        let canonical_is_available = inventory.displays().iter().any(|entry| {
            entry.display().id() == display_id
                && matches!(entry.availability(), DisplayAvailability::Available { .. })
        });
        if canonical_is_available {
            return Some(display_id.clone());
        }
    }

    let evidence = association.evidence()?;
    let mut matches = inventory.displays().iter().filter(|entry| {
        matches!(entry.availability(), DisplayAvailability::Available { .. })
            && entry.display().facts().full_bounds() == evidence.full_bounds()
            && entry.display().facts().work_area() == evidence.work_area()
            && entry.display().facts().scale() == evidence.scale()
    });
    let matched = matches.next()?.display().id().clone();
    matches.next().is_none().then_some(matched)
}
