use longhorn_core::{PhysicalPoint, PhysicalRect, ScaleFactor, WindowId};
use longhorn_native_content_prototype::{
    AttachGeneration, InputRoutingMode, NativeContentIslandId,
};
use serde::{Deserialize, Serialize};

use crate::BackingSurfaceError;

/// Complete consumer-supplied backing-surface launch request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeAttachRequest {
    /// Shared island identity.
    pub island_id: NativeContentIslandId,
    /// Attach generation protected by the runtime handle.
    pub generation: AttachGeneration,
    /// Stable host-window binding.
    pub host_window_id: WindowId,
}

/// Fresh native and renderer evidence for one backing surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSnapshot {
    /// Full native backing storage bounds in physical pixels.
    pub storage_bounds: PhysicalRect,
    /// Current physical presentation and interaction clip.
    pub clip: PhysicalRect,
    /// Whether desired presentation is enabled independently of clip area.
    pub presentation_enabled: bool,
    /// Current adapter input route.
    pub input_routing: InputRoutingMode,
    /// Fresh native host scale.
    pub native_scale: ScaleFactor,
    /// Whether the controlled native view still has its expected superview.
    pub native_view_attached: bool,
    /// Consumer renderer frame sequence produced from this state.
    pub frame_sequence: u64,
}

/// Declared backing-view detach receipt.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetachOutcome {
    /// The controlled backing view was removed and released.
    Detached,
    /// Native ownership intentionally remains until process exit.
    RetainedForProcessLifetime,
}

/// Current-generation native callback category.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RuntimeEventKind {
    /// A consumer renderer produced a clipped frame.
    FramePresented {
        /// Monotonic consumer-owned frame sequence.
        sequence: u64,
    },
    /// Fresh full-host native storage changed.
    StorageChanged {
        /// Complete physical storage bounds.
        bounds: PhysicalRect,
    },
}

/// Generation-bound runtime callback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEvent {
    /// Island identity.
    pub island_id: NativeContentIslandId,
    /// Host-window binding.
    pub host_window_id: WindowId,
    /// Attach generation.
    pub generation: AttachGeneration,
    /// Native or renderer observation.
    pub kind: RuntimeEventKind,
}

/// Adapter lifecycle evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AdapterEvent {
    /// Native attachment began.
    AttachStarted {
        /// Target generation.
        generation: AttachGeneration,
    },
    /// Native attachment completed with fresh full-host evidence.
    Attached {
        /// Attached generation.
        generation: AttachGeneration,
    },
    /// A current runtime callback was admitted.
    Runtime {
        /// Event generation.
        generation: AttachGeneration,
        /// Admitted event.
        event: RuntimeEventKind,
    },
    /// Host destruction invalidated callback authority before native release.
    HostInvalidated {
        /// Invalidated generation.
        generation: AttachGeneration,
    },
    /// Declared detach returned exact evidence.
    Detached {
        /// Detached generation.
        generation: AttachGeneration,
        /// Selected lifecycle receipt.
        outcome: DetachOutcome,
    },
}

/// Input gate result before any consumer semantic callback runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "reason")]
pub enum InputAdmission {
    /// The consumer may invoke its own typed semantic callback.
    Admitted,
    /// No consumer callback may run.
    Rejected(InputRejection),
}

/// Exact reason a renderer-forwarded sample was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRejection {
    /// Desired visibility disabled presentation.
    PresentationDisabled,
    /// Current viewport has no physical area.
    EmptyViewport,
    /// The sample lies outside the current clip.
    OutsideViewport,
    /// Consumer-supplied host focus evidence is false.
    HostUnfocused,
    /// The current route is not renderer-forwarded.
    RoutingDisabled,
}

/// Narrow consumer port required by the backing-surface plan executor.
pub trait BackingSurfaceRuntime: Clone + Send + Sync + 'static {
    /// Opaque handle retained only by the selected runtime.
    type Handle: Clone + Send + Sync + 'static;

    /// Attaches consumer-supplied backing storage and returns fresh evidence.
    fn attach(
        &self,
        request: RuntimeAttachRequest,
    ) -> Result<(Self::Handle, RuntimeSnapshot), BackingSurfaceError>;
    /// Changes only the presentation and interaction clip.
    fn set_viewport(
        &self,
        handle: &Self::Handle,
        clip: PhysicalRect,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError>;
    /// Enables or suppresses consumer rendering without detaching storage.
    fn set_presentation_enabled(
        &self,
        handle: &Self::Handle,
        enabled: bool,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError>;
    /// Changes only the declared common input route.
    fn set_input_routing(
        &self,
        handle: &Self::Handle,
        mode: InputRoutingMode,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError>;
    /// Refreshes full-host native geometry and consumer render evidence.
    fn refresh(&self, handle: &Self::Handle) -> Result<RuntimeSnapshot, BackingSurfaceError>;
    /// Applies the declared detach policy.
    fn detach(&self, handle: &Self::Handle) -> Result<DetachOutcome, BackingSurfaceError>;
}

pub(crate) fn contains(rect: &PhysicalRect, point: PhysicalPoint) -> bool {
    rect.contains_point(&point)
}
