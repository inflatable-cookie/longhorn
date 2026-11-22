use longhorn_core::WindowId;
use longhorn_windowing::{
    CapturedWindowPlacement, ScheduledWindowLifecycleWake, WindowPlacementSink,
};

use crate::GpuiWindowKey;

/// Deadline seam for capture settling and persistence debounce.
///
/// GPUI's foreground executor schedules onto the main thread, so a wake
/// arrives back on the same thread that produced it. The Tauri scheduler binds
/// a `Weak<dyn WindowLifecycleWakeHandler>` because its wakes cross threads;
/// here the caller simply hands the wake back.
pub trait GpuiLifecycleScheduler {
    /// Accepts one deadline. Returning `Err` reports a scheduling failure.
    fn schedule(&mut self, wake: ScheduledWindowLifecycleWake) -> Result<(), String>;
}

/// Complete live capture seam.
pub trait GpuiWindowCaptureBackend {
    /// Captures one window without persistence or product policy.
    ///
    /// There is no `retained_normal` parameter. GPUI reports a maximized
    /// window's restore bounds itself, so the caller has nothing to thread
    /// back in.
    fn capture(
        &mut self,
        window_id: &WindowId,
        key: GpuiWindowKey,
    ) -> Result<CapturedWindowPlacement, String>;
}

/// Injected seams for one GPUI lifecycle host.
///
/// The scheduler and capture backend run on the platform main thread and may
/// borrow the GPUI application context.
pub struct GpuiWindowLifecycleServices<C, S, B, U> {
    /// Monotonic clock.
    pub clock: C,
    /// Deadline scheduler.
    pub scheduler: S,
    /// Live capture.
    pub capture: B,
    /// User-close policy.
    pub user_close: U,
    /// Placement staging and bounded flush.
    ///
    /// This one is `Send + Sync` because it is Longhorn's own port, shared
    /// with the Tauri host unchanged, and its implementation writes to storage
    /// off the main thread.
    pub sink: Box<dyn WindowPlacementSink>,
}
