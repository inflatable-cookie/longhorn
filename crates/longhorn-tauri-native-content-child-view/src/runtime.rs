use std::sync::Arc;

use longhorn_core::PhysicalRect;
use longhorn_native_content::{AttachGeneration, NativeContentIslandId};
use serde::Serialize;
use tauri::Url;

use crate::{ChildViewError, ChildViewLabel, ChildViewSpec};

/// Adapter-local lifecycle evidence. This is not a renderer protocol event.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ChildViewAdapterEvent {
    /// Generation-bound runtime callbacks exist before native construction.
    ListenerInstalled {
        /// Protected generation.
        generation: AttachGeneration,
    },
    /// Native construction began.
    AttachStarted {
        /// Generation being attached.
        generation: AttachGeneration,
    },
    /// Native construction returned a retained handle.
    Attached {
        /// Attached generation.
        generation: AttachGeneration,
    },
    /// A generation-bound runtime callback was admitted.
    Runtime {
        /// Callback generation.
        generation: AttachGeneration,
        /// Product-free runtime category.
        event: ChildViewRuntimeEventKind,
    },
    /// Renderer unmount left native content alive.
    RendererUnmounted {
        /// Still-live generation.
        generation: AttachGeneration,
    },
    /// Explicit native close is starting.
    DetachStarted {
        /// Generation being closed.
        generation: AttachGeneration,
    },
    /// Explicit native close completed.
    Detached {
        /// Retired generation.
        generation: AttachGeneration,
    },
    /// Host destruction invalidated local attachment state.
    HostInvalidated {
        /// Invalidated generation.
        generation: AttachGeneration,
    },
}

/// Product-free callback category from the Tauri child runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildViewRuntimeEventKind {
    /// The selected page-load lifecycle began.
    PageLoadStarted,
    /// The selected page-load lifecycle finished.
    PageLoadFinished,
}

/// Complete generation-bound runtime callback envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildViewRuntimeEvent {
    /// Shared island identity.
    pub island_id: NativeContentIslandId,
    /// Attach generation that installed the callback.
    pub generation: AttachGeneration,
    /// Exact child transport label.
    pub child_label: ChildViewLabel,
    /// Product-free callback category.
    pub kind: ChildViewRuntimeEventKind,
}

/// Native construction request assembled from injected policy and plan authority.
#[derive(Clone)]
pub struct RuntimeAttachRequest {
    /// Generation reserved before native creation.
    pub generation: AttachGeneration,
    /// Complete injected construction and security policy.
    pub spec: ChildViewSpec,
    /// Callback installed before native child construction begins.
    pub callback: Arc<dyn Fn(ChildViewRuntimeEvent) + Send + Sync>,
}

/// Narrow runtime port required by the child-only executor.
pub trait ChildViewRuntime: Clone + Send + Sync + 'static {
    /// Opaque runtime handle retained only inside the adapter.
    type Handle: Clone + Send + Sync + 'static;

    /// Creates one initially hidden child with callbacks already installed.
    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, ChildViewError>;
    /// Applies complete physical child bounds.
    fn set_bounds(&self, handle: &Self::Handle, bounds: PhysicalRect)
    -> Result<(), ChildViewError>;
    /// Shows the current child.
    fn show(&self, handle: &Self::Handle) -> Result<(), ChildViewError>;
    /// Hides the current child.
    fn hide(&self, handle: &Self::Handle) -> Result<(), ChildViewError>;
    /// Requests native child focus.
    fn focus(&self, handle: &Self::Handle) -> Result<(), ChildViewError>;
    /// Reads the fresh current child document URL.
    fn current_url(&self, handle: &Self::Handle) -> Result<Url, ChildViewError>;
    /// Submits one already policy-admitted document navigation.
    fn navigate(&self, handle: &Self::Handle, url: Url) -> Result<(), ChildViewError>;
    /// Explicitly closes the current child.
    fn close(&self, handle: &Self::Handle) -> Result<(), ChildViewError>;
    /// Reads fresh physical child bounds.
    fn bounds(&self, handle: &Self::Handle) -> Result<PhysicalRect, ChildViewError>;
}
