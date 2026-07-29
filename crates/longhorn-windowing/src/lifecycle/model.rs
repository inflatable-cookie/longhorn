use longhorn_core::{ScaleFactor, ScreenPoint, ScreenSize, WindowId};
use serde::{Deserialize, Serialize};

use super::{MonotonicMillis, WindowLifecycleDuration};
use crate::{ApplyGeneration, WindowOperationKind};

/// Complete caller-owned timing policy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowLifecyclePolicy {
    programmatic_attribution: WindowLifecycleDuration,
    user_precedence: WindowLifecycleDuration,
    settle_delay: WindowLifecycleDuration,
    persistence_debounce: WindowLifecycleDuration,
    flush_timeout: WindowLifecycleDuration,
}

impl WindowLifecyclePolicy {
    /// Returns Longhorn's opt-in desktop baseline.
    ///
    /// Consumers still own timing policy and may deviate explicitly. This is
    /// not a `Default` implementation because selecting it is a product choice.
    #[must_use]
    pub const fn recommended() -> Self {
        Self::new(
            WindowLifecycleDuration::from_millis(400),
            WindowLifecycleDuration::from_millis(400),
            WindowLifecycleDuration::from_millis(200),
            WindowLifecycleDuration::from_millis(500),
            WindowLifecycleDuration::from_millis(2_000),
        )
    }

    /// Constructs exact attribution, settling, debounce, and flush bounds.
    #[must_use]
    pub const fn new(
        programmatic_attribution: WindowLifecycleDuration,
        user_precedence: WindowLifecycleDuration,
        settle_delay: WindowLifecycleDuration,
        persistence_debounce: WindowLifecycleDuration,
        flush_timeout: WindowLifecycleDuration,
    ) -> Self {
        Self {
            programmatic_attribution,
            user_precedence,
            settle_delay,
            persistence_debounce,
            flush_timeout,
        }
    }

    /// Returns the programmatic native-event attribution window.
    #[must_use]
    pub const fn programmatic_attribution(self) -> WindowLifecycleDuration {
        self.programmatic_attribution
    }

    /// Returns how long native user input outranks later apply evidence.
    #[must_use]
    pub const fn user_precedence(self) -> WindowLifecycleDuration {
        self.user_precedence
    }

    /// Returns the quiet period before complete live-state capture.
    #[must_use]
    pub const fn settle_delay(self) -> WindowLifecycleDuration {
        self.settle_delay
    }

    /// Returns the delay between completed capture and persistence flush.
    #[must_use]
    pub const fn persistence_debounce(self) -> WindowLifecycleDuration {
        self.persistence_debounce
    }

    /// Returns the bound attached to flush directives.
    #[must_use]
    pub const fn flush_timeout(self) -> WindowLifecycleDuration {
        self.flush_timeout
    }
}

/// Independent generation for settled capture and debounce work.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CaptureGeneration(u64);

impl CaptureGeneration {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized generation value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Pure lifecycle input category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowLifecycleEventKind {
    /// Native outer-origin change.
    Moved,
    /// Native inner-size change.
    Resized,
    /// Native scale-factor change.
    ScaleChanged,
    /// Native focus loss.
    Blurred,
    /// Native close request.
    CloseRequested,
    /// Native destruction.
    Destroyed,
    /// Settling timer fired.
    CaptureDeadline,
    /// Host completed the requested live capture.
    CaptureCompleted,
    /// Host could not complete or stage the requested capture.
    CaptureFailed,
    /// Persistence debounce timer fired.
    FlushDeadline,
    /// Caller requested immediate bounded flush.
    FlushRequested,
}

/// One native fact or caller-driven lifecycle completion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowLifecycleEvent {
    /// Outer-frame origin changed.
    Moved {
        /// Stable managed identity.
        window_id: WindowId,
        /// Newly observed outer origin.
        outer_origin: ScreenPoint,
    },
    /// Inner content size changed.
    Resized {
        /// Stable managed identity.
        window_id: WindowId,
        /// Newly observed inner size.
        inner_size: ScreenSize,
    },
    /// Native scale changed.
    ScaleChanged {
        /// Stable managed identity.
        window_id: WindowId,
        /// Newly observed validated scale.
        scale: ScaleFactor,
    },
    /// Window lost focus.
    Blurred {
        /// Stable managed identity.
        window_id: WindowId,
    },
    /// Native close was requested.
    CloseRequested {
        /// Stable managed identity.
        window_id: WindowId,
    },
    /// Native window was destroyed.
    Destroyed {
        /// Stable managed identity.
        window_id: WindowId,
    },
    /// A previously scheduled settle deadline fired.
    CaptureDeadline {
        /// Stable managed identity.
        window_id: WindowId,
        /// Scheduled capture generation.
        generation: CaptureGeneration,
    },
    /// The host captured complete live state.
    CaptureCompleted {
        /// Stable managed identity.
        window_id: WindowId,
        /// Captured generation.
        generation: CaptureGeneration,
    },
    /// Complete live capture or staging failed and may be retried explicitly.
    CaptureFailed {
        /// Stable managed identity.
        window_id: WindowId,
        /// Failed capture generation.
        generation: CaptureGeneration,
    },
    /// A previously scheduled persistence debounce fired.
    FlushDeadline {
        /// Stable managed identity.
        window_id: WindowId,
        /// Scheduled capture generation.
        generation: CaptureGeneration,
    },
    /// Caller requested immediate bounded flush.
    FlushRequested {
        /// Stable managed identity.
        window_id: WindowId,
    },
}

impl WindowLifecycleEvent {
    /// Returns stable logical target identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        match self {
            Self::Moved { window_id, .. }
            | Self::Resized { window_id, .. }
            | Self::ScaleChanged { window_id, .. }
            | Self::Blurred { window_id }
            | Self::CloseRequested { window_id }
            | Self::Destroyed { window_id }
            | Self::CaptureDeadline { window_id, .. }
            | Self::CaptureCompleted { window_id, .. }
            | Self::CaptureFailed { window_id, .. }
            | Self::FlushDeadline { window_id, .. }
            | Self::FlushRequested { window_id } => window_id,
        }
    }

    /// Returns input category.
    #[must_use]
    pub const fn kind(&self) -> WindowLifecycleEventKind {
        match self {
            Self::Moved { .. } => WindowLifecycleEventKind::Moved,
            Self::Resized { .. } => WindowLifecycleEventKind::Resized,
            Self::ScaleChanged { .. } => WindowLifecycleEventKind::ScaleChanged,
            Self::Blurred { .. } => WindowLifecycleEventKind::Blurred,
            Self::CloseRequested { .. } => WindowLifecycleEventKind::CloseRequested,
            Self::Destroyed { .. } => WindowLifecycleEventKind::Destroyed,
            Self::CaptureDeadline { .. } => WindowLifecycleEventKind::CaptureDeadline,
            Self::CaptureCompleted { .. } => WindowLifecycleEventKind::CaptureCompleted,
            Self::CaptureFailed { .. } => WindowLifecycleEventKind::CaptureFailed,
            Self::FlushDeadline { .. } => WindowLifecycleEventKind::FlushDeadline,
            Self::FlushRequested { .. } => WindowLifecycleEventKind::FlushRequested,
        }
    }
}

/// Apply-evidence registration result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ApplyRegistrationOutcome {
    /// A newer apply generation replaced older evidence.
    Registered,
    /// Another operation extended the current generation's evidence.
    Extended,
    /// Older generation evidence was ignored.
    StaleGeneration {
        /// Current registered generation.
        current: ApplyGeneration,
    },
    /// Evidence timestamp preceded already processed input.
    StaleTimestamp {
        /// Latest accepted timestamp.
        latest: MonotonicMillis,
    },
}

/// Why no external work should run for one input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum IgnoreReason {
    /// Event matched expected current-generation apply evidence.
    ProgrammaticApply {
        /// Matching apply generation.
        generation: ApplyGeneration,
        /// Expected operation category.
        operation: WindowOperationKind,
    },
    /// Close consumed explicit programmatic-close evidence.
    ProgrammaticClose {
        /// Matching apply generation.
        generation: ApplyGeneration,
    },
    /// Input timestamp preceded already accepted input for this window.
    StaleTimestamp {
        /// Latest accepted timestamp.
        latest: MonotonicMillis,
    },
    /// Deadline or completion named a superseded capture generation.
    StaleCaptureGeneration {
        /// Current generation when one exists.
        current: Option<CaptureGeneration>,
    },
    /// Timer fired before its declared deadline.
    EarlyDeadline {
        /// Earliest valid timestamp.
        due_at: MonotonicMillis,
    },
    /// No capture work was pending.
    NoPendingCapture,
    /// Capture was already requested for this generation.
    CaptureAlreadyRequested,
    /// Capture completion was already accepted for this generation.
    CaptureAlreadyCompleted,
    /// Failed capture was reset so a later explicit action may retry it.
    CaptureFailedReset,
    /// Completion arrived before capture was requested.
    CaptureNotRequested,
}

/// Why complete live capture should run now.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureReason {
    /// Settling deadline elapsed.
    Settled,
    /// Focus loss requests immediate final-state capture.
    Blur,
    /// Bounded flush requires current state first.
    Flush,
    /// User close requires current state first.
    UserClose,
}

/// Why an injected sink should flush.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlushReason {
    /// Persistence debounce elapsed.
    Debounce,
    /// Caller explicitly requested flush.
    Explicit,
    /// User close requires bounded flush.
    UserClose,
    /// Destruction requires bounded best-effort flush.
    Destroy,
}

/// Pure work emitted for the host adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowLifecycleDirective {
    /// Perform no external work.
    Ignore {
        /// Stable logical identity.
        window_id: WindowId,
        /// Inspectable reason.
        reason: IgnoreReason,
    },
    /// Schedule complete live capture.
    ScheduleCapture {
        /// Stable logical identity.
        window_id: WindowId,
        /// Coalescing generation.
        generation: CaptureGeneration,
        /// Caller-clock deadline.
        due_at: MonotonicMillis,
    },
    /// Capture complete live state now.
    CaptureNow {
        /// Stable logical identity.
        window_id: WindowId,
        /// Current coalescing generation.
        generation: CaptureGeneration,
        /// Trigger.
        reason: CaptureReason,
    },
    /// Schedule persistence flush after captured state is staged.
    ScheduleFlush {
        /// Stable logical identity.
        window_id: WindowId,
        /// Current coalescing generation.
        generation: CaptureGeneration,
        /// Caller-clock deadline.
        due_at: MonotonicMillis,
    },
    /// Request bounded flush from the injected sink.
    Flush {
        /// Stable logical identity.
        window_id: WindowId,
        /// Captured generation when known.
        generation: Option<CaptureGeneration>,
        /// Caller-owned wait bound.
        timeout: WindowLifecycleDuration,
        /// Trigger.
        reason: FlushReason,
    },
    /// Report user close to consumer policy without changing desired state.
    UserClose {
        /// Stable logical identity.
        window_id: WindowId,
    },
    /// Remove all coordinator state for a destroyed window.
    Forget {
        /// Stable logical identity.
        window_id: WindowId,
    },
}
