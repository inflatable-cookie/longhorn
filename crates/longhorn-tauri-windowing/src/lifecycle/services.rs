use std::{
    sync::{
        Arc, OnceLock, Weak,
        mpsc::{Receiver, channel},
    },
    time::{Duration, Instant},
};

use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::{ApplyGeneration, MonotonicMillis, WindowOperation};
use tauri::{Runtime, WebviewWindow};

use super::{
    CapturedWindowPlacement, ScheduledWindowLifecycleWake, WindowFlushRequest,
    WindowGeometryMapper, WindowLifecycleReport,
};

/// Process-local monotonic time source.
pub trait WindowLifecycleClock: Send + Sync {
    /// Returns milliseconds from the clock's arbitrary epoch.
    fn now(&self) -> MonotonicMillis;
}

/// `Instant`-backed process clock.
pub struct ProcessMonotonicClock {
    epoch: Instant,
}

impl ProcessMonotonicClock {
    /// Starts a new arbitrary process-local epoch.
    #[must_use]
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Default for ProcessMonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowLifecycleClock for ProcessMonotonicClock {
    fn now(&self) -> MonotonicMillis {
        let elapsed = self.epoch.elapsed().as_millis();
        MonotonicMillis::new(u64::try_from(elapsed).unwrap_or(u64::MAX))
    }
}

/// Host-owned timer or event-loop scheduling seam.
pub trait WindowLifecycleScheduler: Send + Sync {
    /// Binds the shared host wake target when this scheduler needs one.
    fn bind(&self, _handler: Weak<dyn WindowLifecycleWakeHandler>) -> Result<(), String> {
        Ok(())
    }

    /// Accepts one exact wake for later delivery.
    fn schedule(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String>;
}

impl<F> WindowLifecycleScheduler for F
where
    F: Fn(ScheduledWindowLifecycleWake) -> Result<(), String> + Send + Sync,
{
    fn schedule(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        self(wake)
    }
}

/// Scheduler-independent delivery target for one lifecycle wake.
pub trait WindowLifecycleWakeHandler: Send + Sync {
    /// Delivers one previously scheduled wake.
    fn handle_scheduled_wake(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String>;
}

/// Scheduler backed by Tauri's blocking async-runtime pool.
pub struct TauriAsyncWindowLifecycleScheduler {
    clock: Arc<dyn WindowLifecycleClock>,
    handler: OnceLock<Weak<dyn WindowLifecycleWakeHandler>>,
}

impl TauriAsyncWindowLifecycleScheduler {
    /// Constructs an unbound scheduler over the host's monotonic clock.
    #[must_use]
    pub fn new(clock: Arc<dyn WindowLifecycleClock>) -> Self {
        Self {
            clock,
            handler: OnceLock::new(),
        }
    }
}

impl WindowLifecycleScheduler for TauriAsyncWindowLifecycleScheduler {
    fn bind(&self, handler: Weak<dyn WindowLifecycleWakeHandler>) -> Result<(), String> {
        self.handler
            .set(handler)
            .map_err(|_| "Tauri lifecycle scheduler is already bound".to_string())
    }

    fn schedule(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        let handler = self
            .handler
            .get()
            .cloned()
            .ok_or_else(|| "Tauri lifecycle scheduler is not bound".to_string())?;
        let delay = wake.due_at().get().saturating_sub(self.clock.now().get());
        tauri::async_runtime::spawn_blocking(move || {
            if delay > 0 {
                std::thread::sleep(Duration::from_millis(delay));
            }
            if let Some(handler) = handler.upgrade() {
                let _ = handler.handle_scheduled_wake(wake);
            }
        });
        Ok(())
    }
}

/// Complete live capture seam.
pub trait WindowCaptureBackend<R: Runtime>: Send + Sync {
    /// Captures one window without persistence or product policy.
    fn capture(
        &self,
        window_id: &WindowId,
        window: &WebviewWindow<R>,
        retained_normal: Option<WindowPlacement>,
    ) -> Result<CapturedWindowPlacement, String>;
}

/// Sink-owned stage and bounded flush authority.
pub trait WindowPlacementSink: Send + Sync {
    /// Accepts one schema-opaque placement proposal.
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String>;

    /// Starts one flush and returns its acknowledgement channel.
    fn request_flush(
        &self,
        request: &WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String>;
}

/// Flush completion sent by an injected sink.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowPlacementFlushCompletion {
    /// Sink completed successfully.
    Succeeded,
    /// Sink completed with an inspectable failure.
    Failed(String),
}

/// One receiver used by the adapter to enforce its wait bound.
pub struct WindowPlacementFlushTicket {
    receiver: Receiver<WindowPlacementFlushCompletion>,
}

impl WindowPlacementFlushTicket {
    /// Wraps a sink completion receiver.
    #[must_use]
    pub const fn new(receiver: Receiver<WindowPlacementFlushCompletion>) -> Self {
        Self { receiver }
    }

    /// Constructs an already successful synchronous completion.
    #[must_use]
    pub fn completed() -> Self {
        Self::from_completion(WindowPlacementFlushCompletion::Succeeded)
    }

    /// Constructs an already failed synchronous completion.
    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self::from_completion(WindowPlacementFlushCompletion::Failed(reason.into()))
    }

    fn from_completion(completion: WindowPlacementFlushCompletion) -> Self {
        let (sender, receiver) = channel();
        sender
            .send(completion)
            .expect("new completion receiver remains connected");
        Self::new(receiver)
    }

    pub(crate) fn wait(
        self,
        timeout_millis: u64,
    ) -> Result<WindowPlacementFlushCompletion, std::sync::mpsc::RecvTimeoutError> {
        self.receiver
            .recv_timeout(Duration::from_millis(timeout_millis))
    }
}

/// Consumer-owned user-close policy callback.
pub trait WindowUserCloseHandler: Send + Sync {
    /// Receives user close without inferred desired-state mutation.
    fn user_close(&self, window_id: &WindowId) -> Result<(), String>;
}

/// Explicit no-op user-close policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopWindowUserCloseHandler;

impl WindowUserCloseHandler for NoopWindowUserCloseHandler {
    fn user_close(&self, _window_id: &WindowId) -> Result<(), String> {
        Ok(())
    }
}

impl<F> WindowUserCloseHandler for F
where
    F: Fn(&WindowId) -> Result<(), String> + Send + Sync,
{
    fn user_close(&self, window_id: &WindowId) -> Result<(), String> {
        self(window_id)
    }
}

/// Async listener receipt observer.
pub trait WindowLifecycleReporter: Send + Sync {
    /// Records one complete event result.
    fn report(&self, report: WindowLifecycleReport);
}

/// Explicit no-op asynchronous lifecycle reporter.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopWindowLifecycleReporter;

impl WindowLifecycleReporter for NoopWindowLifecycleReporter {
    fn report(&self, _report: WindowLifecycleReport) {}
}

impl<F> WindowLifecycleReporter for F
where
    F: Fn(WindowLifecycleReport) + Send + Sync,
{
    fn report(&self, report: WindowLifecycleReport) {
        self(report);
    }
}

/// Native reveal seam.
pub trait WindowRevealBackend<R: Runtime>: Send + Sync {
    /// Shows one placement-ready, page-ready window.
    fn reveal(&self, window: &WebviewWindow<R>) -> Result<(), String>;
}

/// Direct Tauri show backend.
#[derive(Clone, Copy, Debug, Default)]
pub struct TauriWindowRevealBackend;

impl<R: Runtime> WindowRevealBackend<R> for TauriWindowRevealBackend {
    fn reveal(&self, window: &WebviewWindow<R>) -> Result<(), String> {
        window.show().map_err(|error| error.to_string())
    }
}

/// Observer invoked by Card 018 immediately before a native apply operation.
pub trait ProgrammaticApplyObserver: Send + Sync {
    /// Installs exact generation and operation evidence.
    fn register_apply(
        &self,
        generation: ApplyGeneration,
        operation: &WindowOperation,
    ) -> Result<(), String>;
}

/// Complete injected runtime boundary for the lifecycle host.
pub struct TauriWindowLifecycleServices<R: Runtime> {
    pub(crate) clock: Arc<dyn WindowLifecycleClock>,
    pub(crate) scheduler: Arc<dyn WindowLifecycleScheduler>,
    pub(crate) mapper: Arc<dyn WindowGeometryMapper>,
    pub(crate) capture: Arc<dyn WindowCaptureBackend<R>>,
    pub(crate) sink: Arc<dyn WindowPlacementSink>,
    pub(crate) user_close: Arc<dyn WindowUserCloseHandler>,
    pub(crate) reporter: Arc<dyn WindowLifecycleReporter>,
    pub(crate) reveal: Arc<dyn WindowRevealBackend<R>>,
}

impl<R: Runtime> TauriWindowLifecycleServices<R> {
    /// Collects the host's explicit clock, scheduling, I/O, and callback seams.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        clock: Arc<dyn WindowLifecycleClock>,
        scheduler: Arc<dyn WindowLifecycleScheduler>,
        mapper: Arc<dyn WindowGeometryMapper>,
        capture: Arc<dyn WindowCaptureBackend<R>>,
        sink: Arc<dyn WindowPlacementSink>,
        user_close: Arc<dyn WindowUserCloseHandler>,
        reporter: Arc<dyn WindowLifecycleReporter>,
        reveal: Arc<dyn WindowRevealBackend<R>>,
    ) -> Self {
        Self {
            clock,
            scheduler,
            mapper,
            capture,
            sink,
            user_close,
            reporter,
            reveal,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            mpsc::{Sender, channel},
        },
        time::Duration,
    };

    use longhorn_core::WindowId;
    use longhorn_windowing::{MonotonicMillis, WindowLifecycleEvent};

    use super::{
        TauriAsyncWindowLifecycleScheduler, WindowLifecycleClock, WindowLifecycleScheduler,
        WindowLifecycleWakeHandler, WindowPlacementFlushCompletion, WindowPlacementFlushTicket,
    };
    use crate::ScheduledWindowLifecycleWake;

    struct FixedClock;

    impl WindowLifecycleClock for FixedClock {
        fn now(&self) -> MonotonicMillis {
            MonotonicMillis::new(10)
        }
    }

    struct RecordingHandler(Sender<WindowId>);

    impl WindowLifecycleWakeHandler for RecordingHandler {
        fn handle_scheduled_wake(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
            self.0
                .send(wake.event().window_id().clone())
                .map_err(|error| error.to_string())
        }
    }

    #[test]
    fn synchronous_ticket_constructors_deliver_exact_completion() {
        assert_eq!(
            WindowPlacementFlushTicket::completed().wait(0).unwrap(),
            WindowPlacementFlushCompletion::Succeeded
        );
        assert_eq!(
            WindowPlacementFlushTicket::failed("disk full")
                .wait(0)
                .unwrap(),
            WindowPlacementFlushCompletion::Failed("disk full".to_string())
        );
    }

    #[test]
    fn tauri_scheduler_binds_then_delivers_on_the_runtime() {
        let scheduler = TauriAsyncWindowLifecycleScheduler::new(Arc::new(FixedClock));
        let wake = ScheduledWindowLifecycleWake::new(
            MonotonicMillis::new(10),
            WindowLifecycleEvent::Blurred {
                window_id: WindowId::new("window:scheduled").unwrap(),
            },
        );
        assert!(scheduler.schedule(wake.clone()).is_err());

        let (sender, receiver) = channel();
        let handler: Arc<dyn WindowLifecycleWakeHandler> = Arc::new(RecordingHandler(sender));
        scheduler.bind(Arc::downgrade(&handler)).unwrap();
        scheduler.schedule(wake).unwrap();

        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
            WindowId::new("window:scheduled").unwrap()
        );
    }
}
