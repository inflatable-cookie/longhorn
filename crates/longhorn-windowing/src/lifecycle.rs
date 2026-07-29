//! Pure native-window event attribution and settling.

mod coordinator;
mod model;
mod persistence;
mod state;
mod time;
mod transitions;

use crate::{ApplyGeneration, WindowOperation};
use state::{ApplyExpectation, ExpectedEffect, GeometryEvent, PendingCapture, WindowState};
use transitions::{handle_event, ignore};

pub use coordinator::WindowLifecycleCoordinator;
pub use model::{
    ApplyRegistrationOutcome, CaptureGeneration, CaptureReason, FlushReason, IgnoreReason,
    WindowLifecycleDirective, WindowLifecycleEvent, WindowLifecycleEventKind,
    WindowLifecyclePolicy,
};
pub use time::{MonotonicMillis, WindowLifecycleDuration, WindowLifecycleError};
