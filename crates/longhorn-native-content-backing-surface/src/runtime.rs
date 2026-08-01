use std::sync::Arc;

use longhorn_core::{PhysicalPoint, PhysicalRect, ScaleFactor};
use longhorn_native_content::{AttachGeneration, InputRoutingMode, NativeContentIslandId};
use serde::Serialize;

use crate::{BackingSurfaceError, BackingSurfaceSpec};

/// Fresh native-storage and consumer-renderer evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackingSurfaceSnapshot {
    /// Full native backing storage bounds in physical pixels.
    pub storage_bounds: PhysicalRect,
    /// Current physical output and interaction clip.
    pub clip: PhysicalRect,
    /// Whether consumer rendering is enabled independently of clip area.
    pub presentation_enabled: bool,
    /// Current product-free input route.
    pub input_routing: InputRoutingMode,
    /// Fresh native host scale.
    pub native_scale: ScaleFactor,
    /// Whether native storage remains attached to its expected host.
    pub native_storage_attached: bool,
    /// Monotonic consumer-renderer frame sequence.
    pub frame_sequence: u64,
}

/// Product-free current-generation runtime callback category.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BackingSurfaceRuntimeEventKind {
    /// Consumer rendering completed one frame using the current clip.
    FramePresented {
        /// Monotonic consumer-owned frame sequence.
        sequence: u64,
    },
    /// Fresh full-host native storage bounds changed.
    StorageChanged {
        /// Complete current physical storage bounds.
        bounds: PhysicalRect,
    },
}

/// Generation- and host-bound runtime callback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackingSurfaceRuntimeEvent {
    /// Shared island identity.
    pub island_id: NativeContentIslandId,
    /// Stable logical host binding.
    pub host_window_id: longhorn_core::WindowId,
    /// Attach generation that installed the callback.
    pub generation: AttachGeneration,
    /// Monotonic event sequence within the attach generation.
    pub sequence: u64,
    /// Product-free observation.
    pub kind: BackingSurfaceRuntimeEventKind,
}

/// Adapter-local lifecycle and ordering evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BackingSurfaceAdapterEvent {
    /// Generation callback existed before native attachment.
    ListenerInstalled {
        /// Protected generation.
        generation: AttachGeneration,
    },
    /// Native storage and renderer attachment began.
    AttachStarted {
        /// Generation being attached.
        generation: AttachGeneration,
    },
    /// Runtime returned retained storage and fresh evidence.
    Attached {
        /// Attached generation.
        generation: AttachGeneration,
    },
    /// One current runtime callback was admitted.
    Runtime {
        /// Admitted generation.
        generation: AttachGeneration,
        /// Admitted product-free event.
        event: BackingSurfaceRuntimeEventKind,
    },
    /// Host destruction invalidated callbacks before native detach.
    HostInvalidated {
        /// Invalidated generation.
        generation: AttachGeneration,
    },
    /// Reversible native detach began.
    DetachStarted {
        /// Generation being detached.
        generation: AttachGeneration,
    },
    /// Reversible native detach completed.
    Detached {
        /// Retired generation.
        generation: AttachGeneration,
    },
}

/// Result of physical-point admission before semantic consumer dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "reason")]
pub enum InputAdmission {
    /// The consumer may dispatch its own typed semantic input.
    Admitted,
    /// No consumer semantic callback may run.
    Rejected(InputRejection),
}

/// Exact reason physical input did not pass the backing-surface gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRejection {
    /// Desired visibility disabled presentation.
    PresentationDisabled,
    /// Current clip or storage has no area.
    EmptyViewport,
    /// The sample lies outside the semantic viewport.
    OutsideViewport,
    /// The sample lies outside current backing storage.
    OutsideStorage,
    /// Consumer-supplied host focus evidence is false.
    HostUnfocused,
    /// Current routing is disabled.
    RoutingDisabled,
}

/// Complete attach request with runtime callbacks installed before attachment.
#[derive(Clone)]
pub struct RuntimeAttachRequest {
    /// Exact generation reserved by the adapter.
    pub generation: AttachGeneration,
    /// Complete island and host mapping.
    pub spec: BackingSurfaceSpec,
    /// Callback installed before native storage or renderer attachment.
    pub callback: Arc<dyn Fn(BackingSurfaceRuntimeEvent) + Send + Sync>,
}

/// Consumer-owned storage, renderer-lifecycle, and clipping port.
pub trait BackingSurfaceRuntime: Clone + Send + Sync + 'static {
    /// Opaque storage/renderer handle retained only inside the adapter.
    type Handle: Clone + Send + Sync + 'static;

    /// Attaches consumer storage and renderer with callbacks already installed.
    fn attach(
        &self,
        request: RuntimeAttachRequest,
    ) -> Result<(Self::Handle, BackingSurfaceSnapshot), BackingSurfaceError>;
    /// Applies only the output and interaction clip.
    fn set_viewport(
        &self,
        handle: &Self::Handle,
        clip: PhysicalRect,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError>;
    /// Enables or suppresses rendering without detaching storage.
    fn set_presentation_enabled(
        &self,
        handle: &Self::Handle,
        enabled: bool,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError>;
    /// Applies only the product-free input route.
    fn set_input_routing(
        &self,
        handle: &Self::Handle,
        mode: InputRoutingMode,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError>;
    /// Reads fresh full-host storage, clip, route, and render evidence.
    fn observe(&self, handle: &Self::Handle)
    -> Result<BackingSurfaceSnapshot, BackingSurfaceError>;
    /// Reversibly removes consumer storage and renderer resources.
    fn detach(&self, handle: &Self::Handle) -> Result<(), BackingSurfaceError>;
}

pub(crate) fn contains(rect: &PhysicalRect, point: PhysicalPoint) -> bool {
    rect.contains_point(&point)
}
