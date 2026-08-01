use std::sync::Arc;

use longhorn_core::PhysicalRect;
use longhorn_native_content_prototype::{AttachGeneration, NativeContentIslandId};
use serde::Serialize;

use crate::{ChildWebviewError, ChildWebviewSpec};

/// Event emitted by the adapter for ordering and lifecycle evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AdapterEvent {
    /// Runtime callbacks were constructed before native creation began.
    ListenerInstalled {
        /// Generation protected by the listener.
        generation: AttachGeneration,
    },
    /// Native construction began.
    AttachStarted {
        /// Generation being created.
        generation: AttachGeneration,
    },
    /// Native construction returned a usable handle.
    Attached {
        /// Attached generation.
        generation: AttachGeneration,
    },
    /// A generation-bound runtime callback was admitted.
    Runtime {
        /// Callback generation.
        generation: AttachGeneration,
        /// Runtime event category.
        event: RuntimeEventKind,
    },
    /// Renderer unmount was observed without native destruction.
    RendererUnmounted {
        /// Still-live generation.
        generation: AttachGeneration,
    },
    /// Explicit native close is about to begin for the current generation.
    DetachStarted {
        /// Generation whose retained handle will be closed.
        generation: AttachGeneration,
    },
    /// Explicit reversible close completed.
    Detached {
        /// Closed generation.
        generation: AttachGeneration,
    },
    /// Host destruction invalidated native attachment authority.
    HostInvalidated {
        /// Invalidated generation.
        generation: AttachGeneration,
    },
}

/// Generation-bound callback category from the selected Tauri mechanism.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RuntimeEventKind {
    /// Navigation policy accepted or rejected a URL.
    Navigation {
        /// Candidate URL.
        url: String,
        /// Injected consumer decision.
        allowed: bool,
    },
    /// A page load began.
    PageLoadStarted {
        /// Page URL.
        url: String,
    },
    /// A page load finished and may satisfy consumer readiness.
    PageLoadFinished {
        /// Page URL.
        url: String,
    },
    /// Closed popup policy rejected a new-window request.
    PopupDenied {
        /// Requested URL.
        url: String,
    },
    /// Closed download policy rejected a request.
    DownloadDenied {
        /// Requested URL.
        url: String,
    },
}

/// Complete runtime callback envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEvent {
    /// Shared island identity.
    pub island_id: NativeContentIslandId,
    /// Attach generation that installed the callback.
    pub generation: AttachGeneration,
    /// Exact child transport label.
    pub webview_label: String,
    /// Runtime event category.
    pub kind: RuntimeEventKind,
}

/// Native construction request assembled from consumer policy and plan authority.
#[derive(Clone)]
pub struct RuntimeAttachRequest {
    /// Generation reserved before native creation.
    pub generation: AttachGeneration,
    /// Complete construction and security policy.
    pub spec: ChildWebviewSpec,
    /// Callback installed into the builder before `add_child`.
    pub callback: Arc<dyn Fn(RuntimeEvent) + Send + Sync>,
}

/// Narrow host port required by the child-only plan executor.
pub trait ChildWebviewRuntime: Clone + Send + Sync + 'static {
    /// Opaque runtime handle retained only inside the selected adapter.
    type Handle: Clone + Send + Sync + 'static;

    /// Creates one initially hidden child with callbacks already installed.
    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, ChildWebviewError>;
    /// Applies complete physical child bounds.
    fn set_bounds(
        &self,
        handle: &Self::Handle,
        bounds: PhysicalRect,
    ) -> Result<(), ChildWebviewError>;
    /// Shows the current child.
    fn show(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError>;
    /// Hides the current child.
    fn hide(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError>;
    /// Requests native child focus.
    fn focus(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError>;
    /// Explicitly closes the current child.
    fn close(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError>;
    /// Reads fresh physical child bounds from the native runtime.
    fn bounds(&self, handle: &Self::Handle) -> Result<PhysicalRect, ChildWebviewError>;
    /// Evaluates a controlled proof probe without adding renderer authority.
    fn evaluate(&self, handle: &Self::Handle, script: &str) -> Result<(), ChildWebviewError>;
}
