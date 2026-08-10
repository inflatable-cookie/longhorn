//! Card 176: a real lifecycle host over real windows and a real store.
//!
//! # The shape gpui forces, and why it is not the shape Tauri needs
//!
//! Every Longhorn seam that needs to *see* a window has the same problem here.
//! gpui hands out `&mut App` as a borrow that cannot be held: it is not
//! `Send`, not `Sync`, and lives only for the callback it arrived in.
//! `GpuiAppBackend` is built inside a callback and dropped at the end of it.
//!
//! `GpuiWindowLifecycleHost` owns its services for its whole life, so a
//! service that needs `&mut App` cannot be one of them. Two consequences, both
//! visible below:
//!
//! - **capture is fed, not fetched.** `GpuiWindowCaptureBackend::capture`
//!   takes no host context, so [`CachedCapture`] answers from facts the
//!   application observed a moment earlier, at a point where it had `&mut App`.
//! - **the scheduler cannot arm its own timer.** `GpuiLifecycleScheduler`
//!   takes no context either, and gpui's executors need one, so
//!   [`DrainingScheduler`] records deadlines and the application drains them
//!   where it can.
//!
//! Both services are therefore *handles onto shared state* rather than owned
//! implementations: `GpuiWindowLifecycleHost` takes its services and exposes
//! no way back to them, which is correct for a host whose services are
//! self-sufficient and leaves an application here with nothing to feed. An
//! `Rc<RefCell<..>>` on each side costs nothing and needs no adapter change.
//!
//! Neither is a defect in the adapter. Tauri's equivalents are self-sufficient
//! because `tauri::WebviewWindow` is a cloneable, `Send` handle a service can
//! keep; gpui's is a borrow. The seams are the same shape and only one host
//! can satisfy them alone.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Instant};

use gpui::{AnyWindowHandle, App, Window};
use longhorn_core::WindowId;
use longhorn_gpui_windowing::{
    GpuiLifecycleClock, GpuiLifecycleScheduler, GpuiWindowCaptureBackend, GpuiWindowFacts,
    GpuiWindowKey, NoopGpuiUserCloseHandler, capture_from_gpui_facts,
};
use longhorn_windowing::{
    CapturedWindowPlacement, MonotonicMillis, ScheduledWindowLifecycleWake, WindowLifecycleEvent,
    WindowLifecyclePolicy,
};

use crate::store::sink;

/// A real monotonic clock, counted from process start.
pub struct ProcessClock {
    started: Instant,
}

impl ProcessClock {
    fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl GpuiLifecycleClock for ProcessClock {
    fn now(&self) -> MonotonicMillis {
        MonotonicMillis::new(u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX))
    }
}

/// Records deadlines for the application to drain.
///
/// `schedule` is handed a wake and no way to arm anything: gpui's foreground
/// and background executors both need a context this trait does not carry.
/// Recording is the whole of what a scheduler can do here, and the application
/// arms the timers where it has `&mut App`.
#[derive(Clone, Default)]
pub struct DrainingScheduler {
    pending: Rc<RefCell<Vec<ScheduledWindowLifecycleWake>>>,
}

impl GpuiLifecycleScheduler for DrainingScheduler {
    fn schedule(&mut self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        self.pending.borrow_mut().push(wake);
        Ok(())
    }
}

/// Answers captures from facts each window recorded from its own render.
///
/// The alternative would be holding `&mut App`, which cannot be done, so this
/// is the shape rather than a shortcut.
///
/// **Fed on render, not at close.** The first version observed every window
/// immediately before the close decision, which is the one moment it cannot
/// work: `on_window_should_close` runs inside the closing window's own
/// dispatch, gpui has taken that window out of the application's map, and the
/// observation fails with "window not found". The capture then failed, nothing
/// staged, and the flush succeeded in 42.8µs by having nothing to write —
/// against the 15-22ms a real write costs. Nothing was lost that time because
/// the window had not moved. It would have been, silently, if it had.
///
/// A render has `&mut Window` for its own window and gpui redraws on move and
/// resize, so recording there is both possible and fresh.
///
/// A window with no cached facts still fails its capture rather than inventing
/// one. A capture that guesses is a placement that is wrong on disk.
#[derive(Clone, Default)]
pub struct CachedCapture {
    facts: Rc<RefCell<BTreeMap<WindowId, GpuiWindowFacts>>>,
}

impl GpuiWindowCaptureBackend for CachedCapture {
    fn capture(
        &mut self,
        window_id: &WindowId,
        _key: GpuiWindowKey,
    ) -> Result<CapturedWindowPlacement, String> {
        let facts = self.facts.borrow();
        let facts = facts
            .get(window_id)
            .ok_or_else(|| format!("no observed facts for {window_id}"))?;
        capture_from_gpui_facts(window_id, facts)
    }
}

type Host = longhorn_gpui_windowing::GpuiWindowLifecycleHost<
    ProcessClock,
    DrainingScheduler,
    CachedCapture,
    NoopGpuiUserCloseHandler,
>;

/// The lifecycle host, shared across both windows' callbacks.
pub struct LifecycleHost {
    inner: Rc<RefCell<Host>>,
    capture: CachedCapture,
    scheduler: DrainingScheduler,
    /// The last bounds each window rendered at, so a move can be noticed.
    last_bounds: RefCell<BTreeMap<WindowId, longhorn_gpui_windowing::GpuiLogicalRect>>,
}

impl LifecycleHost {
    /// Builds the host over the real store and installs both windows.
    #[must_use]
    pub fn new(windows: &[(WindowId, AnyWindowHandle)]) -> Self {
        let capture = CachedCapture::default();
        let scheduler = DrainingScheduler::default();
        let mut host = Host::new(
            WindowLifecyclePolicy::recommended(),
            longhorn_gpui_windowing::GpuiWindowLifecycleServices {
                clock: ProcessClock::new(),
                scheduler: scheduler.clone(),
                capture: capture.clone(),
                user_close: NoopGpuiUserCloseHandler,
                sink: Box::new(sink()),
            },
        );

        for (window_id, handle) in windows {
            host.install(
                window_id.clone(),
                GpuiWindowKey::new(handle.window_id().as_u64()),
            );
        }

        // The window list is not kept. It was only ever needed to observe at
        // close, and that is the one place observation cannot work.
        Self {
            inner: Rc::new(RefCell::new(host)),
            capture,
            scheduler,
            last_bounds: RefCell::new(BTreeMap::new()),
        }
    }

    /// Records one window's facts from its own render, and tells the
    /// coordinator when they changed.
    ///
    /// Both halves are load-bearing, and the second was missing for a while.
    ///
    /// Feeding the cache makes a capture *possible*: a window has `&mut Window`
    /// for itself here, and observing it at close is impossible.
    ///
    /// Reporting the move makes a capture *happen*. The coordinator schedules
    /// a capture when it is told state changed and not otherwise, so a cache
    /// that is fresh and a coordinator that was never told produce a close
    /// with no capture, nothing staged, and a flush that succeeds by writing
    /// nothing. Measured: a window moved from y=120 to y=324, closed, and the
    /// store still held y=120.
    pub fn record_from_render(&self, window_id: &WindowId, window: &Window) {
        let facts = longhorn_gpui_windowing_prototype::facts_from_window(window);
        let bounds = facts.bounds();

        let changed = self
            .last_bounds
            .borrow()
            .get(window_id)
            .is_none_or(|previous| *previous != bounds);

        if let Ok(scale) = facts.scale_factor() {
            self.inner.borrow_mut().record_scale(window_id, scale);
        }
        self.capture
            .facts
            .borrow_mut()
            .insert(window_id.clone(), facts);

        if !changed {
            return;
        }
        self.last_bounds
            .borrow_mut()
            .insert(window_id.clone(), bounds);

        // gpui has no "window moved" callback an application can bind, so the
        // move is noticed by comparing renders. A product with a real event
        // source uses that instead; what matters is that the coordinator hears
        // about it at all.
        let moved = WindowLifecycleEvent::Moved {
            window_id: window_id.clone(),
            outer_origin: match bounds.to_screen_origin() {
                Ok(origin) => origin,
                Err(error) => {
                    eprintln!("[lifecycle] {window_id} has unusable bounds: {error}");
                    return;
                }
            },
        };
        if let Err(error) = self.inner.borrow_mut().handle_lifecycle_event(moved) {
            eprintln!("[lifecycle] {window_id} move refused: {error}");
        }
    }

    /// Answers gpui's synchronous close question.
    ///
    /// The whole decision is taken here, because `on_should_close` wants a
    /// boolean now. Contract 020 records that Tauri defers differently — it
    /// prevents every user close and lets policy close later — and neither
    /// resumption path is the other's.
    pub fn should_close(&self, _cx: &mut App, window_id: &WindowId) -> bool {
        // Nothing is observed here. The facts arrived from this window's own
        // renders, which is the only route that works — see `CachedCapture`.
        let started = Instant::now();
        let mut host = self.inner.borrow_mut();
        let decision = match host.handle_close_requested(window_id) {
            Ok((decision, receipt)) => {
                eprintln!(
                    "[lifecycle] close {window_id} -> {decision:?} in {:?}: {:?}",
                    started.elapsed(),
                    receipt.actions()
                );
                decision
            }
            Err(error) => {
                eprintln!("[lifecycle] close {window_id} refused: {error}");
                return false;
            }
        };

        eprintln!(
            "[lifecycle] outstanding after close: {} (this window {})",
            host.outstanding(),
            host.outstanding_for(window_id)
        );
        decision.should_close()
    }

    /// Drains deadlines the scheduler recorded and delivers them.
    ///
    /// Synchronously rather than on a timer. A real product arms
    /// `cx.background_executor().timer` and delivers the wake when it fires;
    /// draining immediately is enough to show the path and keeps the example
    /// about durability rather than about scheduling.
    pub fn drain_wakes(&self) {
        let pending = self.scheduler.pending.borrow_mut().split_off(0);
        let mut host = self.inner.borrow_mut();
        for wake in pending {
            if let Err(error) = host.handle_scheduled_wake(&wake) {
                eprintln!("[lifecycle] wake failed: {error}");
            }
        }
    }

    /// How long the host thinks it still has to do.
    #[must_use]
    pub fn outstanding(&self) -> usize {
        self.inner.borrow().outstanding()
    }

    /// Writes everything still staged, on the way out.
    ///
    /// Called from the close callback after the decision, because a GPUI
    /// window is gone the moment `on_should_close` returns `true` and whatever
    /// it staged has no later chance. A product with a real shutdown path
    /// calls it there instead; the point is that something calls it.
    pub fn shutdown_flush(&self) {
        let receipt = self.inner.borrow_mut().shutdown_flush();
        eprintln!(
            "[lifecycle] shutdown flush: {:?}, complete={}",
            receipt.outcome(),
            receipt.is_complete()
        );
    }
}

/// Installs the host and binds every window's close callback.
///
/// `on_window_should_close` is where gpui hands back `&mut App`, which is why
/// the whole close decision — observe, capture, flush, answer — happens inside
/// it. There is nowhere else to put it.
pub fn install(windows: Vec<(WindowId, AnyWindowHandle)>, cx: &mut App) -> Rc<LifecycleHost> {
    let host = Rc::new(LifecycleHost::new(&windows));

    for (window_id, handle) in windows {
        let host = Rc::clone(&host);
        let window_id = window_id.clone();
        // `update` to reach the `Window`, which is what carries the callback.
        let bound = handle.update(cx, move |_view, window, cx| {
            let host = Rc::clone(&host);
            let window_id = window_id.clone();
            window.on_window_should_close(cx, move |_window, cx| {
                let permitted = host.should_close(cx, &window_id);
                // Deadlines the close scheduled are delivered before the
                // answer is returned, because after `true` there may be no
                // window left to deliver them to.
                host.drain_wakes();
                if permitted {
                    // And the staged placement is written before the window
                    // goes, which is the whole point: without this the capture
                    // a close takes is never persisted.
                    host.shutdown_flush();
                }
                eprintln!(
                    "[lifecycle] {window_id} outstanding after drain: {}",
                    host.outstanding()
                );
                permitted
            });
        });
        if let Err(error) = bound {
            eprintln!("[lifecycle] could not bind close for a window: {error}");
        }
    }

    host
}
