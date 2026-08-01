use std::{sync::Arc, time::Duration};

use longhorn_native_content::{AttachGeneration, NativeContentFailureCode, NativeContentIslandId};
use serde::Serialize;

use crate::{HelperSnapshot, IsolatedContentRequest, IsolatedWindowError, IsolatedWindowSpec};

/// Exact bounded owner-process teardown outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TeardownOutcome {
    /// Content closed and its owner exited within the bound.
    Completed {
        /// Observed exit status when available.
        exit_status: Option<i32>,
    },
    /// The deadline elapsed and ownership remains unresolved.
    TimedOut {
        /// Applied deadline in milliseconds.
        timeout_millis: u64,
    },
    /// The disposable owner process was terminated.
    OwnerProcessTerminated {
        /// Observed exit status when available.
        exit_status: Option<i32>,
    },
}

/// Generation-bound event from the injected owner runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsolatedWindowRuntimeEvent {
    /// Shared island identity.
    pub island_id: NativeContentIslandId,
    /// Attach generation that launched the owner.
    pub generation: AttachGeneration,
    /// Product-free event category.
    pub kind: IsolatedWindowRuntimeEventKind,
}

/// Product-free lifecycle evidence from the isolated content owner.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum IsolatedWindowRuntimeEventKind {
    /// Owner reports bounded startup progress.
    Progress {
        /// Consumer-defined bounded startup phase.
        phase: String,
    },
    /// Owner reports consumer-defined readiness and fresh state.
    Ready {
        /// Fresh state at readiness.
        snapshot: HelperSnapshot,
        /// Operating-system disposable owner process id.
        owner_process_id: u32,
        /// Confirms consumer-owned native content was attached.
        native_content_attached: bool,
    },
    /// Content submitted one request for consumer policy admission.
    ContentRequest {
        /// Correlated request awaiting consumer policy.
        request: IsolatedContentRequest,
    },
    /// Effective native focus changed.
    FocusChanged {
        /// Whether native content is focused.
        focused: bool,
    },
    /// Effective native visibility changed.
    VisibilityChanged {
        /// Whether native content is visible.
        visible: bool,
    },
    /// Owner was lost before successful teardown.
    HelperLost {
        /// Stable failure category for this generation.
        code: NativeContentFailureCode,
        /// Observed exit status when available.
        exit_status: Option<i32>,
    },
}

/// Adapter-local lifecycle and ordering evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum IsolatedWindowAdapterEvent {
    /// Generation callback existed before owner launch.
    ListenerInstalled {
        /// Protected generation.
        generation: AttachGeneration,
    },
    /// Owner launch began.
    AttachStarted {
        /// Generation being attached.
        generation: AttachGeneration,
    },
    /// Runtime returned a retained owner handle.
    Attached {
        /// Retained generation.
        generation: AttachGeneration,
    },
    /// One current runtime event was admitted.
    Runtime {
        /// Admitted generation.
        generation: AttachGeneration,
        /// Admitted product-free event.
        event: IsolatedWindowRuntimeEventKind,
    },
    /// A host-driven resize echo was suppressed.
    ResizeCycleSuppressed {
        /// Current generation.
        generation: AttachGeneration,
        /// Echoed host-driven size.
        size: longhorn_core::PhysicalSize,
    },
    /// Bounded owner teardown began.
    DetachStarted {
        /// Generation being detached.
        generation: AttachGeneration,
    },
    /// Bounded teardown returned exact evidence.
    TeardownReported {
        /// Attempted generation.
        generation: AttachGeneration,
        /// Exact bounded teardown result.
        outcome: TeardownOutcome,
    },
}

/// Complete owner launch request with callback installed before launch.
#[derive(Clone)]
pub struct RuntimeAttachRequest {
    /// Exact generation reserved by the adapter.
    pub generation: AttachGeneration,
    /// Complete mapping and timeout policy.
    pub spec: IsolatedWindowSpec,
    /// Callback installed before the owner is created.
    pub callback: Arc<dyn Fn(IsolatedWindowRuntimeEvent) + Send + Sync>,
}

/// Narrow injected port required by isolated-window coordination.
pub trait IsolatedWindowRuntime: Clone + Send + Sync + 'static {
    /// Opaque owner handle retained only inside the selected runtime.
    type Handle: Clone + Send + Sync + 'static;

    /// Launches one disposable owner with callbacks already installed.
    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, IsolatedWindowError>;
    /// Applies physical content-area size without outer placement.
    fn set_content_size(
        &self,
        handle: &Self::Handle,
        size: longhorn_core::PhysicalSize,
        timeout: Duration,
    ) -> Result<(), IsolatedWindowError>;
    /// Shows current native content.
    fn show(&self, handle: &Self::Handle, timeout: Duration) -> Result<(), IsolatedWindowError>;
    /// Hides current native content.
    fn hide(&self, handle: &Self::Handle, timeout: Duration) -> Result<(), IsolatedWindowError>;
    /// Requests native focus.
    fn focus(&self, handle: &Self::Handle, timeout: Duration) -> Result<(), IsolatedWindowError>;
    /// Releases native focus when owned.
    fn release_focus(
        &self,
        handle: &Self::Handle,
        timeout: Duration,
    ) -> Result<(), IsolatedWindowError>;
    /// Applies a consumer-admitted resize hint.
    fn set_resizable(
        &self,
        handle: &Self::Handle,
        resizable: bool,
        timeout: Duration,
    ) -> Result<(), IsolatedWindowError>;
    /// Reads fresh native state.
    fn observe(
        &self,
        handle: &Self::Handle,
        timeout: Duration,
    ) -> Result<HelperSnapshot, IsolatedWindowError>;
    /// Performs bounded teardown, terminating the disposable owner when required.
    fn teardown(
        &self,
        handle: &Self::Handle,
        timeout: Duration,
    ) -> Result<TeardownOutcome, IsolatedWindowError>;
}
