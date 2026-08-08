//! Lifecycle actions, receipts, and errors.

use longhorn_core::WindowId;
use longhorn_windowing::{
    ApplyGeneration, CaptureGeneration, CaptureReason, IgnoreReason, WindowLifecycleEventKind,
};
use serde::{Deserialize, Serialize};

use super::{
    CapturedWindowPlacement, ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowFlushRequest,
};

/// One executed adapter action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TauriWindowLifecycleAction {
    /// Pure coordinator intentionally emitted no work.
    Ignored {
        /// Inspectable classification.
        reason: IgnoreReason,
    },
    /// Host accepted a capture or flush wake.
    Scheduled {
        /// Exact wake.
        wake: ScheduledWindowLifecycleWake,
    },
    /// Host scheduler rejected a wake.
    ScheduleFailed {
        /// Exact rejected wake.
        wake: ScheduledWindowLifecycleWake,
        /// Scheduler diagnostic.
        detail: String,
    },
    /// Complete placement was accepted by the injected sink.
    PlacementStaged {
        /// Capture generation.
        generation: CaptureGeneration,
        /// Capture trigger.
        reason: CaptureReason,
        /// Exact schema-opaque proposal.
        placement: CapturedWindowPlacement,
    },
    /// Complete native capture failed.
    CaptureFailed {
        /// Capture generation.
        generation: CaptureGeneration,
        /// Capture diagnostic.
        detail: String,
    },
    /// Sink rejected captured placement.
    PersistenceFailed {
        /// Capture generation.
        generation: CaptureGeneration,
        /// Sink diagnostic.
        detail: String,
    },
    /// One bounded flush reached a terminal outcome.
    Flushed {
        /// Exact request.
        request: WindowFlushRequest,
        /// Terminal outcome.
        outcome: WindowFlushOutcome,
    },
    /// One bounded flush left the event thread; its terminal outcome arrives
    /// as a later reporter receipt.
    FlushDeferred {
        /// Exact request.
        request: WindowFlushRequest,
    },
    /// Consumer user-close callback completed.
    UserCloseReported,
    /// Consumer user-close callback failed.
    UserCloseFailed {
        /// Consumer diagnostic.
        detail: String,
    },
    /// Destroy removed listener/capture state.
    Forgotten,
}

/// Complete result for one native or scheduled lifecycle input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TauriWindowLifecycleReceipt {
    window_id: WindowId,
    event: WindowLifecycleEventKind,
    actions: Vec<TauriWindowLifecycleAction>,
}

impl TauriWindowLifecycleReceipt {
    pub(crate) const fn new(
        window_id: WindowId,
        event: WindowLifecycleEventKind,
        actions: Vec<TauriWindowLifecycleAction>,
    ) -> Self {
        Self {
            window_id,
            event,
            actions,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns processed input category.
    #[must_use]
    pub const fn event(&self) -> WindowLifecycleEventKind {
        self.event
    }

    /// Returns ordered action outcomes.
    #[must_use]
    pub fn actions(&self) -> &[TauriWindowLifecycleAction] {
        &self.actions
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        WindowId,
        WindowLifecycleEventKind,
        Vec<TauriWindowLifecycleAction>,
    ) {
        (self.window_id, self.event, self.actions)
    }
}

/// Fatal adapter failure before a complete receipt was possible.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TauriWindowLifecycleError {
    /// Teardown has deactivated this host.
    InactiveHost,
    /// Stable id has no installed window.
    UnknownWindow {
        /// Missing identity.
        window_id: WindowId,
    },
    /// Native handle has no installed lifecycle window.
    UnknownWindowHandle {
        /// Missing transport identity.
        transport_handle: longhorn_windowing::HostWindowHandle,
    },
    /// Stable id already has an installed window.
    DuplicateWindow {
        /// Repeated identity.
        window_id: WindowId,
    },
    /// Native label is not a valid host transport handle.
    InvalidWindowLabel {
        /// Validation diagnostic.
        detail: String,
    },
    /// Shared state lock was poisoned.
    StateUnavailable {
        /// State category.
        state: String,
    },
    /// Scheduler could not bind the shared host target.
    SchedulerBinding {
        /// Scheduler diagnostic.
        detail: String,
    },
    /// Native event conversion failed.
    EventTranslation {
        /// Conversion diagnostic.
        detail: String,
    },
    /// Pure lifecycle coordinator rejected arithmetic or generation input.
    Coordination {
        /// Coordinator diagnostic.
        detail: String,
    },
    /// Apply evidence could not be installed before native mutation.
    ApplyRegistration {
        /// Apply generation.
        generation: ApplyGeneration,
        /// Registration diagnostic.
        detail: String,
    },
}
