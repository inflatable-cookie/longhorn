//! Desired and observed surface intent enums.

use longhorn_core::{
    ClientRect, NativeContentIslandId, NativeContentKindId, NativeContentRevision, PhysicalRect,
    PhysicalSize, RoundingMode, ScaleFactor, VisibilityReasonId, WindowId,
};
use serde::{Deserialize, Serialize};

use crate::{AttachGeneration, CoordinationError};
/// Desired native-content presence.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DesiredPresence {
    /// No native content should remain attached.
    Absent,
    /// Native content should exist for the current generation.
    Present,
}

/// Explicit desired presentation visibility.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum DesiredVisibility {
    /// Present the content when the host allows it.
    Visible,
    /// Hide the content for a consumer-computed reason.
    Hidden {
        /// Stable consumer reason. Longhorn does not infer this from DOM state.
        reason: VisibilityReasonId,
    },
}

/// Desired focus operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum FocusIntent {
    /// Preserve current focus.
    Unchanged,
    /// Request native focus.
    Request,
    /// Release focus only when the adapter owns it.
    ReleaseIfOwned,
}

/// Current native attachment lifecycle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AttachmentLifecycle {
    /// No native content is attached.
    Absent,
    /// The selected adapter is attaching native content.
    Attaching,
    /// Native content is attached.
    Attached,
    /// The selected adapter is detaching native content.
    Detaching,
    /// The current generation failed terminally.
    Failed,
}

/// Consumer-defined readiness evidence inside one attached generation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ObservedReadiness {
    /// Readiness is unavailable.
    Unknown,
    /// The consumer reports that content is not ready.
    NotReady,
    /// The consumer reports that its declared readiness condition passed.
    Ready,
}

/// Effective native visibility observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EffectiveVisibility {
    /// Native content is observed visible.
    Visible,
    /// Native content is observed hidden.
    Hidden,
    /// The adapter cannot currently establish visibility.
    Unknown,
}

/// Effective native focus observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EffectiveFocus {
    /// Native content is observed focused.
    Focused,
    /// Native content is observed unfocused.
    Unfocused,
    /// The adapter cannot currently establish focus.
    Unknown,
}

/// Mechanism-specific native geometry observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ObservedGeometry {
    /// No current native geometry evidence exists.
    Unknown,
    /// A child view reports its complete physical bounds.
    ChildBounds {
        /// Fresh native child bounds.
        #[cfg_attr(
            feature = "bindings",
            ts(
                type = "{ origin: { x: number; y: number }; size: { width: number; height: number } }"
            )
        )]
        bounds: PhysicalRect,
    },
    /// An isolated window reports its physical content size.
    IsolatedContent {
        /// Fresh native content size, excluding outer-frame placement.
        #[cfg_attr(feature = "bindings", ts(type = "{ width: number; height: number }"))]
        size: PhysicalSize,
    },
    /// A backing surface reports storage bounds and current clip separately.
    BackingSurface {
        /// Fresh physical bounds of the native backing storage.
        #[cfg_attr(
            feature = "bindings",
            ts(
                type = "{ origin: { x: number; y: number }; size: { width: number; height: number } }"
            )
        )]
        storage_bounds: PhysicalRect,
        /// Fresh physical presentation and interaction clip.
        #[cfg_attr(
            feature = "bindings",
            ts(
                type = "{ origin: { x: number; y: number }; size: { width: number; height: number } }"
            )
        )]
        clip: PhysicalRect,
    },
}

