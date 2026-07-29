use longhorn_core::{DisplayId, WindowId, WindowPlacement};
use serde::{Deserialize, Serialize};

use crate::WindowPlacementConfig;

/// Consumer authority for adopting an attached display after a settled user move.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeDisplayAdoption {
    /// Preserve configured home regardless of the attached display.
    Preserve,
    /// Propose the attached display as the new configured home.
    AdoptAttachedDisplay,
}

/// Proposed configured-home change.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HomeDisplayChange {
    /// No configured-home mutation is proposed.
    Unchanged,
    /// Explicit policy permits adopting this canonical display.
    Adopt {
        /// Proposed new configured home.
        display_id: DisplayId,
    },
}

/// Proposed per-display normal-geometry update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlacementMemoryUpdate {
    display_id: DisplayId,
    normal_placement: WindowPlacement,
}

impl PlacementMemoryUpdate {
    /// Returns the attached canonical display.
    #[must_use]
    pub const fn display_id(&self) -> &DisplayId {
        &self.display_id
    }

    /// Returns settled normal geometry.
    #[must_use]
    pub const fn normal_placement(&self) -> WindowPlacement {
        self.normal_placement
    }
}

/// Pure proposal emitted after host settling identifies a user placement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SettledPlacementProposal {
    window_id: WindowId,
    memory_update: PlacementMemoryUpdate,
    maximized: bool,
    home_display_change: HomeDisplayChange,
}

impl SettledPlacementProposal {
    /// Returns logical window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the proposed per-display normal-memory update.
    #[must_use]
    pub const fn memory_update(&self) -> &PlacementMemoryUpdate {
        &self.memory_update
    }

    /// Returns settled maximized state, separate from normal geometry.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns the policy-governed configured-home proposal.
    #[must_use]
    pub const fn home_display_change(&self) -> &HomeDisplayChange {
        &self.home_display_change
    }
}

/// Proposes persistence changes without mutating configuration or performing I/O.
#[must_use]
pub fn propose_settled_placement(
    config: &WindowPlacementConfig,
    attached_display_id: DisplayId,
    normal_placement: WindowPlacement,
    maximized: bool,
    adoption: HomeDisplayAdoption,
) -> SettledPlacementProposal {
    let home_display_change = if adoption == HomeDisplayAdoption::AdoptAttachedDisplay
        && config.home_display_id() != Some(&attached_display_id)
    {
        HomeDisplayChange::Adopt {
            display_id: attached_display_id.clone(),
        }
    } else {
        HomeDisplayChange::Unchanged
    };

    SettledPlacementProposal {
        window_id: config.window_id().clone(),
        memory_update: PlacementMemoryUpdate {
            display_id: attached_display_id,
            normal_placement,
        },
        maximized,
        home_display_change,
    }
}
