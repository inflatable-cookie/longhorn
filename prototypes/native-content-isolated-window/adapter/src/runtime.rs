use std::{sync::Arc, time::Duration};

use longhorn_core::{PhysicalSize, WindowId};
use longhorn_native_content_prototype::{
    AttachGeneration, NativeContentFailureCode, NativeContentIslandId,
};
use serde::{Deserialize, Serialize};

use crate::IsolatedWindowError;

/// Scriptable request emitted by the controlled fake native child.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ChildRequest {
    /// Propose a new content size without mutating desired state.
    Resize {
        /// Requested physical content size.
        size: PhysicalSize,
    },
    /// Ask consumer policy to show the isolated content.
    Show,
    /// Ask consumer policy to hide the isolated content.
    Hide,
    /// Ask consumer policy to close this generation.
    Close,
    /// Report whether the fake child currently supports host resize.
    ResizeHint {
        /// Proposed native resizable state.
        resizable: bool,
    },
}

/// Generation-bound event from the disposable helper.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HelperEventKind {
    /// Helper reports bounded startup progress before readiness.
    Progress {
        /// Stable fixture phase.
        phase: String,
    },
    /// Helper created the native window and attached the fake child.
    Ready {
        /// Fresh physical content size at readiness.
        content_size: PhysicalSize,
        /// Operating-system helper process id.
        process_id: u32,
        /// Confirms a real platform child view was inserted.
        native_child_attached: bool,
    },
    /// Controlled fake child emitted a consumer-admitted request.
    ChildRequest {
        /// Mechanism-specific request with no product payload.
        request: ChildRequest,
    },
    /// Native focus observation changed.
    FocusChanged {
        /// Whether the isolated content window is focused.
        focused: bool,
    },
    /// Native visibility observation changed.
    VisibilityChanged {
        /// Whether the isolated content window is visible.
        visible: bool,
    },
    /// Helper exited before the adapter completed teardown.
    HelperLost {
        /// Stable failure category for the terminal generation.
        code: NativeContentFailureCode,
        /// Optional platform exit status.
        exit_status: Option<i32>,
    },
}

/// Complete runtime event envelope protected by island and generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelperEvent {
    /// Shared island identity.
    pub island_id: NativeContentIslandId,
    /// Attach generation that launched the helper.
    pub generation: AttachGeneration,
    /// Runtime event category.
    pub kind: HelperEventKind,
}

/// Event emitted by the adapter for lifecycle and ordering evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AdapterEvent {
    /// Generation callback was installed before helper launch.
    ListenerInstalled {
        /// Protected generation.
        generation: AttachGeneration,
    },
    /// Disposable helper launch began.
    AttachStarted {
        /// Generation being attached.
        generation: AttachGeneration,
    },
    /// Runtime returned a usable process handle.
    Attached {
        /// Attached generation.
        generation: AttachGeneration,
    },
    /// A current helper event was admitted.
    Runtime {
        /// Event generation.
        generation: AttachGeneration,
        /// Admitted event.
        event: HelperEventKind,
    },
    /// A host-driven resize echo was suppressed before becoming a proposal.
    ResizeCycleSuppressed {
        /// Current generation.
        generation: AttachGeneration,
        /// Echoed physical size.
        size: PhysicalSize,
    },
    /// Bounded teardown began.
    DetachStarted {
        /// Generation being detached.
        generation: AttachGeneration,
    },
    /// Bounded teardown returned exact evidence.
    TeardownReported {
        /// Attempted generation.
        generation: AttachGeneration,
        /// Completion, timeout, or owner termination evidence.
        outcome: TeardownOutcome,
    },
}

/// Fresh isolated-window state read from the selected runtime.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeSnapshot {
    /// Physical content size excluding outer placement.
    pub content_size: PhysicalSize,
    /// Effective native visibility.
    pub visible: bool,
    /// Effective native focus.
    pub focused: bool,
}

/// Exact bounded helper teardown outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TeardownOutcome {
    /// Helper cooperatively closed and exited within the bound.
    Completed {
        /// Observed exit status when available.
        exit_status: Option<i32>,
    },
    /// Helper missed the deadline; unresolved ownership is explicit.
    TimedOut {
        /// Applied timeout bound.
        timeout_millis: u64,
    },
    /// Adapter terminated the disposable owner process.
    OwnerProcessTerminated {
        /// Observed exit status when available.
        exit_status: Option<i32>,
    },
}

/// Complete helper launch request with callbacks installed before spawn.
#[derive(Clone)]
pub struct RuntimeAttachRequest {
    /// Island identity protected by the helper channel.
    pub island_id: NativeContentIslandId,
    /// Generation reserved before helper launch.
    pub generation: AttachGeneration,
    /// Stable outer-window identity; placement remains consumer-owned.
    pub host_window_id: WindowId,
    /// Callback installed before process creation.
    pub callback: Arc<dyn Fn(HelperEvent) + Send + Sync>,
}

/// Narrow host port required by the isolated-window plan executor.
pub trait IsolatedWindowRuntime: Clone + Send + Sync + 'static {
    /// Opaque helper handle retained only by the selected adapter.
    type Handle: Clone + Send + Sync + 'static;

    /// Launches one disposable helper and attaches its fake native child.
    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, IsolatedWindowError>;
    /// Applies physical content size without changing outer placement.
    fn set_content_size(
        &self,
        handle: &Self::Handle,
        size: PhysicalSize,
    ) -> Result<(), IsolatedWindowError>;
    /// Shows isolated native content.
    fn show(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError>;
    /// Hides isolated native content.
    fn hide(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError>;
    /// Requests native focus.
    fn focus(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError>;
    /// Releases focus only when this helper owns it.
    fn release_focus(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError>;
    /// Applies an admitted mechanism-specific resize hint.
    fn set_resizable(
        &self,
        handle: &Self::Handle,
        resizable: bool,
    ) -> Result<(), IsolatedWindowError>;
    /// Injects a controlled fake-child request for proof work.
    fn script_request(
        &self,
        handle: &Self::Handle,
        request: ChildRequest,
    ) -> Result<(), IsolatedWindowError>;
    /// Terminates the controlled helper unexpectedly for failure evidence.
    fn simulate_helper_loss(
        &self,
        handle: &Self::Handle,
    ) -> Result<Option<i32>, IsolatedWindowError>;
    /// Reads fresh native content state without fabricating outer placement.
    fn observe(&self, handle: &Self::Handle) -> Result<RuntimeSnapshot, IsolatedWindowError>;
    /// Performs bounded disposable-helper teardown.
    fn teardown(
        &self,
        handle: &Self::Handle,
        timeout: Duration,
    ) -> Result<TeardownOutcome, IsolatedWindowError>;
}
