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

/// One queued wake with supersession identity.
struct QueuedWake {
    sequence: u64,
    wake: ScheduledWindowLifecycleWake,
}

#[derive(Default)]
struct SchedulerState {
    queue: std::collections::BinaryHeap<std::cmp::Reverse<(u64, u64)>>,
    wakes: std::collections::BTreeMap<u64, QueuedWake>,
    latest:
        std::collections::BTreeMap<(WindowId, longhorn_windowing::WindowLifecycleEventKind), u64>,
    next_sequence: u64,
    worker_running: bool,
}

struct SchedulerInner {
    clock: Arc<dyn WindowLifecycleClock>,
    handler: OnceLock<Weak<dyn WindowLifecycleWakeHandler>>,
    state: std::sync::Mutex<SchedulerState>,
    wakeup: std::sync::Condvar,
}

/// Scheduler backed by one shared timer thread. A newer wake for the same
/// window and event kind supersedes an undelivered older one, so debounce
/// storms deliver only their latest deadline and occupy no thread per wake.
pub struct TauriAsyncWindowLifecycleScheduler {
    inner: Arc<SchedulerInner>,
}

impl TauriAsyncWindowLifecycleScheduler {
    /// Constructs an unbound scheduler over the host's monotonic clock.
    #[must_use]
    pub fn new(clock: Arc<dyn WindowLifecycleClock>) -> Self {
        Self {
            inner: Arc::new(SchedulerInner {
                clock,
                handler: OnceLock::new(),
                state: std::sync::Mutex::new(SchedulerState::default()),
                wakeup: std::sync::Condvar::new(),
            }),
        }
    }

    fn run_worker(inner: &Arc<SchedulerInner>) {
        const IDLE_LIVENESS_CHECK: Duration = Duration::from_secs(60);
        let exit = |inner: &SchedulerInner| {
            if let Ok(mut state) = inner.state.lock() {
                state.worker_running = false;
            }
        };
        loop {
            let due_wake = {
                let Ok(mut state) = inner.state.lock() else {
                    return exit(inner);
                };
                loop {
                    let now = inner.clock.now().get();
                    match state.queue.peek() {
                        Some(std::cmp::Reverse((due, _))) if *due <= now => {
                            let std::cmp::Reverse((_, sequence)) =
                                state.queue.pop().unwrap_or(std::cmp::Reverse((0, 0)));
                            let Some(queued) = state.wakes.remove(&sequence) else {
                                continue;
                            };
                            let key = (
                                queued.wake.event().window_id().clone(),
                                queued.wake.event().kind(),
                            );
                            if state.latest.get(&key) != Some(&queued.sequence) {
                                continue;
                            }
                            state.latest.remove(&key);
                            break Some(queued.wake);
                        }
                        Some(std::cmp::Reverse((due, _))) => {
                            let wait = Duration::from_millis(due.saturating_sub(now));
                            let Ok((next, _)) = inner.wakeup.wait_timeout(state, wait) else {
                                return exit(inner);
                            };
                            state = next;
                        }
                        None => {
                            let Ok((next, _)) =
                                inner.wakeup.wait_timeout(state, IDLE_LIVENESS_CHECK)
                            else {
                                return exit(inner);
                            };
                            state = next;
                            if state.queue.is_empty()
                                && inner
                                    .handler
                                    .get()
                                    .is_none_or(|handler| handler.upgrade().is_none())
                            {
                                state.worker_running = false;
                                return;
                            }
                        }
                    }
                }
            };
            if let Some(wake) = due_wake {
                let Some(handler) = inner.handler.get().and_then(Weak::upgrade) else {
                    return exit(inner);
                };
                // Delivery failures are reported by the handler itself
                // through the lifecycle reporter seam.
                let _ = handler.handle_scheduled_wake(wake);
            }
        }
    }
}

impl WindowLifecycleScheduler for TauriAsyncWindowLifecycleScheduler {
    fn bind(&self, handler: Weak<dyn WindowLifecycleWakeHandler>) -> Result<(), String> {
        self.inner
            .handler
            .set(handler)
            .map_err(|_| "Tauri lifecycle scheduler is already bound".to_string())
    }

    fn schedule(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        if self.inner.handler.get().is_none() {
            return Err("Tauri lifecycle scheduler is not bound".to_string());
        }
        let spawn_worker = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| "lifecycle scheduler state is poisoned".to_string())?;
            let sequence = state.next_sequence;
            state.next_sequence += 1;
            let key = (wake.event().window_id().clone(), wake.event().kind());
            state.latest.insert(key, sequence);
            state
                .queue
                .push(std::cmp::Reverse((wake.due_at().get(), sequence)));
            state.wakes.insert(sequence, QueuedWake { sequence, wake });
            let spawn_worker = !state.worker_running;
            state.worker_running = true;
            spawn_worker
        };
        self.inner.wakeup.notify_one();
        if spawn_worker {
            let inner = Arc::clone(&self.inner);
            if let Err(error) = std::thread::Builder::new()
                .name("longhorn-window-lifecycle-timer".to_string())
                .spawn(move || Self::run_worker(&inner))
            {
                if let Ok(mut state) = self.inner.state.lock() {
                    state.worker_running = false;
                }
                return Err(format!("lifecycle timer thread failed to start: {error}"));
            }
        }
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

    struct RecordingHandler(Sender<ScheduledWindowLifecycleWake>);

    impl WindowLifecycleWakeHandler for RecordingHandler {
        fn handle_scheduled_wake(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
            self.0.send(wake).map_err(|error| error.to_string())
        }
    }

    struct AdjustableClock(std::sync::atomic::AtomicU64);

    impl WindowLifecycleClock for AdjustableClock {
        fn now(&self) -> MonotonicMillis {
            MonotonicMillis::new(self.0.load(std::sync::atomic::Ordering::SeqCst))
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
            receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .event()
                .window_id(),
            &WindowId::new("window:scheduled").unwrap()
        );
    }

    #[test]
    fn newer_wake_for_the_same_window_and_kind_supersedes_the_older_one() {
        let clock = Arc::new(AdjustableClock(std::sync::atomic::AtomicU64::new(0)));
        let scheduler = TauriAsyncWindowLifecycleScheduler::new(clock.clone());
        let (sender, receiver) = channel();
        let handler: Arc<dyn WindowLifecycleWakeHandler> = Arc::new(RecordingHandler(sender));
        scheduler.bind(Arc::downgrade(&handler)).unwrap();

        let wake_at = |due: u64| {
            ScheduledWindowLifecycleWake::new(
                MonotonicMillis::new(due),
                WindowLifecycleEvent::Blurred {
                    window_id: WindowId::new("window:superseded").unwrap(),
                },
            )
        };
        scheduler.schedule(wake_at(50)).unwrap();
        scheduler.schedule(wake_at(60)).unwrap();
        clock.0.store(100, std::sync::atomic::Ordering::SeqCst);

        let delivered = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(delivered.due_at(), MonotonicMillis::new(60));
        assert!(
            receiver.recv_timeout(Duration::from_millis(300)).is_err(),
            "superseded wake was still delivered"
        );
    }
}
