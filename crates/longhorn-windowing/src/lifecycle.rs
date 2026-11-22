//! Pure native-window event attribution and settling.

mod capture;
mod coordinator;
mod flush;
mod model;
mod persistence;
mod placement_sink;
mod ports;
mod state;
mod time;
mod transitions;

use crate::{ApplyGeneration, WindowOperation};
use state::{ApplyExpectation, ExpectedEffect, GeometryEvent, PendingCapture, WindowState};
use transitions::{handle_event, ignore};

pub use capture::{CapturedDisplayAssociation, CapturedDisplayEvidence, CapturedWindowPlacement};
pub use coordinator::WindowLifecycleCoordinator;
pub use flush::{
    ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget,
};
pub use model::{
    ApplyRegistrationOutcome, CaptureGeneration, CaptureReason, FlushReason, IgnoreReason,
    WindowLifecycleDirective, WindowLifecycleEvent, WindowLifecycleEventKind,
    WindowLifecyclePolicy,
};
pub use placement_sink::{
    WindowPlacementFlushCompletion, WindowPlacementFlushTicket, WindowPlacementSink,
    flush_window_placements,
};
pub use time::{MonotonicMillis, WindowLifecycleDuration, WindowLifecycleError};

pub use ports::{
    NoopWindowUserCloseHandler, ProcessMonotonicClock, WindowLifecycleClock, WindowUserCloseHandler,
};
