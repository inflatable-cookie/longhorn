//! Lifecycle clocks, schedulers, and injected service ports.

pub use longhorn_windowing::{
    WindowPlacementFlushCompletion, WindowPlacementFlushTicket, WindowPlacementSink,
};
mod bundle;
mod clock;
mod ports;
mod scheduler;

#[cfg(test)]
mod tests;

pub use bundle::TauriWindowLifecycleServices;
pub use clock::{ProcessMonotonicClock, WindowLifecycleClock};
pub use ports::{
    NoopWindowLifecycleReporter, NoopWindowUserCloseHandler, ProgrammaticApplyObserver,
    TauriWindowRevealBackend, WindowCaptureBackend, WindowLifecycleReporter, WindowRevealBackend,
    WindowUserCloseHandler,
};
pub use scheduler::{
    TauriAsyncWindowLifecycleScheduler, WindowLifecycleScheduler, WindowLifecycleWakeHandler,
};
