//! Lifecycle clocks, schedulers, and injected service ports.

mod bundle;
mod ports;
mod scheduler;

#[cfg(test)]
mod tests;

pub use bundle::TauriWindowLifecycleServices;
pub use ports::{
    NoopWindowLifecycleReporter, ProgrammaticApplyObserver, TauriWindowRevealBackend,
    WindowCaptureBackend, WindowLifecycleReporter, WindowRevealBackend,
};
pub use scheduler::{
    TauriAsyncWindowLifecycleScheduler, WindowLifecycleScheduler, WindowLifecycleWakeHandler,
};
