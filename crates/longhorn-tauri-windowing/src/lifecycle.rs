//! Tauri event capture, persistence, reveal, and bounded flush.

mod capture;
mod host;
mod model;
mod services;
mod translation;

pub use capture::{
    TauriWindowCaptureBackend, TauriWindowCaptureError, UniformWindowGeometryMapper,
    WindowGeometryMapper, WindowScaleGeometryMapper,
};
pub use host::TauriWindowLifecycleHost;
pub use model::{
    TauriWindowLifecycleAction, TauriWindowLifecycleError, TauriWindowLifecycleReceipt,
    WindowLifecycleReport, WindowRevealReceipt, WindowRevealStatus, WindowShutdownReceipt,
};
pub use services::{
    NoopWindowLifecycleReporter, ProgrammaticApplyObserver, TauriAsyncWindowLifecycleScheduler,
    TauriWindowLifecycleServices, TauriWindowRevealBackend, WindowCaptureBackend,
    WindowLifecycleReporter, WindowLifecycleScheduler, WindowLifecycleWakeHandler,
    WindowRevealBackend,
};
pub use translation::translate_tauri_window_event;
