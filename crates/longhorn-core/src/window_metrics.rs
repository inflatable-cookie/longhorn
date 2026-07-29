use serde::{Deserialize, Serialize};

use crate::{ScreenPoint, ScreenRect, ScreenSize};

/// Desired placement expressed as outer-frame origin plus inner content size.
///
/// Desired placement cannot substitute for observed outer bounds:
///
/// ```compile_fail
/// use longhorn_core::{LiveWindowMetrics, ScreenPoint, ScreenSize, WindowPlacement};
///
/// let placement = WindowPlacement::new(ScreenPoint::new(10, 20), ScreenSize::new(800, 600));
/// let _live: LiveWindowMetrics = placement;
/// ```
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowPlacement {
    outer_origin: ScreenPoint,
    inner_size: ScreenSize,
}

impl WindowPlacement {
    /// Constructs desired placement.
    #[must_use]
    pub const fn new(outer_origin: ScreenPoint, inner_size: ScreenSize) -> Self {
        Self {
            outer_origin,
            inner_size,
        }
    }

    /// Returns the desired outer-frame origin.
    #[must_use]
    pub const fn outer_origin(&self) -> ScreenPoint {
        self.outer_origin
    }

    /// Returns the desired inner content size.
    #[must_use]
    pub const fn inner_size(&self) -> ScreenSize {
        self.inner_size
    }
}

/// Observed live geometry with explicit outer bounds.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveWindowMetrics {
    outer_bounds: ScreenRect,
    inner_size: ScreenSize,
}

impl LiveWindowMetrics {
    /// Constructs observed live geometry.
    #[must_use]
    pub const fn new(outer_bounds: ScreenRect, inner_size: ScreenSize) -> Self {
        Self {
            outer_bounds,
            inner_size,
        }
    }

    /// Returns live outer bounds for screen hit-testing.
    #[must_use]
    pub const fn outer_bounds(&self) -> ScreenRect {
        self.outer_bounds
    }

    /// Returns observed inner content size.
    #[must_use]
    pub const fn inner_size(&self) -> ScreenSize {
        self.inner_size
    }
}
