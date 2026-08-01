use longhorn_core::{
    ClientRect, NativeContentIslandId, NativeContentKindId, NativeContentRevision, PhysicalRect,
    PhysicalSize, RoundingMode, ScaleFactor, VisibilityReasonId, WindowId,
};
use serde::{Deserialize, Serialize};

use crate::{AttachGeneration, CoordinationError};

/// Native host mechanism selected for one island.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NativeContentMechanism {
    /// A child view whose native bounds follow the semantic viewport.
    ChildView,
    /// An independently placed native window whose content size follows the viewport.
    IsolatedWindow,
    /// Native storage that may fill the host while rendering uses a viewport clip.
    BackingSurface,
}

/// Declared native detach behavior for one mechanism.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DetachPolicy {
    /// The adapter can detach and release the native content directly.
    Reversible,
    /// Safe detach requires terminating the owner process.
    OwnerProcessTermination,
    /// Native ownership intentionally lasts for the process lifetime.
    ProcessLifetime,
}

/// Declared route for content input.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum InputRoutingMode {
    /// Native content receives platform input directly.
    NativeDirect,
    /// Trusted renderer chrome forwards consumer-defined semantics.
    RendererForwarded,
    /// Content input is disabled.
    Disabled,
}

/// Honest capabilities of one selected mechanism adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct MechanismCapabilities {
    mechanism: NativeContentMechanism,
    active_input_routing: InputRoutingMode,
    accepts_content_size_requests: bool,
    detach_policy: DetachPolicy,
    observes_visibility: bool,
    observes_focus: bool,
}

impl MechanismCapabilities {
    /// Constructs a complete mechanism description with no inferred capabilities.
    #[must_use]
    pub const fn new(
        mechanism: NativeContentMechanism,
        active_input_routing: InputRoutingMode,
        accepts_content_size_requests: bool,
        detach_policy: DetachPolicy,
        observes_visibility: bool,
        observes_focus: bool,
    ) -> Self {
        Self {
            mechanism,
            active_input_routing,
            accepts_content_size_requests,
            detach_policy,
            observes_visibility,
            observes_focus,
        }
    }

    /// Returns the selected host mechanism.
    #[must_use]
    pub const fn mechanism(self) -> NativeContentMechanism {
        self.mechanism
    }

    /// Returns the only enabled input route supported by this adapter.
    #[must_use]
    pub const fn active_input_routing(self) -> InputRoutingMode {
        self.active_input_routing
    }

    /// Returns whether content may propose a new content size.
    #[must_use]
    pub const fn accepts_content_size_requests(self) -> bool {
        self.accepts_content_size_requests
    }

    /// Returns declared detach behavior.
    #[must_use]
    pub const fn detach_policy(self) -> DetachPolicy {
        self.detach_policy
    }

    /// Returns whether effective visibility can be observed.
    #[must_use]
    pub const fn observes_visibility(self) -> bool {
        self.observes_visibility
    }

    /// Returns whether native focus can be observed.
    #[must_use]
    pub const fn observes_focus(self) -> bool {
        self.observes_focus
    }

    pub(crate) fn validate_input(self, mode: InputRoutingMode) -> Result<(), CoordinationError> {
        if mode == InputRoutingMode::Disabled || mode == self.active_input_routing {
            Ok(())
        } else {
            Err(CoordinationError::UnsupportedInputRouting {
                supported: self.active_input_routing,
                supplied: mode,
            })
        }
    }
}

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

/// Complete desired state for one native-content island.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(try_from = "DesiredStateWire")]
pub struct DesiredState {
    island_id: NativeContentIslandId,
    kind_id: NativeContentKindId,
    capabilities: MechanismCapabilities,
    revision: NativeContentRevision,
    generation: AttachGeneration,
    host_window_id: WindowId,
    viewport: ClientRect,
    scale: ScaleFactor,
    rounding: RoundingMode,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    focus: FocusIntent,
    input_routing: InputRoutingMode,
}

#[derive(Deserialize)]
struct DesiredStateWire {
    island_id: NativeContentIslandId,
    kind_id: NativeContentKindId,
    capabilities: MechanismCapabilities,
    revision: NativeContentRevision,
    generation: AttachGeneration,
    host_window_id: WindowId,
    viewport: ClientRect,
    scale: ScaleFactor,
    rounding: RoundingMode,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    focus: FocusIntent,
    input_routing: InputRoutingMode,
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
    generation: AttachGeneration,
    host_window_id: WindowId,
    viewport: ClientRect,
    scale: ScaleFactor,
    rounding: RoundingMode,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    focus: FocusIntent,
    input_routing: InputRoutingMode,
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

/// Current observed native state for one island.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ObservedState {
    revision: NativeContentRevision,
    generation: AttachGeneration,
    lifecycle: AttachmentLifecycle,
    readiness: ObservedReadiness,
    visibility: EffectiveVisibility,
    focus: EffectiveFocus,
    geometry: ObservedGeometry,
    input_routing: Option<InputRoutingMode>,
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
    generation: AttachGeneration,
    lifecycle: AttachmentLifecycle,
    readiness: ObservedReadiness,
    visibility: EffectiveVisibility,
    focus: EffectiveFocus,
    geometry: ObservedGeometry,
    input_routing: Option<InputRoutingMode>,
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
