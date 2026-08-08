//! Observed native-content island state.

use longhorn_core::NativeContentRevision;
use serde::{Deserialize, Serialize};

use crate::AttachGeneration;

use super::{
    AttachmentLifecycle, EffectiveFocus, EffectiveVisibility, InputRoutingMode, ObservedGeometry,
    ObservedReadiness,
};
/// Current observed native state for one island.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ObservedState {
    pub(crate) revision: NativeContentRevision,
    pub(crate) generation: AttachGeneration,
    pub(crate) lifecycle: AttachmentLifecycle,
    pub(crate) readiness: ObservedReadiness,
    pub(crate) visibility: EffectiveVisibility,
    pub(crate) focus: EffectiveFocus,
    pub(crate) geometry: ObservedGeometry,
    pub(crate) input_routing: Option<InputRoutingMode>,
}

impl ObservedState {
    /// Constructs an initial absent observation for an attach generation.
    #[must_use]
    pub const fn absent(generation: AttachGeneration) -> Self {
        Self {
            revision: NativeContentRevision::INITIAL,
            generation,
            lifecycle: AttachmentLifecycle::Absent,
            readiness: ObservedReadiness::Unknown,
            visibility: EffectiveVisibility::Unknown,
            focus: EffectiveFocus::Unknown,
            geometry: ObservedGeometry::Unknown,
            input_routing: None,
        }
    }

    /// Returns observed-state revision.
    #[must_use]
    pub const fn revision(&self) -> NativeContentRevision {
        self.revision
    }
    /// Returns observed attach generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }
    /// Returns observed lifecycle.
    #[must_use]
    pub const fn lifecycle(&self) -> AttachmentLifecycle {
        self.lifecycle
    }
    /// Returns consumer readiness evidence.
    #[must_use]
    pub const fn readiness(&self) -> ObservedReadiness {
        self.readiness
    }
    /// Returns effective visibility evidence.
    #[must_use]
    pub const fn visibility(&self) -> EffectiveVisibility {
        self.visibility
    }
    /// Returns effective focus evidence.
    #[must_use]
    pub const fn focus(&self) -> EffectiveFocus {
        self.focus
    }
    /// Returns mechanism-specific geometry evidence.
    #[must_use]
    pub const fn geometry(&self) -> &ObservedGeometry {
        &self.geometry
    }
    /// Returns observed input routing when the adapter reports it.
    #[must_use]
    pub const fn input_routing(&self) -> Option<InputRoutingMode> {
        self.input_routing
    }

    pub(crate) fn replace(&mut self, revision: NativeContentRevision, update: ObservationUpdate) {
        self.revision = revision;
        self.generation = update.generation;
        self.lifecycle = update.lifecycle;
        self.readiness = update.readiness;
        self.visibility = update.visibility;
        self.focus = update.focus;
        self.geometry = update.geometry;
        self.input_routing = update.input_routing;
    }
}

/// One complete fresh adapter observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ObservationUpdate {
    pub(crate) generation: AttachGeneration,
    pub(crate) lifecycle: AttachmentLifecycle,
    pub(crate) readiness: ObservedReadiness,
    pub(crate) visibility: EffectiveVisibility,
    pub(crate) focus: EffectiveFocus,
    pub(crate) geometry: ObservedGeometry,
    pub(crate) input_routing: Option<InputRoutingMode>,
}

impl ObservationUpdate {
    /// Constructs a complete observation without inferred platform state.
    #[must_use]
    pub const fn new(
        generation: AttachGeneration,
        lifecycle: AttachmentLifecycle,
        readiness: ObservedReadiness,
        visibility: EffectiveVisibility,
        focus: EffectiveFocus,
        geometry: ObservedGeometry,
        input_routing: Option<InputRoutingMode>,
    ) -> Self {
        Self {
            generation,
            lifecycle,
            readiness,
            visibility,
            focus,
            geometry,
            input_routing,
        }
    }

    /// Constructs the complete absent observation used after host invalidation.
    #[must_use]
    pub const fn absent(generation: AttachGeneration) -> Self {
        Self::new(
            generation,
            AttachmentLifecycle::Absent,
            ObservedReadiness::Unknown,
            EffectiveVisibility::Unknown,
            EffectiveFocus::Unknown,
            ObservedGeometry::Unknown,
            None,
        )
    }

    /// Returns the generation named by the adapter.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }
    /// Returns the proposed lifecycle.
    #[must_use]
    pub const fn lifecycle(&self) -> AttachmentLifecycle {
        self.lifecycle
    }
    pub(crate) const fn geometry(&self) -> &ObservedGeometry {
        &self.geometry
    }
    pub(crate) const fn readiness(&self) -> ObservedReadiness {
        self.readiness
    }
    pub(crate) const fn visibility(&self) -> EffectiveVisibility {
        self.visibility
    }
    pub(crate) const fn focus(&self) -> EffectiveFocus {
        self.focus
    }
    pub(crate) const fn input_routing(&self) -> Option<InputRoutingMode> {
        self.input_routing
    }
}
