mod capture;
mod quiescence;
mod receipt;
mod services;
mod shutdown;
mod translation;

use std::collections::BTreeMap;

pub use capture::capture_from_gpui_facts;
pub use quiescence::GpuiWindowQuiescenceProbe;
pub use receipt::{GpuiCloseDecision, GpuiLifecycleAction, GpuiLifecycleReceipt, close_is_safe};
pub use services::{GpuiLifecycleScheduler, GpuiWindowCaptureBackend, GpuiWindowLifecycleServices};
pub use shutdown::GpuiShutdownReceipt;
use translation::translation_error;
pub use translation::{GpuiWindowEvent, translate_gpui_window_event};

use longhorn_core::{ScaleFactor, WindowId};
use longhorn_windowing::{
    CaptureGeneration, ScheduledWindowLifecycleWake, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget, WindowLifecycleClock, WindowLifecycleCoordinator, WindowLifecycleDirective,
    WindowLifecycleEvent, WindowLifecyclePolicy, WindowUserCloseHandler, flush_window_placements,
};

use crate::{GpuiWindowKey, GpuiWindowLifecycleError, scale_factor_from_gpui};

/// Which counter one scheduling attempt raised, so a failure rolls back the
/// right one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScheduledKind {
    Capture,
    Flush,
}

/// Whether one window has a capture or a flush still to settle.
///
/// Flags rather than counters. The coordinator debounces, so every move
/// reschedules the *same* pending capture with a fresh deadline — counting
/// scheduled deadlines made a dragged window accumulate one per event while
/// only one ever settled. The window then could never close and the restart
/// interlock never read quiet. A window has at most one capture and one flush
/// in flight, and that is what this records.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct OutstandingWindowWork {
    capture_pending: bool,
    flush_pending: bool,
}

impl OutstandingWindowWork {
    const fn total(&self) -> usize {
        self.capture_pending as usize + self.flush_pending as usize
    }
}

struct InstalledWindow {
    key: GpuiWindowKey,
    scale: Option<ScaleFactor>,
}

/// GPUI listener and I/O adapter over the pure lifecycle coordinator.
///
/// Owned by value on the main thread. The Tauri host is an `Arc` with a
/// `Mutex` around its coordinator and its window map, because Tauri delivers
/// events on threads the host does not control and its flushes are spawned
/// onto a blocking pool. Neither is true here.
pub struct GpuiWindowLifecycleHost<C, S, B, U> {
    coordinator: WindowLifecycleCoordinator,
    /// Supplies the shared flush bound for the shutdown aggregate.
    policy: WindowLifecyclePolicy,
    windows: BTreeMap<WindowId, InstalledWindow>,
    services: GpuiWindowLifecycleServices<C, S, B, U>,
    /// Outstanding captures and flushes, per window.
    ///
    /// Per window rather than two totals. A total answers the restart
    /// interlock, which is an application-wide question, and cannot answer
    /// "may *this* window close" — with two totals, a window with nothing to
    /// save was refused its close because a different window had been moved.
    /// Only a multi-window teardown shows it; every earlier lifecycle test
    /// used one window.
    outstanding: BTreeMap<WindowId, OutstandingWindowWork>,
}

impl<C, S, B, U> GpuiWindowLifecycleHost<C, S, B, U>
where
    C: WindowLifecycleClock,
    S: GpuiLifecycleScheduler,
    B: GpuiWindowCaptureBackend,
    U: WindowUserCloseHandler,
{
    /// Constructs an empty host with caller-owned policy and seams.
    #[must_use]
    pub fn new(
        policy: WindowLifecyclePolicy,
        services: GpuiWindowLifecycleServices<C, S, B, U>,
    ) -> Self {
        Self {
            coordinator: WindowLifecycleCoordinator::new(policy),
            policy,
            windows: BTreeMap::new(),
            services,
            outstanding: BTreeMap::new(),
        }
    }

    /// Installs one managed window.
    pub fn install(&mut self, window_id: WindowId, key: GpuiWindowKey) {
        self.windows
            .insert(window_id, InstalledWindow { key, scale: None });
    }

    /// Returns whether a window is installed.
    #[must_use]
    pub fn is_installed(&self, window_id: &WindowId) -> bool {
        self.windows.contains_key(window_id)
    }

    /// Records the scale a window is currently on.
    ///
    /// GPUI reports scale only through a resize callback and
    /// `Window::scale_factor`. Without a recorded previous value the adapter
    /// cannot tell a resize from a display change, so a caller that observes a
    /// window at rest should seed it here.
    pub fn record_scale(&mut self, window_id: &WindowId, scale: ScaleFactor) {
        if let Some(installed) = self.windows.get_mut(window_id) {
            installed.scale = Some(scale);
        }
    }

    /// Handles one native GPUI callback.
    pub fn handle_gpui_event(
        &mut self,
        window_id: &WindowId,
        event: GpuiWindowEvent,
    ) -> Result<Vec<GpuiLifecycleReceipt>, GpuiWindowLifecycleError> {
        let previous_scale = self
            .windows
            .get(window_id)
            .ok_or_else(|| GpuiWindowLifecycleError::UnknownWindow {
                window_id: window_id.clone(),
            })?
            .scale;
        let translated = translate_gpui_window_event(window_id, event, previous_scale)?;
        if let GpuiWindowEvent::Resized { scale, .. } = event {
            let scale = scale_factor_from_gpui(scale).map_err(translation_error)?;
            self.record_scale(window_id, scale);
        }
        translated
            .into_iter()
            .map(|event| self.handle_lifecycle_event(event))
            .collect()
    }

    /// Answers GPUI's `on_should_close`.
    ///
    /// The decision is synchronous by necessity: GPUI's callback returns a
    /// `bool` and the platform acts on it immediately. Longhorn's close path
    /// therefore runs its capture and its bounded flush inline, and the answer
    /// is `Defer` if that work did not settle or product policy refused.
    ///
    /// The two hosts differ in what happens next. Longhorn's Tauri host calls
    /// `api.prevent_close()` on every user close and hands the decision to
    /// product policy, which closes the window later by its own route. GPUI
    /// has no such handle: a `false` here is the whole answer, and a deferred
    /// close resumes only when the user asks again. Contract 020 requires that
    /// a host let Longhorn "observe and defer a close"; both do, and neither
    /// resumption path is the other's.
    pub fn handle_close_requested(
        &mut self,
        window_id: &WindowId,
    ) -> Result<(GpuiCloseDecision, GpuiLifecycleReceipt), GpuiWindowLifecycleError> {
        let receipt = self.handle_lifecycle_event(WindowLifecycleEvent::CloseRequested {
            window_id: window_id.clone(),
        })?;
        let decision = if close_is_safe(&receipt) && self.outstanding_for(window_id) == 0 {
            GpuiCloseDecision::Close
        } else {
            GpuiCloseDecision::Defer
        };
        Ok((decision, receipt))
    }

    /// Delivers one wake previously accepted by the scheduler.
    pub fn handle_scheduled_wake(
        &mut self,
        wake: &ScheduledWindowLifecycleWake,
    ) -> Result<GpuiLifecycleReceipt, GpuiWindowLifecycleError> {
        self.handle_lifecycle_event(wake.event().clone())
    }

    /// Handles one already translated lifecycle input.
    pub fn handle_lifecycle_event(
        &mut self,
        event: WindowLifecycleEvent,
    ) -> Result<GpuiLifecycleReceipt, GpuiWindowLifecycleError> {
        self.handle_event(event, &mut None)
    }

    fn handle_event(
        &mut self,
        event: WindowLifecycleEvent,
        shutdown_targets: &mut Option<&mut Vec<WindowFlushTarget>>,
    ) -> Result<GpuiLifecycleReceipt, GpuiWindowLifecycleError> {
        let window_id = event.window_id().clone();
        if !self.windows.contains_key(&window_id) {
            return Err(GpuiWindowLifecycleError::UnknownWindow { window_id });
        }
        let event_kind = event.kind();
        let now = self.services.clock.now();
        let directives = self.coordinator.handle(now, event).map_err(|error| {
            GpuiWindowLifecycleError::Coordination {
                detail: error.to_string(),
            }
        })?;

        let mut actions = Vec::with_capacity(directives.len());
        self.execute_directives(directives, &mut actions, shutdown_targets)?;
        Ok(GpuiLifecycleReceipt::new(window_id, event_kind, actions))
    }

    /// Returns the host's own outstanding work for the restart interlock.
    ///
    /// Application-wide on purpose: a restart takes every window with it, so
    /// the interlock wants the total. Closing one window does not, which is
    /// what [`Self::outstanding_for`] is for.
    #[must_use]
    pub fn outstanding(&self) -> usize {
        self.outstanding
            .values()
            .map(OutstandingWindowWork::total)
            .sum()
    }

    /// Returns one window's own outstanding work.
    ///
    /// The number a close decision reads. A window with nothing pending may
    /// close while another window still has state to save; the two questions
    /// are not the same and were answered by the same counter until a
    /// multi-window teardown said otherwise.
    #[must_use]
    pub fn outstanding_for(&self, window_id: &WindowId) -> usize {
        self.outstanding
            .get(window_id)
            .map_or(0, OutstandingWindowWork::total)
    }

    fn note_capture(&mut self, window_id: &WindowId) {
        self.outstanding
            .entry(window_id.clone())
            .or_default()
            .capture_pending = true;
    }

    fn note_flush(&mut self, window_id: &WindowId) {
        self.outstanding
            .entry(window_id.clone())
            .or_default()
            .flush_pending = true;
    }

    fn settle_capture(&mut self, window_id: &WindowId) {
        if let Some(work) = self.outstanding.get_mut(window_id) {
            work.capture_pending = false;
        }
    }

    fn settle_flush(&mut self, window_id: &WindowId) {
        if let Some(work) = self.outstanding.get_mut(window_id) {
            work.flush_pending = false;
        }
    }

    fn execute_directives(
        &mut self,
        directives: Vec<WindowLifecycleDirective>,
        actions: &mut Vec<GpuiLifecycleAction>,
        shutdown_targets: &mut Option<&mut Vec<WindowFlushTarget>>,
    ) -> Result<(), GpuiWindowLifecycleError> {
        for directive in directives {
            let action = match directive {
                WindowLifecycleDirective::CaptureNow {
                    window_id,
                    generation,
                    ..
                } => {
                    let action = self.capture_now(&window_id, generation);
                    let event = if matches!(action, GpuiLifecycleAction::Captured { .. }) {
                        WindowLifecycleEvent::CaptureCompleted {
                            window_id,
                            generation,
                        }
                    } else {
                        WindowLifecycleEvent::CaptureFailed {
                            window_id,
                            generation,
                        }
                    };
                    actions.push(action);
                    let nested = self
                        .coordinator
                        .handle(self.services.clock.now(), event)
                        .map_err(|error| GpuiWindowLifecycleError::Coordination {
                            detail: error.to_string(),
                        })?;
                    self.execute_directives(nested, actions, shutdown_targets)?;
                    continue;
                }
                WindowLifecycleDirective::Flush {
                    window_id,
                    generation,
                    timeout,
                    reason,
                } => {
                    self.settle_flush(&window_id);
                    let target = WindowFlushTarget::new(window_id, generation);
                    if let Some(targets) = shutdown_targets {
                        targets.push(target);
                        continue;
                    }
                    let request = WindowFlushRequest::new(
                        vec![target],
                        timeout,
                        WindowFlushScope::Window { reason },
                    );
                    let outcome = flush_window_placements(&*self.services.sink, &request);
                    GpuiLifecycleAction::Flushed {
                        request: Box::new(request),
                        outcome,
                    }
                }
                WindowLifecycleDirective::Ignore { reason, .. } => {
                    GpuiLifecycleAction::Ignored { reason }
                }
                WindowLifecycleDirective::ScheduleCapture {
                    window_id,
                    generation,
                    due_at,
                } => {
                    self.note_capture(&window_id);
                    let rollback = window_id.clone();
                    self.schedule(
                        ScheduledWindowLifecycleWake::new(
                            due_at,
                            WindowLifecycleEvent::CaptureDeadline {
                                window_id,
                                generation,
                            },
                        ),
                        &rollback,
                        ScheduledKind::Capture,
                    )
                }
                WindowLifecycleDirective::ScheduleFlush {
                    window_id,
                    generation,
                    due_at,
                } => {
                    self.note_flush(&window_id);
                    let rollback = window_id.clone();
                    self.schedule(
                        ScheduledWindowLifecycleWake::new(
                            due_at,
                            WindowLifecycleEvent::FlushDeadline {
                                window_id,
                                generation,
                            },
                        ),
                        &rollback,
                        ScheduledKind::Flush,
                    )
                }
                WindowLifecycleDirective::UserClose { window_id } => {
                    match self.services.user_close.user_close(&window_id) {
                        Ok(()) => GpuiLifecycleAction::UserCloseReported,
                        Err(detail) => GpuiLifecycleAction::UserCloseFailed { detail },
                    }
                }
                WindowLifecycleDirective::Forget { window_id } => {
                    self.windows.remove(&window_id);
                    // A forgotten window's counters go with it. Leaving them would
                    // hold the restart interlock open forever on work that has no
                    // window to complete against.
                    self.outstanding.remove(&window_id);
                    GpuiLifecycleAction::Forgotten
                }
            };
            actions.push(action);
        }
        Ok(())
    }

    fn schedule(
        &mut self,
        wake: ScheduledWindowLifecycleWake,
        window_id: &WindowId,
        kind: ScheduledKind,
    ) -> GpuiLifecycleAction {
        match self.services.scheduler.schedule(wake.clone()) {
            Ok(()) => GpuiLifecycleAction::Scheduled {
                wake: Box::new(wake),
            },
            Err(detail) => {
                // Roll back the counter this call incremented. It always
                // decremented the capture counter, so a flush that failed to
                // schedule left its own count raised and took a capture down
                // with it.
                match kind {
                    ScheduledKind::Capture => self.settle_capture(window_id),
                    ScheduledKind::Flush => self.settle_flush(window_id),
                }
                GpuiLifecycleAction::ScheduleFailed { detail }
            }
        }
    }

    fn capture_now(
        &mut self,
        window_id: &WindowId,
        generation: CaptureGeneration,
    ) -> GpuiLifecycleAction {
        self.settle_capture(window_id);
        let Some(installed) = self.windows.get(window_id) else {
            return GpuiLifecycleAction::CaptureFailed {
                detail: format!("window {window_id} is not installed"),
            };
        };
        let key = installed.key;
        match self.services.capture.capture(window_id, key) {
            Ok(placement) => match self.services.sink.stage(&placement) {
                Ok(()) => GpuiLifecycleAction::Captured { generation },
                Err(detail) => GpuiLifecycleAction::CaptureFailed { detail },
            },
            Err(detail) => GpuiLifecycleAction::CaptureFailed { detail },
        }
    }
}
