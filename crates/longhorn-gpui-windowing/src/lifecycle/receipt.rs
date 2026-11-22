use longhorn_core::WindowId;
use longhorn_windowing::{
    CaptureGeneration, ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowFlushRequest,
    WindowLifecycleEventKind,
};

/// The answer GPUI's `on_should_close` needs, synchronously.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuiCloseDecision {
    /// Longhorn has no outstanding work; GPUI may close the window.
    Close,
    /// Longhorn is not ready; GPUI must keep the window open.
    Defer,
}

impl GpuiCloseDecision {
    /// Returns the boolean GPUI's callback expects.
    #[must_use]
    pub const fn should_close(self) -> bool {
        matches!(self, Self::Close)
    }
}

/// One externally visible thing the host did for a lifecycle input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiLifecycleAction {
    /// No external work ran, with the coordinator's reason.
    Ignored {
        /// Inspectable reason.
        reason: longhorn_windowing::IgnoreReason,
    },
    /// A deadline was accepted by the scheduler.
    Scheduled {
        /// The accepted wake.
        wake: Box<ScheduledWindowLifecycleWake>,
    },
    /// A deadline was refused by the scheduler.
    ScheduleFailed {
        /// Scheduler diagnostic.
        detail: String,
    },
    /// Live state was captured and staged.
    Captured {
        /// The staged capture generation.
        generation: CaptureGeneration,
    },
    /// Capture or staging failed.
    CaptureFailed {
        /// Boundary diagnostic.
        detail: String,
    },
    /// A bounded flush ran to a terminal result.
    Flushed {
        /// The request as issued.
        request: Box<WindowFlushRequest>,
        /// Terminal outcome.
        outcome: WindowFlushOutcome,
    },
    /// User-close policy accepted the close.
    UserCloseReported,
    /// User-close policy refused the close.
    UserCloseFailed {
        /// Policy diagnostic.
        detail: String,
    },
    /// Coordinator state for a destroyed window was released.
    Forgotten,
}

/// Complete result of one lifecycle input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiLifecycleReceipt {
    window_id: WindowId,
    event_kind: WindowLifecycleEventKind,
    actions: Vec<GpuiLifecycleAction>,
}

impl GpuiLifecycleReceipt {
    pub(super) const fn new(
        window_id: WindowId,
        event_kind: WindowLifecycleEventKind,
        actions: Vec<GpuiLifecycleAction>,
    ) -> Self {
        Self {
            window_id,
            event_kind,
            actions,
        }
    }

    /// Returns the logical target.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the originating input category.
    #[must_use]
    pub const fn event_kind(&self) -> WindowLifecycleEventKind {
        self.event_kind
    }

    /// Returns every externally visible action, in order.
    #[must_use]
    pub fn actions(&self) -> &[GpuiLifecycleAction] {
        &self.actions
    }
}

/// Returns whether one close-request receipt leaves nothing unresolved.
///
/// The Tauri host does not need this: it prevents every user close and lets
/// product policy decide afterwards. GPUI demands an answer inside the
/// callback, so the adapter has to have one, and a refusal from product policy
/// or a flush that did not reach the sink are both reasons to keep the window
/// open.
#[must_use]
pub fn close_is_safe(receipt: &GpuiLifecycleReceipt) -> bool {
    receipt.actions().iter().all(|action| match action {
        GpuiLifecycleAction::CaptureFailed { .. }
        | GpuiLifecycleAction::ScheduleFailed { .. }
        | GpuiLifecycleAction::UserCloseFailed { .. } => false,
        GpuiLifecycleAction::Flushed { outcome, .. } => {
            matches!(outcome, WindowFlushOutcome::Succeeded)
        }
        GpuiLifecycleAction::Ignored { .. }
        | GpuiLifecycleAction::Scheduled { .. }
        | GpuiLifecycleAction::Captured { .. }
        | GpuiLifecycleAction::UserCloseReported
        | GpuiLifecycleAction::Forgotten => true,
    })
}
