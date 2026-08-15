//! Service scheduler and ticket tests.

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
fn a_schedule_after_the_handler_is_gone_fails_loudly() {
    let scheduler = TauriAsyncWindowLifecycleScheduler::new(Arc::new(FixedClock));
    let wake = ScheduledWindowLifecycleWake::new(
        MonotonicMillis::new(10),
        WindowLifecycleEvent::Blurred {
            window_id: WindowId::new("window:orphaned").unwrap(),
        },
    );

    let (sender, _receiver) = channel();
    let handler: Arc<dyn WindowLifecycleWakeHandler> = Arc::new(RecordingHandler(sender));
    scheduler.bind(Arc::downgrade(&handler)).unwrap();
    drop(handler);

    let error = scheduler.schedule(wake).unwrap_err();
    assert!(
        error.contains("wake handler is gone"),
        "expected a loud refusal, got: {error}"
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
