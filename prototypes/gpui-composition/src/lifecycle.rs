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

use gpui::{AnyWindowHandle, App};
use longhorn_core::WindowId;
use longhorn_gpui_windowing::{
    GpuiLifecycleClock, GpuiLifecycleScheduler, GpuiWindowBackend, GpuiWindowCaptureBackend,
    GpuiWindowFacts, GpuiWindowKey, NoopGpuiUserCloseHandler, capture_from_gpui_facts,
};
use longhorn_windowing::{
    CapturedWindowPlacement, MonotonicMillis, ScheduledWindowLifecycleWake, WindowLifecyclePolicy,
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

/// Answers captures from facts the application observed a moment ago.
///
/// The alternative would be holding `&mut App`, which cannot be done, so this
/// is the shape rather than a shortcut. Freshness is the application's
/// responsibility: [`LifecycleHost::observe_into_cache`] runs immediately
/// before every close decision, so what the host captures is what the window
/// looked like when the user clicked.
///
/// A window with no cached facts fails its capture rather than inventing one.
/// A capture that guesses is a placement that is wrong on disk.
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
    windows: Vec<(WindowId, AnyWindowHandle)>,
    capture: CachedCapture,
    scheduler: DrainingScheduler,
}

impl LifecycleHost {
    /// Builds the host over the real store and installs both windows.
    #[must_use]
    pub fn new(windows: Vec<(WindowId, AnyWindowHandle)>) -> Self {
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

        for (window_id, handle) in &windows {
            host.install(
                window_id.clone(),
                GpuiWindowKey::new(handle.window_id().as_u64()),
            );
        }

        Self {
            inner: Rc::new(RefCell::new(host)),
            windows,
            capture,
            scheduler,
        }
    }

    /// Observes every window and feeds the capture cache.
    ///
    /// Called immediately before a close decision, which is the only moment
    /// that matters: a capture taken from stale facts writes the wrong
    /// placement, and this is the one place the application has `&mut App`
    /// and a reason to use it.
    pub fn observe_into_cache(&self, cx: &mut App) {
        let mut backend = longhorn_gpui_windowing_prototype::GpuiAppBackend::new(cx);
        for (_, handle) in &self.windows {
            backend.adopt(*handle);
        }

        let mut host = self.inner.borrow_mut();
        for (window_id, handle) in &self.windows {
            let key = GpuiWindowKey::new(handle.window_id().as_u64());
            match backend.observe(key) {
                Ok(facts) => {
                    if let Ok(scale) = facts.scale_factor() {
                        host.record_scale(window_id, scale);
                    }
                    self.capture
                        .facts
                        .borrow_mut()
                        .insert(window_id.clone(), facts);
                }
                Err(error) => eprintln!("[lifecycle] observe {window_id} failed: {error}"),
            }
        }
    }

    /// Answers gpui's synchronous close question.
    ///
    /// The whole decision is taken here, because `on_should_close` wants a
    /// boolean now. Contract 020 records that Tauri defers differently — it
    /// prevents every user close and lets policy close later — and neither
    /// resumption path is the other's.
    pub fn should_close(&self, cx: &mut App, window_id: &WindowId) -> bool {
        self.observe_into_cache(cx);

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
}

/// Installs the host and binds every window's close callback.
///
/// `on_window_should_close` is where gpui hands back `&mut App`, which is why
/// the whole close decision — observe, capture, flush, answer — happens inside
/// it. There is nowhere else to put it.
pub fn install(windows: Vec<(WindowId, AnyWindowHandle)>, cx: &mut App) {
    let host = Rc::new(LifecycleHost::new(windows.clone()));

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
}
