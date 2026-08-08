//! Native mechanism selection and capabilities.

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

