//! Desired native-content island state.

use longhorn_core::{
    ClientRect, NativeContentIslandId, NativeContentKindId, NativeContentRevision, RoundingMode,
    ScaleFactor, WindowId,
};
use serde::{Deserialize, Serialize};

use crate::{AttachGeneration, CoordinationError};

use super::{
    DesiredPresence, DesiredVisibility, FocusIntent, InputRoutingMode, MechanismCapabilities,
};
/// Complete desired state for one native-content island.

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(try_from = "DesiredStateWire")]
pub struct DesiredState {
    pub(crate) island_id: NativeContentIslandId,
    pub(crate) kind_id: NativeContentKindId,
    pub(crate) capabilities: MechanismCapabilities,
    pub(crate) revision: NativeContentRevision,
    pub(crate) generation: AttachGeneration,
    pub(crate) host_window_id: WindowId,
    pub(crate) viewport: ClientRect,
    pub(crate) scale: ScaleFactor,
    pub(crate) rounding: RoundingMode,
    pub(crate) presence: DesiredPresence,
    pub(crate) visibility: DesiredVisibility,
    pub(crate) focus: FocusIntent,
    pub(crate) input_routing: InputRoutingMode,
}

#[derive(Deserialize)]
pub(crate) struct DesiredStateWire {
    pub(crate) island_id: NativeContentIslandId,
    pub(crate) kind_id: NativeContentKindId,
    pub(crate) capabilities: MechanismCapabilities,
    pub(crate) revision: NativeContentRevision,
    pub(crate) generation: AttachGeneration,
    pub(crate) host_window_id: WindowId,
    pub(crate) viewport: ClientRect,
    pub(crate) scale: ScaleFactor,
    pub(crate) rounding: RoundingMode,
    pub(crate) presence: DesiredPresence,
    pub(crate) visibility: DesiredVisibility,
    pub(crate) focus: FocusIntent,
    pub(crate) input_routing: InputRoutingMode,
}

impl TryFrom<DesiredStateWire> for DesiredState {
    type Error = CoordinationError;

    fn try_from(value: DesiredStateWire) -> Result<Self, Self::Error> {
        value.capabilities.validate_input(value.input_routing)?;
        Ok(Self {
            island_id: value.island_id,
            kind_id: value.kind_id,
            capabilities: value.capabilities,
            revision: value.revision,
            generation: value.generation,
            host_window_id: value.host_window_id,
            viewport: value.viewport,
            scale: value.scale,
            rounding: value.rounding,
            presence: value.presence,
            visibility: value.visibility,
            focus: value.focus,
            input_routing: value.input_routing,
        })
    }
}

impl DesiredState {
    /// Constructs initial desired state after validating mechanism capabilities.
    pub fn new(
        island_id: NativeContentIslandId,
        kind_id: NativeContentKindId,
        capabilities: MechanismCapabilities,
        update: DesiredUpdate,
    ) -> Result<Self, CoordinationError> {
        capabilities.validate_input(update.input_routing)?;
        Ok(Self {
            island_id,
            kind_id,
            capabilities,
            revision: NativeContentRevision::INITIAL,
            generation: update.generation,
            host_window_id: update.host_window_id,
            viewport: update.viewport,
            scale: update.scale,
            rounding: update.rounding,
            presence: update.presence,
            visibility: update.visibility,
            focus: update.focus,
            input_routing: update.input_routing,
        })
    }

    /// Returns island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }
    /// Returns consumer-owned kind identity.
    #[must_use]
    pub const fn kind_id(&self) -> &NativeContentKindId {
        &self.kind_id
    }
    /// Returns immutable mechanism capabilities.
    #[must_use]
    pub const fn capabilities(&self) -> MechanismCapabilities {
        self.capabilities
    }
    /// Returns desired-state revision.
    #[must_use]
    pub const fn revision(&self) -> NativeContentRevision {
        self.revision
    }
    /// Returns current desired attach generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }
    /// Returns current host-window binding.
    #[must_use]
    pub const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }
    /// Returns semantic client viewport.
    #[must_use]
    pub const fn viewport(&self) -> ClientRect {
        self.viewport
    }
    /// Returns scale evidence used for this desired apply.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }
    /// Returns explicit physical rounding mode.
    #[must_use]
    pub const fn rounding(&self) -> RoundingMode {
        self.rounding
    }
    /// Returns desired presence.
    #[must_use]
    pub const fn presence(&self) -> DesiredPresence {
        self.presence
    }
    /// Returns desired visibility.
    #[must_use]
    pub const fn visibility(&self) -> &DesiredVisibility {
        &self.visibility
    }
    /// Returns desired focus intent.
    #[must_use]
    pub const fn focus(&self) -> FocusIntent {
        self.focus
    }
    /// Returns desired input route.
    #[must_use]
    pub const fn input_routing(&self) -> InputRoutingMode {
        self.input_routing
    }

    pub(crate) fn replace(&mut self, revision: NativeContentRevision, update: DesiredUpdate) {
        self.revision = revision;
        self.generation = update.generation;
        self.host_window_id = update.host_window_id;
        self.viewport = update.viewport;
        self.scale = update.scale;
        self.rounding = update.rounding;
        self.presence = update.presence;
        self.visibility = update.visibility;
        self.focus = update.focus;
        self.input_routing = update.input_routing;
    }
}

/// Full mutable portion of desired native-content state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct DesiredUpdate {
    pub(crate) generation: AttachGeneration,
    pub(crate) host_window_id: WindowId,
    pub(crate) viewport: ClientRect,
    pub(crate) scale: ScaleFactor,
    pub(crate) rounding: RoundingMode,
    pub(crate) presence: DesiredPresence,
    pub(crate) visibility: DesiredVisibility,
    pub(crate) focus: FocusIntent,
    pub(crate) input_routing: InputRoutingMode,
}

impl DesiredUpdate {
    /// Constructs a complete desired update without hidden defaults.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        generation: AttachGeneration,
        host_window_id: WindowId,
        viewport: ClientRect,
        scale: ScaleFactor,
        rounding: RoundingMode,
        presence: DesiredPresence,
        visibility: DesiredVisibility,
        focus: FocusIntent,
        input_routing: InputRoutingMode,
    ) -> Self {
        Self {
            generation,
            host_window_id,
            viewport,
            scale,
            rounding,
            presence,
            visibility,
            focus,
            input_routing,
        }
    }

    /// Returns requested attach generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }
    pub(crate) const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }
    pub(crate) const fn input_routing(&self) -> InputRoutingMode {
        self.input_routing
    }
}
