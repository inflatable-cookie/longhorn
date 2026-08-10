use std::collections::BTreeMap;

use longhorn_core::{ScaleFactor, WindowId, WindowPlacement, report_best_effort_failure};
use longhorn_update::{OutstandingWork, QuiescenceKind, QuiescenceProbe};
use longhorn_windowing::{
    CaptureGeneration, CapturedDisplayAssociation, CapturedWindowPlacement, MonotonicMillis,
    ScheduledWindowLifecycleWake, WindowFlushOutcome, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget, WindowLifecycleCoordinator, WindowLifecycleDirective, WindowLifecycleEvent,
    WindowLifecycleEventKind, WindowLifecyclePolicy, WindowPlacementFlushCompletion,
    WindowPlacementSink,
};

use crate::{
    GpuiLogicalRect, GpuiLogicalSize, GpuiWindowKey, GpuiWindowLifecycleError,
    scale_factor_from_gpui,
};

/// One native GPUI window callback, as GPUI delivers it.
///
/// The shape is GPUI's, not Longhorn's. `on_moved` carries no payload, so the
/// adapter's caller reads `Window::bounds` and passes it here. `on_resize`
/// carries the scale factor alongside the size, because GPUI has no separate
/// scale-change callback — the only way to learn that a window crossed onto a
/// display with a different scale is to notice it inside a resize.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuiWindowEvent {
    /// The window's outer bounds changed.
    Moved {
        /// Bounds read from `Window::bounds` after the callback fired.
        bounds: GpuiLogicalRect,
    },
    /// The window's content size or scale changed.
    Resized {
        /// New content size.
        content_size: GpuiLogicalSize,
        /// Scale reported with the resize.
        scale: f32,
    },
    /// The operating system changed the window's active state.
    ActiveStatusChanged {
        /// Whether the window is now active.
        active: bool,
    },
    /// The user asked to close the window and GPUI wants a decision now.
    CloseRequested,
    /// The window has been removed.
    Closed,
}

/// Translates one GPUI callback into Longhorn's vocabulary.
///
/// Returns a sequence, not a single event: a GPUI resize carries a scale, so
/// one callback becomes both `Resized` and `ScaleChanged` when the scale moved.
/// The Tauri translation is one-to-one because Tauri has a dedicated
/// `ScaleFactorChanged` event.
///
/// `previous_scale` is what the caller last recorded for this window. Passing
/// `None` suppresses the `ScaleChanged`, which is correct for the first
/// observation of a window.
pub fn translate_gpui_window_event(
    window_id: &WindowId,
    event: GpuiWindowEvent,
    previous_scale: Option<ScaleFactor>,
) -> Result<Vec<WindowLifecycleEvent>, GpuiWindowLifecycleError> {
    let translated = match event {
        GpuiWindowEvent::Moved { bounds } => vec![WindowLifecycleEvent::Moved {
            window_id: window_id.clone(),
            outer_origin: bounds.to_screen_origin().map_err(translation_error)?,
        }],
        GpuiWindowEvent::Resized {
            content_size,
            scale,
        } => {
            let scale = scale_factor_from_gpui(scale).map_err(translation_error)?;
            let mut events = vec![WindowLifecycleEvent::Resized {
                window_id: window_id.clone(),
                inner_size: content_size.to_screen_size().map_err(translation_error)?,
            }];
            if previous_scale.is_some_and(|previous| previous != scale) {
                events.push(WindowLifecycleEvent::ScaleChanged {
                    window_id: window_id.clone(),
                    scale,
                });
            }
            events
        }
        // GPUI reports focus gain and loss through one callback. Longhorn's
        // vocabulary has only `Blurred`, so focus gain translates to nothing —
        // exactly as it does on Tauri, where `Focused(true)` is dropped.
        GpuiWindowEvent::ActiveStatusChanged { active: false } => {
            vec![WindowLifecycleEvent::Blurred {
                window_id: window_id.clone(),
            }]
        }
        GpuiWindowEvent::ActiveStatusChanged { active: true } => Vec::new(),
        GpuiWindowEvent::CloseRequested => vec![WindowLifecycleEvent::CloseRequested {
            window_id: window_id.clone(),
        }],
        GpuiWindowEvent::Closed => vec![WindowLifecycleEvent::Destroyed {
            window_id: window_id.clone(),
        }],
    };
    Ok(translated)
}

fn translation_error(detail: impl ToString) -> GpuiWindowLifecycleError {
    GpuiWindowLifecycleError::EventTranslation {
        detail: detail.to_string(),
    }
}

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

/// What one shutdown flush did.
///
/// Two halves because they can fail independently: a window that could not be
/// captured is not the same as a store that could not be written, and a caller
/// deciding whether it is safe to exit needs to tell them apart.
#[derive(Debug)]
pub struct GpuiShutdownReceipt {
    per_window: Vec<GpuiLifecycleReceipt>,
    outcome: Option<WindowFlushOutcome>,
}

impl GpuiShutdownReceipt {
    /// One receipt per window that was asked for a capture.
    #[must_use]
    pub fn per_window(&self) -> &[GpuiLifecycleReceipt] {
        &self.per_window
    }

    /// The aggregate write, or `None` when there were no windows to write.
    #[must_use]
    pub const fn outcome(&self) -> Option<&WindowFlushOutcome> {
        self.outcome.as_ref()
    }

    /// Whether everything that was asked for actually happened.
    ///
    /// A shutdown that half-worked is not a shutdown that succeeded, and the
    /// caller is usually about to exit.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.per_window.iter().all(close_is_safe)
            && self
                .outcome
                .as_ref()
                .is_none_or(|outcome| matches!(outcome, WindowFlushOutcome::Succeeded))
    }
}

/// The answer GPUI's `on_should_close` needs, synchronously.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuiCloseDecision {
    /// Longhorn has no outstanding work; GPUI may close the window.
    Close,
    /// Longhorn is not ready; GPUI must keep the window open.
    Defer,
}

impl GpuiCloseDecision {
    /// Returns the boolean GPUI's callback expects.
    #[must_use]
    pub const fn should_close(self) -> bool {
        matches!(self, Self::Close)
    }
}

/// Monotonic clock seam.
pub trait GpuiLifecycleClock {
    /// Returns the current monotonic millisecond reading.
    fn now(&self) -> MonotonicMillis;
}

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

/// Consumer-owned user-close policy callback.
pub trait GpuiUserCloseHandler {
    /// Receives user close without inferred desired-state mutation.
    fn user_close(&mut self, window_id: &WindowId) -> Result<(), String>;
}

/// Explicit no-op user-close policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopGpuiUserCloseHandler;

impl GpuiUserCloseHandler for NoopGpuiUserCloseHandler {
    fn user_close(&mut self, _window_id: &WindowId) -> Result<(), String> {
        Ok(())
    }
}

/// One externally visible thing the host did for a lifecycle input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiLifecycleAction {
    /// No external work ran, with the coordinator's reason.
    Ignored {
        /// Inspectable reason.
        reason: longhorn_windowing::IgnoreReason,
    },
    /// A deadline was accepted by the scheduler.
    Scheduled {
        /// The accepted wake.
        wake: Box<ScheduledWindowLifecycleWake>,
    },
    /// A deadline was refused by the scheduler.
    ScheduleFailed {
        /// Scheduler diagnostic.
        detail: String,
    },
    /// Live state was captured and staged.
    Captured {
        /// The staged capture generation.
        generation: CaptureGeneration,
    },
    /// Capture or staging failed.
    CaptureFailed {
        /// Boundary diagnostic.
        detail: String,
    },
    /// A bounded flush ran to a terminal result.
    Flushed {
        /// The request as issued.
        request: Box<WindowFlushRequest>,
        /// Terminal outcome.
        outcome: WindowFlushOutcome,
    },
    /// User-close policy accepted the close.
    UserCloseReported,
    /// User-close policy refused the close.
    UserCloseFailed {
        /// Policy diagnostic.
        detail: String,
    },
    /// Coordinator state for a destroyed window was released.
    Forgotten,
}

/// Complete result of one lifecycle input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiLifecycleReceipt {
    window_id: WindowId,
    event_kind: WindowLifecycleEventKind,
    actions: Vec<GpuiLifecycleAction>,
}

impl GpuiLifecycleReceipt {
    /// Returns the logical target.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the originating input category.
    #[must_use]
    pub const fn event_kind(&self) -> WindowLifecycleEventKind {
        self.event_kind
    }

    /// Returns every externally visible action, in order.
    #[must_use]
    pub fn actions(&self) -> &[GpuiLifecycleAction] {
        &self.actions
    }
}

/// Injected seams for one GPUI lifecycle host.
///
/// None of these are `Send + Sync`, and every one takes `&mut self`. On a GPUI
/// host every seam runs on the platform main thread inside the application
/// context, so shared ownership would buy nothing and interior mutability
/// would only hide the constraint.
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
    /// Kept for the shutdown flush, which builds its own request rather than
    /// waiting for a directive that may never arrive.
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
    C: GpuiLifecycleClock,
    S: GpuiLifecycleScheduler,
    B: GpuiWindowCaptureBackend,
    U: GpuiUserCloseHandler,
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
        for directive in directives {
            actions.push(self.execute(directive));
        }
        Ok(GpuiLifecycleReceipt {
            window_id,
            event_kind,
            actions,
        })
    }

    /// Captures every installed window and flushes them as one aggregate.
    ///
    /// The counterpart to `TauriWindowLifecycleHost::shutdown_flush`, and it
    /// was missing. Its absence is a measured data loss rather than a symmetry
    /// argument: a window moved and then closed staged its final capture,
    /// permitted the close, and never wrote — the file still held the old
    /// position. Tauri does not lose it, because it prevents every user close
    /// and its shutdown flush gets a later chance; GPUI answers the close
    /// synchronously and the window is gone.
    ///
    /// An application calls this before the last window goes away. Every
    /// receipt is returned in order, and a caller that wants to know whether
    /// anything failed reads them rather than a single boolean — a shutdown
    /// that half-worked is not a shutdown that succeeded.
    ///
    /// One aggregate rather than one flush per window, because the sink
    /// coalesces and a store that can write ten placements in one mutation
    /// should not be asked ten times.
    pub fn shutdown_flush(&mut self) -> GpuiShutdownReceipt {
        let window_ids: Vec<WindowId> = self.windows.keys().cloned().collect();
        if window_ids.is_empty() {
            return GpuiShutdownReceipt {
                per_window: Vec::new(),
                outcome: None,
            };
        }

        // First pass: ask every window for its current capture. This settles
        // anything a close staged and is what `FlushRequested` is for.
        let mut per_window = Vec::with_capacity(window_ids.len());
        for window_id in &window_ids {
            match self.handle_lifecycle_event(WindowLifecycleEvent::FlushRequested {
                window_id: window_id.clone(),
            }) {
                Ok(receipt) => per_window.push(receipt),
                Err(error) => {
                    // Reported, not swallowed, and the loop continues: one
                    // window that cannot capture must not take the others'
                    // placements with it.
                    report_best_effort_failure(
                        "gpui.shutdown_flush",
                        format!("{window_id} could not capture at shutdown: {error}"),
                    );
                }
            }
        }

        // Second pass: one aggregate write. The coordinator schedules a flush
        // rather than emitting one, which is right for ordinary operation and
        // useless on the way out — the deadline it schedules may never arrive.
        // Tauri's shutdown collects the targets and issues the flush itself for
        // the same reason; this does the same thing.
        let request = WindowFlushRequest::new(
            window_ids
                .into_iter()
                .map(|window_id| WindowFlushTarget::new(window_id, None))
                .collect(),
            self.policy.flush_timeout(),
            WindowFlushScope::ApplicationShutdown,
        );
        let outcome = self.flush(&request);

        GpuiShutdownReceipt {
            per_window,
            outcome: Some(outcome),
        }
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

    fn execute(&mut self, directive: WindowLifecycleDirective) -> GpuiLifecycleAction {
        match directive {
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
            WindowLifecycleDirective::CaptureNow {
                window_id,
                generation,
                ..
            } => self.capture_now(&window_id, generation),
            WindowLifecycleDirective::Flush {
                window_id,
                generation,
                timeout,
                reason,
            } => {
                let settled = window_id.clone();
                let request = WindowFlushRequest::new(
                    vec![WindowFlushTarget::new(window_id, generation)],
                    timeout,
                    WindowFlushScope::Window { reason },
                );
                let outcome = self.flush(&request);
                self.settle_flush(&settled);
                GpuiLifecycleAction::Flushed {
                    request: Box::new(request),
                    outcome,
                }
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
                self.coordinator.release(&window_id);
                GpuiLifecycleAction::Forgotten
            }
        }
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

    fn flush(&mut self, request: &WindowFlushRequest) -> WindowFlushOutcome {
        let ticket = match self.services.sink.request_flush(request) {
            Ok(ticket) => ticket,
            Err(detail) => return WindowFlushOutcome::RequestFailed { detail },
        };
        match ticket.wait(request.timeout().as_millis()) {
            Ok(WindowPlacementFlushCompletion::Succeeded) => WindowFlushOutcome::Succeeded,
            Ok(WindowPlacementFlushCompletion::Failed(detail)) => {
                WindowFlushOutcome::SinkFailed { detail }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => WindowFlushOutcome::TimedOut,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                WindowFlushOutcome::Disconnected
            }
        }
    }
}

/// Returns whether one close-request receipt leaves nothing unresolved.
///
/// The Tauri host does not need this: it prevents every user close and lets
/// product policy decide afterwards. GPUI demands an answer inside the
/// callback, so the adapter has to have one, and a refusal from product policy
/// or a flush that did not reach the sink are both reasons to keep the window
/// open.
#[must_use]
pub fn close_is_safe(receipt: &GpuiLifecycleReceipt) -> bool {
    receipt.actions().iter().all(|action| match action {
        GpuiLifecycleAction::CaptureFailed { .. }
        | GpuiLifecycleAction::ScheduleFailed { .. }
        | GpuiLifecycleAction::UserCloseFailed { .. } => false,
        GpuiLifecycleAction::Flushed { outcome, .. } => {
            matches!(outcome, WindowFlushOutcome::Succeeded)
        }
        GpuiLifecycleAction::Ignored { .. }
        | GpuiLifecycleAction::Scheduled { .. }
        | GpuiLifecycleAction::Captured { .. }
        | GpuiLifecycleAction::UserCloseReported
        | GpuiLifecycleAction::Forgotten => true,
    })
}

/// Reports the window host's own outstanding work to the restart interlock.
///
/// Contract 020 requires that "the host reports its own outstanding work to
/// the restart interlock". `QuiescenceProbe` is a plain pure trait in
/// `longhorn-update`, so this needed no host-specific mechanism.
///
/// The count is read at probe time. A receipt taken a second ago is not an
/// answer to "is it safe to restart now".
pub struct GpuiWindowQuiescenceProbe<F> {
    outstanding: F,
}

impl<F> GpuiWindowQuiescenceProbe<F>
where
    F: Fn() -> usize,
{
    /// Records a probe over the host's outstanding capture and flush count.
    pub const fn new(outstanding: F) -> Self {
        Self { outstanding }
    }
}

impl<F> QuiescenceProbe for GpuiWindowQuiescenceProbe<F>
where
    F: Fn() -> usize,
{
    fn outstanding(&self) -> Option<OutstandingWork> {
        let count = (self.outstanding)();
        (count > 0).then(|| OutstandingWork::new(QuiescenceKind::PendingFlush, count))
    }
}

/// Builds a capture from GPUI facts, with no display association.
///
/// GPUI cannot supply the physical bounds, work area or scale that
/// [`longhorn_windowing::CapturedDisplayEvidence`] requires, so a capture
/// taken from GPUI alone associates with no display. A product that needs the
/// association supplies the missing facts itself and constructs the evidence
/// directly.
pub fn capture_from_gpui_facts(
    window_id: &WindowId,
    facts: &crate::GpuiWindowFacts,
) -> Result<CapturedWindowPlacement, String> {
    let state = facts.bounds_state();
    let restore = state.restore_bounds();

    // Longhorn's normal placement wants an *inner* size, and gpui's bounds are
    // outer. For a window that is not maximized the two are both available and
    // `content_size` is the right one; taking the outer extent instead
    // recorded a 560pt window as 592 on macOS, and applying that back grew it
    // by the titlebar every save-and-restore cycle. Measured, not reasoned:
    // the composition example persisted two real windows and the numbers
    // disagreed by exactly the frame.
    //
    // Maximized is the case that has no clean answer. `content_size` then
    // describes the *maximized* window while the restore bounds describe where
    // it will return to, so the restore extent stays the closest honest value
    // and the frame difference is accepted rather than hidden.
    let inner_size = if state.is_maximized() {
        restore.to_screen_size().map_err(|e| e.to_string())?
    } else {
        facts
            .content_size()
            .to_screen_size()
            .map_err(|e| e.to_string())?
    };
    let normal_placement = WindowPlacement::new(
        restore.to_screen_origin().map_err(|e| e.to_string())?,
        inner_size,
    );
    Ok(CapturedWindowPlacement::new(
        window_id.clone(),
        normal_placement,
        state.is_maximized(),
        CapturedDisplayAssociation::Unresolved,
    ))
}
