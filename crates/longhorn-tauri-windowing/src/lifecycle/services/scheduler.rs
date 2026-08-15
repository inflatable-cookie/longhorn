use std::{
    sync::{Arc, OnceLock, Weak},
    time::Duration,
};

use longhorn_core::WindowId;

use super::super::ScheduledWindowLifecycleWake;
use super::WindowLifecycleClock;

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
pub(crate) struct QueuedWake {
    pub(crate) sequence: u64,
    pub(crate) wake: ScheduledWindowLifecycleWake,
}

#[derive(Default)]
pub(crate) struct SchedulerState {
    pub(crate) queue: std::collections::BinaryHeap<std::cmp::Reverse<(u64, u64)>>,
    pub(crate) wakes: std::collections::BTreeMap<u64, QueuedWake>,
    pub(crate) latest:
        std::collections::BTreeMap<(WindowId, longhorn_windowing::WindowLifecycleEventKind), u64>,
    pub(crate) next_sequence: u64,
    pub(crate) worker_running: bool,
}

pub(crate) struct SchedulerInner {
    pub(crate) clock: Arc<dyn WindowLifecycleClock>,
    pub(crate) handler: OnceLock<Weak<dyn WindowLifecycleWakeHandler>>,
    pub(crate) state: std::sync::Mutex<SchedulerState>,
    pub(crate) wakeup: std::sync::Condvar,
}

/// Scheduler backed by one shared timer thread. A newer wake for the same
/// window and event kind supersedes an undelivered older one, so debounce
/// storms deliver only their latest deadline and occupy no thread per wake.
pub struct TauriAsyncWindowLifecycleScheduler {
    pub(crate) inner: Arc<SchedulerInner>,
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
                    // The handler died after this wake was queued; there is no
                    // reporter left to tell. The loud half of this is in
                    // `schedule`, which refuses new wakes from here on.
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
        // A wake queued behind a dead handler is a wake that vanishes: the
        // worker pops it, fails to upgrade, and exits without a word. Refuse
        // instead, so the caller hears about the dead handler at the one
        // moment it can still act.
        match self.inner.handler.get() {
            None => return Err("Tauri lifecycle scheduler is not bound".to_string()),
            Some(handler) if handler.upgrade().is_none() => {
                return Err(
                    "Tauri lifecycle wake handler is gone; refusing to queue an \
                     undeliverable wake"
                        .to_string(),
                );
            }
            _ => {}
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
