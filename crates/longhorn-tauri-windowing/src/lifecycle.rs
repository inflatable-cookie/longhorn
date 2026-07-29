//! Tauri event capture, persistence, reveal, and bounded flush.

mod capture;
mod host;
mod model;
mod services;
mod translation;

pub use capture::{
    TauriWindowCaptureBackend, UniformWindowGeometryMapper, WindowGeometryMapper,
    WindowScaleGeometryMapper,
};
pub use host::TauriWindowLifecycleHost;
pub use model::{
    CapturedDisplayAssociation, CapturedDisplayEvidence, CapturedWindowPlacement,
    ScheduledWindowLifecycleWake, TauriWindowLifecycleAction, TauriWindowLifecycleError,
    TauriWindowLifecycleReceipt, WindowFlushOutcome, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget, WindowLifecycleReport, WindowRevealReceipt, WindowRevealStatus,
    WindowShutdownReceipt,
};
pub use services::{
    NoopWindowLifecycleReporter, NoopWindowUserCloseHandler, ProcessMonotonicClock,
    ProgrammaticApplyObserver, TauriAsyncWindowLifecycleScheduler, TauriWindowLifecycleServices,
    TauriWindowRevealBackend, WindowCaptureBackend, WindowLifecycleClock, WindowLifecycleReporter,
    WindowLifecycleScheduler, WindowLifecycleWakeHandler, WindowPlacementFlushCompletion,
    WindowPlacementFlushTicket, WindowPlacementSink, WindowRevealBackend, WindowUserCloseHandler,
};
pub use translation::translate_tauri_window_event;
