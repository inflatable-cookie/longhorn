//! Scheduled wakes and flush requests.

use crate::{
    CaptureGeneration, FlushReason, MonotonicMillis, WindowLifecycleDuration, WindowLifecycleEvent,
};
use longhorn_core::WindowId;
use serde::{Deserialize, Serialize};

/// One host wake requested by the pure lifecycle coordinator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScheduledWindowLifecycleWake {
    due_at: MonotonicMillis,
    event: WindowLifecycleEvent,
}

impl ScheduledWindowLifecycleWake {
    /// Records one scheduled wake. Public because host adapters construct
    /// wakes; the type moved out of the Tauri crate with the rest of the
    /// host-agnostic lifecycle model.
    #[must_use]
    pub const fn new(due_at: MonotonicMillis, event: WindowLifecycleEvent) -> Self {
        Self { due_at, event }
    }

    /// Returns the caller-clock deadline.
    #[must_use]
    pub const fn due_at(&self) -> MonotonicMillis {
        self.due_at
    }

    /// Returns the event to deliver when the deadline fires.
    #[must_use]
    pub const fn event(&self) -> &WindowLifecycleEvent {
        &self.event
    }
}

/// One window included in a sink flush request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowFlushTarget {
    window_id: WindowId,
    generation: Option<CaptureGeneration>,
}

impl WindowFlushTarget {
    /// Constructs one exact sink flush target.
    #[must_use]
    pub const fn new(window_id: WindowId, generation: Option<CaptureGeneration>) -> Self {
        Self {
            window_id,
            generation,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns captured generation when known.
    #[must_use]
    pub const fn generation(&self) -> Option<CaptureGeneration> {
        self.generation
    }
}

/// Why and how widely the sink should flush.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowFlushScope {
    /// One Card 019 directive.
    Window {
        /// Pure coordinator reason.
        reason: FlushReason,
    },
    /// One bounded application-shutdown aggregate.
    ApplicationShutdown,
}

/// Exact bounded sink flush request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowFlushRequest {
    targets: Vec<WindowFlushTarget>,
    timeout: WindowLifecycleDuration,
    scope: WindowFlushScope,
}

impl WindowFlushRequest {
    /// Constructs one bounded sink request.
    #[must_use]
    pub fn new(
        targets: Vec<WindowFlushTarget>,
        timeout: WindowLifecycleDuration,
        scope: WindowFlushScope,
    ) -> Self {
        Self {
            targets,
            timeout,
            scope,
        }
    }

    /// Returns stable sorted flush targets.
    #[must_use]
    pub fn targets(&self) -> &[WindowFlushTarget] {
        &self.targets
    }

    /// Returns the maximum wait.
    #[must_use]
    pub const fn timeout(&self) -> WindowLifecycleDuration {
        self.timeout
    }

    /// Returns single-window or shutdown scope.
    #[must_use]
    pub const fn scope(&self) -> WindowFlushScope {
        self.scope
    }
}

/// Bounded flush terminal result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowFlushOutcome {
    /// Sink acknowledged success within the bound.
    Succeeded,
    /// Sink refused to start the request.
    RequestFailed {
        /// Sink diagnostic.
        detail: String,
    },
    /// Sink completed with failure.
    SinkFailed {
        /// Sink diagnostic.
        detail: String,
    },
    /// No acknowledgement arrived within the exact bound.
    TimedOut,
    /// Acknowledgement channel closed without a result.
    Disconnected,
}
