//! Lifecycle reports and reveal/shutdown receipts.

use longhorn_core::WindowId;
use longhorn_windowing::WindowLifecycleEventKind;
use serde::{Deserialize, Serialize};

use super::{
    TauriWindowLifecycleAction, TauriWindowLifecycleError, TauriWindowLifecycleReceipt,
    WindowFlushOutcome,
};

/// Asynchronously reported listener result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowLifecycleReport {
    window_id: WindowId,
    event: Option<WindowLifecycleEventKind>,
    result: Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError>,
}

impl WindowLifecycleReport {
    pub(crate) const fn new(
        window_id: WindowId,
        event: Option<WindowLifecycleEventKind>,
        result: Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError>,
    ) -> Self {
        Self {
            window_id,
            event,
            result,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns translated input category when translation succeeded.
    #[must_use]
    pub const fn event(&self) -> Option<WindowLifecycleEventKind> {
        self.event
    }

    /// Returns the typed listener result.
    pub const fn result(&self) -> &Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError> {
        &self.result
    }
}

/// Reveal gate state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowRevealStatus {
    /// One or both explicit gates remain false.
    Waiting {
        /// Consumer page-ready signal.
        page_ready: bool,
        /// Successful hidden-placement readback.
        placement_ready: bool,
    },
    /// Native show succeeded after both gates.
    Revealed,
    /// Window was already revealed.
    AlreadyRevealed,
    /// Native show failed.
    Failed {
        /// Host diagnostic.
        detail: String,
    },
}

/// Reveal transition receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowRevealReceipt {
    window_id: WindowId,
    status: WindowRevealStatus,
}

impl WindowRevealReceipt {
    pub(crate) const fn new(window_id: WindowId, status: WindowRevealStatus) -> Self {
        Self { window_id, status }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns gate or native-show result.
    #[must_use]
    pub const fn status(&self) -> &WindowRevealStatus {
        &self.status
    }
}

/// Complete bounded aggregate shutdown result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowShutdownReceipt {
    actions: Vec<TauriWindowLifecycleAction>,
    flush: Option<WindowFlushOutcome>,
}

impl WindowShutdownReceipt {
    pub(crate) const fn new(
        actions: Vec<TauriWindowLifecycleAction>,
        flush: Option<WindowFlushOutcome>,
    ) -> Self {
        Self { actions, flush }
    }

    /// Returns capture and scheduling work performed before aggregate flush.
    #[must_use]
    pub fn actions(&self) -> &[TauriWindowLifecycleAction] {
        &self.actions
    }

    /// Returns aggregate sink outcome when at least one target was flushable.
    #[must_use]
    pub const fn flush(&self) -> Option<&WindowFlushOutcome> {
        self.flush.as_ref()
    }
}
