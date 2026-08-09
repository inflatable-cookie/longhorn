//! Contract 020: lifecycle teardown under load.
//!
//! The contract records this as unproven on **either** backend, and names it
//! one of the three places a single-host contract is most likely to have
//! leaked. It is the one of the three that needs no hardware: "under load"
//! here means many windows, work still in flight, and close requests arriving
//! while it is, which a pure host can be driven through exactly.
//!
//! What this does not prove is a real GPUI window tearing down while a real
//! flush is in flight. That still wants a target application, and the ceiling
//! is stated rather than papered over.
//!
//! The invariants below are the ones a teardown can violate silently:
//!
//! 1. no window closes while anything is unresolved
//! 2. nothing closes having lost state without saying so
//! 3. the restart interlock never reads quiet while work is outstanding
//! 4. no window survives its own teardown in the host's registry

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

use longhorn_core::WindowId;
use longhorn_gpui_windowing::{
    GpuiCloseDecision, GpuiLifecycleAction, GpuiLifecycleClock, GpuiLifecycleScheduler,
    GpuiWindowCaptureBackend, GpuiWindowEvent, GpuiWindowKey, GpuiWindowLifecycleHost,
    GpuiWindowLifecycleServices, GpuiWindowQuiescenceProbe, NoopGpuiUserCloseHandler,
    close_is_safe,
};
use longhorn_update::QuiescenceProbe;
use longhorn_windowing::{
    CapturedDisplayAssociation, CapturedWindowPlacement, MonotonicMillis,
    ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowLifecycleEvent, WindowLifecyclePolicy,
    WindowPlacementFlushTicket, WindowPlacementSink,
};

use super::support::{id, placement};

/// Thirteen is enough to interleave and small enough to read in a failure.
const WINDOWS: usize = 13;

struct SteppingClock(AtomicU64);

impl GpuiLifecycleClock for SteppingClock {
    fn now(&self) -> MonotonicMillis {
        // Advances on every read, so no two events in a run share a reading.
        // A teardown bug that only shows when two things happen in the same
        // millisecond would hide behind a fixed clock.
        MonotonicMillis::new(self.0.fetch_add(7, Ordering::Relaxed))
    }
}

#[derive(Default)]
struct CountingScheduler {
    accepted: Vec<ScheduledWindowLifecycleWake>,
}

impl GpuiLifecycleScheduler for CountingScheduler {
    fn schedule(&mut self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        self.accepted.push(wake);
        Ok(())
    }
}

struct MovingCapture;

impl GpuiWindowCaptureBackend for MovingCapture {
    fn capture(
        &mut self,
        window_id: &WindowId,
        _key: GpuiWindowKey,
    ) -> Result<CapturedWindowPlacement, String> {
        Ok(CapturedWindowPlacement::new(
            window_id.clone(),
            placement(20, 30, 900, 700),
            false,
            CapturedDisplayAssociation::Unresolved,
        ))
    }
}

/// A sink that can be made to fail its flush, standing in for a store that is
/// busy or gone at exactly the wrong moment.
#[derive(Default)]
struct LoadedSink {
    staged: Mutex<Vec<WindowId>>,
    flushes: AtomicUsize,
    failing: AtomicBool,
}

impl LoadedSink {
    fn fail_flushes(&self, failing: bool) {
        self.failing.store(failing, Ordering::Relaxed);
    }
}

/// Lets the test keep a handle on the sink the host owns.
///
/// The obvious alternative is a raw pointer back into the box, and the
/// workspace forbids `unsafe`. Sharing an `Arc` says the same thing without
/// asking anyone to trust a lifetime argument in a comment.
struct SharedSink(Arc<LoadedSink>);

impl WindowPlacementSink for SharedSink {
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String> {
        self.0.stage(placement)
    }

    fn request_flush(
        &self,
        request: &longhorn_windowing::WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        self.0.request_flush(request)
    }
}

impl WindowPlacementSink for LoadedSink {
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String> {
        self.staged
            .lock()
            .unwrap()
            .push(placement.window_id().clone());
        Ok(())
    }

    fn request_flush(
        &self,
        _request: &longhorn_windowing::WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        if self.failing.load(Ordering::Relaxed) {
            Ok(WindowPlacementFlushTicket::failed("store is busy"))
        } else {
            Ok(WindowPlacementFlushTicket::completed())
        }
    }
}

type Host = GpuiWindowLifecycleHost<
    SteppingClock,
    CountingScheduler,
    MovingCapture,
    NoopGpuiUserCloseHandler,
>;

fn loaded_host(sink: &Arc<LoadedSink>) -> Host {
    GpuiWindowLifecycleHost::new(
        WindowLifecyclePolicy::recommended(),
        GpuiWindowLifecycleServices {
            clock: SteppingClock(AtomicU64::new(1_000)),
            scheduler: CountingScheduler::default(),
            capture: MovingCapture,
            user_close: NoopGpuiUserCloseHandler,
            sink: Box::new(SharedSink(Arc::clone(sink))),
        },
    )
}

fn window(index: usize) -> WindowId {
    id(&format!("window-{index}"))
}

/// Installs every window and gives each one something to lose: a move, so a
/// capture is pending, and a resize, so the coordinator has staged state a
/// close has to settle.
fn install_all(host: &mut Host) {
    for index in 0..WINDOWS {
        host.install(window(index), GpuiWindowKey::new(index as u64 + 1));
        host.record_scale(&window(index), super::support::scale(2000));

        host.handle_lifecycle_event(WindowLifecycleEvent::Moved {
            window_id: window(index),
            outer_origin: longhorn_core::ScreenPoint::new(index as i32 * 10, 40),
        })
        .expect("move is accepted");

        host.handle_gpui_event(
            &window(index),
            GpuiWindowEvent::Resized {
                content_size: longhorn_gpui_windowing::GpuiLogicalSize::new(
                    900.0 + index as f32,
                    700.0,
                ),
                scale: 2.0,
            },
        )
        .expect("resize is accepted");
    }
}

#[test]
fn every_window_tears_down_with_nothing_left_installed() {
    let mut host = loaded_host(&Arc::new(LoadedSink::default()));
    install_all(&mut host);

    for index in 0..WINDOWS {
        assert!(host.is_installed(&window(index)));
    }

    // Close them out of order. A teardown that only works front to back is a
    // teardown that depends on the order the platform happens to use.
    let order: Vec<usize> = (0..WINDOWS)
        .rev()
        .step_by(2)
        .chain((0..WINDOWS).rev())
        .collect();
    for index in order {
        if !host.is_installed(&window(index)) {
            continue;
        }
        let (decision, receipt) = host
            .handle_close_requested(&window(index))
            .expect("close request is accepted");

        assert_eq!(
            decision,
            GpuiCloseDecision::Close,
            "{:?}",
            receipt.actions()
        );
        host.handle_lifecycle_event(WindowLifecycleEvent::Destroyed {
            window_id: window(index),
        })
        .expect("destroy is accepted");
    }

    for index in 0..WINDOWS {
        assert!(
            !host.is_installed(&window(index)),
            "window-{index} survived its own teardown"
        );
    }
}

#[test]
fn nothing_closes_while_a_flush_is_failing() {
    // The invariant that matters: a window whose placement could not be
    // written must not close, because closing is what makes the loss
    // permanent.
    //
    // These windows are installed and left alone, so the close goes straight
    // to the flush. A window with a capture still pending takes a different
    // path — see `a_close_with_a_pending_capture_stages_without_flushing`,
    // which records what that path actually does.
    let sink = Arc::new(LoadedSink::default());
    let mut host = loaded_host(&sink);
    for index in 0..WINDOWS {
        host.install(window(index), GpuiWindowKey::new(index as u64 + 1));
    }

    sink.fail_flushes(true);

    for index in 0..WINDOWS {
        let (decision, receipt) = host
            .handle_close_requested(&window(index))
            .expect("close request is accepted");

        assert_eq!(
            decision,
            GpuiCloseDecision::Defer,
            "window-{index} closed with a failed flush: {:?}",
            receipt.actions()
        );
        assert!(!close_is_safe(&receipt));
        assert!(
            receipt.actions().iter().any(|action| matches!(
                action,
                GpuiLifecycleAction::Flushed {
                    outcome: WindowFlushOutcome::SinkFailed { .. },
                    ..
                }
            )),
            "the refusal did not name the flush: {:?}",
            receipt.actions()
        );
        assert!(host.is_installed(&window(index)));
    }

    // The store recovers and every window then closes. A defer that could
    // never be resumed would be a hang dressed as safety.
    sink.fail_flushes(false);
    for index in 0..WINDOWS {
        let (decision, receipt) = host
            .handle_close_requested(&window(index))
            .expect("close request is accepted");
        assert_eq!(
            decision,
            GpuiCloseDecision::Close,
            "window-{index}: {:?}",
            receipt.actions()
        );
    }
}

#[test]
fn a_close_with_a_pending_capture_stages_without_flushing() {
    // Recorded rather than asserted as correct. A window moved just before it
    // is closed takes its final capture during the close, stages it, reports
    // the user close, and permits the close — with no flush in the same pass
    // and none scheduled. The placement reaches the sink's staging and its
    // durability then depends on whatever flushes next.
    //
    // The behaviour belongs to the shared coordinator, not to this adapter,
    // so both backends have it. Whether a per-window close should force its
    // own flush is a contract question rather than an adapter bug, and it is
    // written into contract 020's current state rather than changed here.
    let sink = Arc::new(LoadedSink::default());
    let mut host = loaded_host(&sink);
    host.install(window(0), GpuiWindowKey::new(1));
    host.handle_lifecycle_event(WindowLifecycleEvent::Moved {
        window_id: window(0),
        outer_origin: longhorn_core::ScreenPoint::new(10, 10),
    })
    .expect("move is accepted");

    let (decision, receipt) = host
        .handle_close_requested(&window(0))
        .expect("close request is accepted");

    assert!(
        receipt
            .actions()
            .iter()
            .any(|action| matches!(action, GpuiLifecycleAction::Captured { .. }))
    );
    assert!(
        !receipt
            .actions()
            .iter()
            .any(|action| matches!(action, GpuiLifecycleAction::Flushed { .. })),
        "a flush appeared; update this test and contract 020's current state: {:?}",
        receipt.actions()
    );
    assert_eq!(decision, GpuiCloseDecision::Close);
    assert_eq!(sink.staged.lock().unwrap().len(), 1);
}

#[test]
fn the_interlock_never_reads_quiet_while_teardown_has_work_left() {
    // Contract 020 ties restart readiness to this probe. If it can read zero
    // mid-teardown, a restart lands between a capture and its flush.
    let mut host = loaded_host(&Arc::new(LoadedSink::default()));
    install_all(&mut host);

    let outstanding = AtomicUsize::new(host.outstanding());
    let probe = GpuiWindowQuiescenceProbe::new(|| outstanding.load(Ordering::Relaxed));

    for index in 0..WINDOWS {
        outstanding.store(host.outstanding(), Ordering::Relaxed);
        let reported = probe.outstanding().map_or(0, |work| work.count);
        assert_eq!(
            reported,
            host.outstanding(),
            "the probe disagreed with the host before closing window-{index}"
        );

        host.handle_close_requested(&window(index))
            .expect("close request is accepted");
        host.handle_lifecycle_event(WindowLifecycleEvent::Destroyed {
            window_id: window(index),
        })
        .expect("destroy is accepted");
    }

    outstanding.store(host.outstanding(), Ordering::Relaxed);
    assert_eq!(host.outstanding(), 0, "teardown left work behind");
    assert_eq!(probe.outstanding(), None);
}

#[test]
fn a_destroy_for_a_window_already_torn_down_is_refused_rather_than_ignored() {
    // Platforms repeat teardown callbacks. Silently accepting the second one
    // would let a stale key be treated as live; refusing it makes the repeat
    // visible to whoever is bridging the callback.
    let mut host = loaded_host(&Arc::new(LoadedSink::default()));
    install_all(&mut host);

    host.handle_close_requested(&window(0))
        .expect("close request is accepted");
    host.handle_lifecycle_event(WindowLifecycleEvent::Destroyed {
        window_id: window(0),
    })
    .expect("destroy is accepted");

    assert!(
        host.handle_lifecycle_event(WindowLifecycleEvent::Destroyed {
            window_id: window(0),
        })
        .is_err(),
        "a repeated destroy was accepted"
    );
}
