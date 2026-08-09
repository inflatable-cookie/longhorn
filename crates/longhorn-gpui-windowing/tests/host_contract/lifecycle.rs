//! Contract 020: "Lifecycle events — created, moved, resized, focus change,
//! close requested, destroyed, translated into Longhorn's vocabulary" and
//! "Close handling — a host must let Longhorn observe and defer a close".

use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use longhorn_core::WindowId;
use longhorn_gpui_windowing::{
    GpuiCloseDecision, GpuiLifecycleAction, GpuiLifecycleClock, GpuiLifecycleScheduler,
    GpuiLogicalRect, GpuiLogicalSize, GpuiUserCloseHandler, GpuiWindowCaptureBackend,
    GpuiWindowEvent, GpuiWindowKey, GpuiWindowLifecycleHost, GpuiWindowLifecycleServices,
    GpuiWindowQuiescenceProbe, NoopGpuiUserCloseHandler, translate_gpui_window_event,
};
use longhorn_update::{QuiescenceKind, QuiescenceProbe};
use longhorn_windowing::{
    CapturedDisplayAssociation, CapturedWindowPlacement, MonotonicMillis,
    ScheduledWindowLifecycleWake, WindowLifecycleEvent, WindowLifecyclePolicy, WindowPlacementSink,
};

use super::support::{id, placement, scale};

struct FixedClock(AtomicU64);

impl GpuiLifecycleClock for FixedClock {
    fn now(&self) -> MonotonicMillis {
        MonotonicMillis::new(self.0.load(Ordering::Relaxed))
    }
}

#[derive(Default)]
struct RecordingScheduler {
    wakes: Vec<ScheduledWindowLifecycleWake>,
}

impl GpuiLifecycleScheduler for RecordingScheduler {
    fn schedule(&mut self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        self.wakes.push(wake);
        Ok(())
    }
}

struct FixedCapture;

impl GpuiWindowCaptureBackend for FixedCapture {
    fn capture(
        &mut self,
        window_id: &WindowId,
        _key: GpuiWindowKey,
    ) -> Result<CapturedWindowPlacement, String> {
        Ok(CapturedWindowPlacement::new(
            window_id.clone(),
            placement(0, 0, 800, 600),
            false,
            CapturedDisplayAssociation::Unresolved,
        ))
    }
}

#[derive(Default)]
struct RecordingSink {
    staged: Mutex<Vec<WindowId>>,
    flushes: Mutex<usize>,
}

impl WindowPlacementSink for RecordingSink {
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
    ) -> Result<longhorn_windowing::WindowPlacementFlushTicket, String> {
        *self.flushes.lock().unwrap() += 1;
        Ok(longhorn_windowing::WindowPlacementFlushTicket::completed())
    }
}

struct RefusingCloseHandler;

impl GpuiUserCloseHandler for RefusingCloseHandler {
    fn user_close(&mut self, _window_id: &WindowId) -> Result<(), String> {
        Err("product policy refused".to_owned())
    }
}

type Host<U> = GpuiWindowLifecycleHost<FixedClock, RecordingScheduler, FixedCapture, U>;

fn host<U: GpuiUserCloseHandler>(user_close: U) -> Host<U> {
    GpuiWindowLifecycleHost::new(
        WindowLifecyclePolicy::recommended(),
        GpuiWindowLifecycleServices {
            clock: FixedClock(AtomicU64::new(1_000)),
            scheduler: RecordingScheduler::default(),
            capture: FixedCapture,
            user_close,
            sink: Box::new(RecordingSink::default()),
        },
    )
}

#[test]
fn a_gpui_resize_becomes_both_a_resize_and_a_scale_change() {
    // GPUI has no scale-change callback. `on_resize` carries the scale, so one
    // native event is two Longhorn events when a window crosses displays.
    // Tauri's translation is one-to-one because Tauri has a dedicated event.
    let events = translate_gpui_window_event(
        &id("main"),
        GpuiWindowEvent::Resized {
            content_size: GpuiLogicalSize::new(1024.0, 768.0),
            scale: 2.0,
        },
        Some(scale(1000)),
    )
    .unwrap();

    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], WindowLifecycleEvent::Resized { .. }));
    assert!(matches!(
        events[1],
        WindowLifecycleEvent::ScaleChanged { .. }
    ));
}

#[test]
fn an_unchanged_scale_produces_no_scale_change() {
    let events = translate_gpui_window_event(
        &id("main"),
        GpuiWindowEvent::Resized {
            content_size: GpuiLogicalSize::new(1024.0, 768.0),
            scale: 2.0,
        },
        Some(scale(2000)),
    )
    .unwrap();

    assert_eq!(events.len(), 1);
}

#[test]
fn every_contract_020_lifecycle_event_has_a_translation() {
    let moved = translate_gpui_window_event(
        &id("main"),
        GpuiWindowEvent::Moved {
            bounds: GpuiLogicalRect::new(40.0, 60.0, 800.0, 600.0),
        },
        None,
    )
    .unwrap();
    assert!(matches!(moved[0], WindowLifecycleEvent::Moved { .. }));

    // Focus change: GPUI reports gain and loss through one callback, and
    // Longhorn's vocabulary has only Blurred, so a gain translates to nothing.
    // Tauri drops `Focused(true)` for the same reason.
    assert!(
        translate_gpui_window_event(
            &id("main"),
            GpuiWindowEvent::ActiveStatusChanged { active: true },
            None
        )
        .unwrap()
        .is_empty()
    );
    let blurred = translate_gpui_window_event(
        &id("main"),
        GpuiWindowEvent::ActiveStatusChanged { active: false },
        None,
    )
    .unwrap();
    assert!(matches!(blurred[0], WindowLifecycleEvent::Blurred { .. }));

    let requested =
        translate_gpui_window_event(&id("main"), GpuiWindowEvent::CloseRequested, None).unwrap();
    assert!(matches!(
        requested[0],
        WindowLifecycleEvent::CloseRequested { .. }
    ));

    let destroyed =
        translate_gpui_window_event(&id("main"), GpuiWindowEvent::Closed, None).unwrap();
    assert!(matches!(
        destroyed[0],
        WindowLifecycleEvent::Destroyed { .. }
    ));
}

#[test]
fn a_close_request_flushes_and_then_permits_the_close() {
    // With nothing pending the pure coordinator asks for a bounded flush and
    // reports the user close. There is no capture, because nothing changed.
    let mut host = host(NoopGpuiUserCloseHandler);
    host.install(id("main"), GpuiWindowKey::new(1));

    let (decision, receipt) = host.handle_close_requested(&id("main")).unwrap();

    assert!(
        receipt
            .actions()
            .iter()
            .any(|action| matches!(action, GpuiLifecycleAction::Flushed { .. }))
    );
    assert!(
        receipt
            .actions()
            .iter()
            .any(|action| matches!(action, GpuiLifecycleAction::UserCloseReported))
    );
    assert_eq!(decision, GpuiCloseDecision::Close);
    assert!(decision.should_close());
}

#[test]
fn a_close_request_after_a_move_captures_the_final_state_first() {
    let mut host = host(NoopGpuiUserCloseHandler);
    host.install(id("main"), GpuiWindowKey::new(1));
    host.handle_lifecycle_event(WindowLifecycleEvent::Moved {
        window_id: id("main"),
        outer_origin: longhorn_core::ScreenPoint::new(10, 10),
    })
    .unwrap();

    let (decision, receipt) = host.handle_close_requested(&id("main")).unwrap();

    assert!(
        receipt
            .actions()
            .iter()
            .any(|action| matches!(action, GpuiLifecycleAction::Captured { .. })),
        "{:?}",
        receipt.actions()
    );
    assert_eq!(decision, GpuiCloseDecision::Close);
}

#[test]
fn a_close_is_deferred_when_product_policy_refuses_it() {
    // Contract 020 requires that a host "let Longhorn observe and defer a
    // close, because restart readiness depends on it". GPUI's `on_should_close`
    // returns a bool the platform acts on immediately, so the whole decision
    // is taken inside the callback. Longhorn's Tauri host calls
    // `api.prevent_close()` on every user close and lets product policy close
    // the window later by its own route. Both defer; the resumption paths are
    // not the same, and only this one has to answer now.
    let mut host = host(RefusingCloseHandler);
    host.install(id("main"), GpuiWindowKey::new(1));

    let (decision, receipt) = host.handle_close_requested(&id("main")).unwrap();

    assert!(
        receipt
            .actions()
            .iter()
            .any(|action| matches!(action, GpuiLifecycleAction::UserCloseFailed { .. }))
    );
    assert_eq!(decision, GpuiCloseDecision::Defer);
    assert!(!decision.should_close());
}

#[test]
fn the_host_reports_its_own_outstanding_work_to_the_restart_interlock() {
    // Contract 020: "Quiescence participation — the host reports its own
    // outstanding work to the restart interlock." `QuiescenceProbe` is a plain
    // trait in `longhorn-update`, so this needed no host-specific mechanism
    // and the GPUI host satisfies it unchanged.
    let quiet = GpuiWindowQuiescenceProbe::new(|| 0);
    assert_eq!(quiet.outstanding(), None);

    let busy = GpuiWindowQuiescenceProbe::new(|| 2);
    let outstanding = busy.outstanding().expect("two items are outstanding");
    assert_eq!(outstanding.kind, QuiescenceKind::PendingFlush);
    assert_eq!(outstanding.count, 2);
}

#[test]
fn an_event_for_an_uninstalled_window_is_refused() {
    let mut host = host(NoopGpuiUserCloseHandler);

    assert!(
        host.handle_gpui_event(
            &id("ghost"),
            GpuiWindowEvent::ActiveStatusChanged { active: false }
        )
        .is_err()
    );
}

#[test]
fn a_windowed_capture_records_the_content_size_not_the_frame() {
    // Found by persisting two real windows: gpui reported bounds 560x592 and
    // content 560x560 for a window asked for 560x560, and the capture stored
    // 592 in a field that means *inner* size. Applying that back grows the
    // window by the titlebar, and it compounds every save-and-restore cycle.
    let facts = longhorn_gpui_windowing::GpuiWindowFacts::new(
        longhorn_gpui_windowing::GpuiLogicalRect::new(120.0, 120.0, 560.0, 592.0),
        GpuiLogicalSize::new(560.0, 560.0),
        longhorn_gpui_windowing::GpuiWindowBoundsState::Windowed(
            longhorn_gpui_windowing::GpuiLogicalRect::new(120.0, 120.0, 560.0, 592.0),
        ),
        2.0,
        false,
    );

    let capture = longhorn_gpui_windowing::capture_from_gpui_facts(&id("main"), &facts)
        .expect("a windowed capture");

    assert_eq!(capture.normal_placement().inner_size().height(), 560);
    assert_eq!(capture.normal_placement().outer_origin().y().get(), 120);
}

#[test]
fn a_maximized_capture_keeps_the_restore_extent() {
    // The case with no clean answer: `content_size` describes the maximized
    // window, and the restore bounds describe where it returns to. The frame
    // difference is accepted rather than hidden, and this pins that choice.
    let facts = longhorn_gpui_windowing::GpuiWindowFacts::new(
        longhorn_gpui_windowing::GpuiLogicalRect::new(0.0, 0.0, 1920.0, 1080.0),
        GpuiLogicalSize::new(1920.0, 1048.0),
        longhorn_gpui_windowing::GpuiWindowBoundsState::Maximized(
            longhorn_gpui_windowing::GpuiLogicalRect::new(120.0, 120.0, 560.0, 592.0),
        ),
        2.0,
        false,
    );

    let capture = longhorn_gpui_windowing::capture_from_gpui_facts(&id("main"), &facts)
        .expect("a maximized capture");

    assert!(capture.is_maximized());
    assert_eq!(capture.normal_placement().inner_size().height(), 592);
}
