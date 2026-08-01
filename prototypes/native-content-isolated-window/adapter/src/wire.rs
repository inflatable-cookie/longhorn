use longhorn_core::PhysicalSize;
use longhorn_native_content_prototype::AttachGeneration;
use serde::{Deserialize, Serialize};

use crate::{ChildRequest, RuntimeSnapshot};

/// One newline-delimited controller command sent to the disposable helper.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WireCommand {
    /// Correlates the helper acknowledgement.
    pub request_id: u64,
    /// Rejects commands for another attach generation.
    pub generation: AttachGeneration,
    /// Closed helper operation.
    pub command: WireCommandKind,
}

/// Closed command vocabulary for the controlled helper fixture.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WireCommandKind {
    /// Apply physical content size without changing outer origin.
    SetContentSize {
        /// Requested physical content size.
        size: PhysicalSize,
    },
    /// Show the isolated native window.
    Show,
    /// Hide the isolated native window.
    Hide,
    /// Request native focus.
    Focus,
    /// Release native focus when owned.
    ReleaseFocus,
    /// Apply an admitted child resize hint.
    SetResizable {
        /// Whether host-driven resize is enabled.
        resizable: bool,
    },
    /// Ask the fake child to emit one request without applying it.
    ScriptRequest {
        /// Controlled request.
        request: ChildRequest,
    },
    /// Return a fresh native snapshot.
    Observe,
    /// Cooperatively close the fake child and helper.
    Shutdown,
    /// Exit abruptly to prove helper-loss attribution.
    Crash,
}

/// One newline-delimited helper event.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WireEvent {
    /// Attach generation that owns this event.
    pub generation: AttachGeneration,
    /// Closed helper evidence.
    pub event: WireEventKind,
}

/// Closed event vocabulary emitted by the controlled helper fixture.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WireEventKind {
    /// Bounded helper startup progress before readiness.
    Progress {
        /// Stable fixture phase.
        phase: String,
    },
    /// Native window and fake child are attached.
    Ready {
        /// Initial fresh runtime snapshot.
        snapshot: RuntimeSnapshot,
        /// Operating-system helper process id.
        process_id: u32,
        /// Confirms that a real platform child view was inserted.
        native_child_attached: bool,
    },
    /// One command completed or failed.
    Acknowledged {
        /// Correlated request id.
        request_id: u64,
        /// Whether the helper operation succeeded.
        applied: bool,
        /// Optional diagnostic detail on failure.
        detail: Option<String>,
        /// Fresh native state after the operation when available.
        snapshot: Option<RuntimeSnapshot>,
    },
    /// Controlled fake child emitted a request for consumer admission.
    ChildRequest {
        /// Mechanism request with no product payload.
        request: ChildRequest,
    },
    /// Native focus observation changed.
    FocusChanged {
        /// Whether the isolated window is focused.
        focused: bool,
    },
    /// Native visibility observation changed.
    VisibilityChanged {
        /// Whether the isolated window is visible.
        visible: bool,
    },
    /// Cooperative helper teardown completed.
    TeardownCompleted,
    /// Informational native resize observation.
    ContentResized {
        /// Fresh physical content size.
        size: PhysicalSize,
    },
}
