use longhorn_core::{ScaleFactor, ScreenRect};
use serde::{Deserialize, Serialize};

use super::ids::DisplayLabel;

/// Host knowledge about whether a display is physically built in.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayBuiltinStatus {
    /// The host adapter cannot determine built-in status.
    Unknown,
    /// The host identifies the display as built in.
    BuiltIn,
    /// The host identifies the display as external.
    External,
}

/// Current display facts expressed in screen DIPs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DisplayFacts {
    machine_label: Option<DisplayLabel>,
    is_main: bool,
    builtin_status: DisplayBuiltinStatus,
    full_bounds: ScreenRect,
    work_area: ScreenRect,
    scale: ScaleFactor,
}

impl DisplayFacts {
    /// Constructs current or last-observed display facts.
    #[must_use]
    pub const fn new(
        machine_label: Option<DisplayLabel>,
        is_main: bool,
        builtin_status: DisplayBuiltinStatus,
        full_bounds: ScreenRect,
        work_area: ScreenRect,
        scale: ScaleFactor,
    ) -> Self {
        Self {
            machine_label,
            is_main,
            builtin_status,
            full_bounds,
            work_area,
            scale,
        }
    }

    /// Returns the machine-provided label.
    #[must_use]
    pub const fn machine_label(&self) -> Option<&DisplayLabel> {
        self.machine_label.as_ref()
    }

    /// Returns whether the host currently marks this as the main display.
    #[must_use]
    pub const fn is_main(&self) -> bool {
        self.is_main
    }

    /// Returns the host's current built-in status.
    #[must_use]
    pub const fn builtin_status(&self) -> DisplayBuiltinStatus {
        self.builtin_status
    }

    /// Returns full display bounds.
    #[must_use]
    pub const fn full_bounds(&self) -> ScreenRect {
        self.full_bounds
    }

    /// Returns usable work-area bounds.
    #[must_use]
    pub const fn work_area(&self) -> ScreenRect {
        self.work_area
    }

    /// Returns current scale evidence.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }
}
