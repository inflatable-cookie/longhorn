use std::{error::Error, fmt};

use longhorn_core::{DisplayId, GeometryError, ScreenRect, WindowId, WindowPlacement};
use serde::{Deserialize, Serialize};

use crate::WindowRole;

/// Why a currently available display was selected.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PlacementReason {
    /// The configured home display is available.
    ConfiguredHome,
    /// The first available configured fallback was selected.
    ConfiguredFallback,
    /// The required window most usefully intersects this display.
    UsefulIntersection {
        /// Positive intersection area in screen-DIP square units.
        area: u64,
    },
    /// No useful intersection existed, so the current main display won.
    MainDisplay,
    /// No configured, intersecting, or main display existed.
    DeterministicFallback,
}

/// Successfully resolved placement for one enabled logical window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedWindowPlacement {
    pub(crate) window_id: WindowId,
    pub(crate) role: WindowRole,
    pub(crate) configured_home_display_id: Option<DisplayId>,
    pub(crate) target_display_id: DisplayId,
    pub(crate) target_work_area: ScreenRect,
    pub(crate) reason: PlacementReason,
    pub(crate) normal_placement: WindowPlacement,
    pub(crate) maximized: bool,
}

impl ResolvedWindowPlacement {
    /// Returns logical window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns product-neutral window role.
    #[must_use]
    pub const fn role(&self) -> WindowRole {
        self.role
    }

    /// Returns the unchanged configured home display.
    #[must_use]
    pub const fn configured_home_display_id(&self) -> Option<&DisplayId> {
        self.configured_home_display_id.as_ref()
    }

    /// Returns the selected available display.
    #[must_use]
    pub const fn target_display_id(&self) -> &DisplayId {
        &self.target_display_id
    }

    /// Returns the selected display work area used for fitting.
    #[must_use]
    pub const fn target_work_area(&self) -> ScreenRect {
        self.target_work_area
    }

    /// Returns inspectable display-selection evidence.
    #[must_use]
    pub const fn reason(&self) -> &PlacementReason {
        &self.reason
    }

    /// Returns fitted normal geometry independently of maximized state.
    #[must_use]
    pub const fn normal_placement(&self) -> WindowPlacement {
        self.normal_placement
    }

    /// Returns desired maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns whether resolution temporarily departed from configured home.
    #[must_use]
    pub fn is_temporary_fallback(&self) -> bool {
        self.configured_home_display_id.as_ref() != Some(&self.target_display_id)
    }
}

/// Why an enabled window could not be placed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailablePlacementReason {
    /// The current inventory contains no available displays.
    NoAvailableDisplays,
    /// Displays exist, but an optional window has no available configured target.
    NoConfiguredDisplayAvailable,
}

/// Explicit unavailable outcome without fabricated geometry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnavailableWindowPlacement {
    window_id: WindowId,
    role: WindowRole,
    configured_home_display_id: Option<DisplayId>,
    reason: UnavailablePlacementReason,
}

impl UnavailableWindowPlacement {
    pub(crate) const fn new(
        window_id: WindowId,
        role: WindowRole,
        configured_home_display_id: Option<DisplayId>,
        reason: UnavailablePlacementReason,
    ) -> Self {
        Self {
            window_id,
            role,
            configured_home_display_id,
            reason,
        }
    }

    /// Returns logical window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns product-neutral window role.
    #[must_use]
    pub const fn role(&self) -> WindowRole {
        self.role
    }

    /// Returns the unchanged configured home display.
    #[must_use]
    pub const fn configured_home_display_id(&self) -> Option<&DisplayId> {
        self.configured_home_display_id.as_ref()
    }

    /// Returns why no placement was fabricated.
    #[must_use]
    pub const fn reason(&self) -> UnavailablePlacementReason {
        self.reason
    }
}

/// Complete placement result for one logical window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "value")]
pub enum WindowPlacementResolution {
    /// Configuration disabled this logical window.
    Disabled {
        /// Stable logical identity.
        window_id: WindowId,
        /// Product-neutral role retained for diagnostics.
        role: WindowRole,
    },
    /// An available display and fitted normal placement were resolved.
    Resolved(ResolvedWindowPlacement),
    /// No eligible placement exists.
    Unavailable(UnavailableWindowPlacement),
}

/// Pure placement resolution failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementResolutionError {
    /// The selected display had invalid work-area geometry.
    Geometry {
        /// Logical window being resolved.
        window_id: WindowId,
        /// Selected canonical display.
        display_id: DisplayId,
        /// Checked geometry failure.
        source: GeometryError,
    },
}

impl fmt::Display for PlacementResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry {
                window_id,
                display_id,
                source,
            } => write!(
                formatter,
                "window {window_id} placement on display {display_id} failed: {source}"
            ),
        }
    }
}

impl Error for PlacementResolutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Geometry { source, .. } => Some(source),
        }
    }
}
