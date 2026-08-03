use longhorn_core::{DisplayId, PhysicalRect, ScaleFactor, ScreenRect, WindowId, WindowPlacement};
use longhorn_windowing::{
    ApplyGeneration, CaptureGeneration, CaptureReason, FlushReason, IgnoreReason, MonotonicMillis,
    SavedDisplayAssociation, SavedDisplayEvidence, SavedWindowPlacement, WindowLifecycleDuration,
    WindowLifecycleEvent, WindowLifecycleEventKind, resolve_saved_display_association,
};
use serde::{Deserialize, Serialize};

/// Raw current-monitor evidence without canonical display identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapturedDisplayEvidence {
    machine_label: Option<String>,
    full_bounds: PhysicalRect,
    work_area: PhysicalRect,
    scale: ScaleFactor,
    logical_full_bounds: ScreenRect,
    logical_work_area: ScreenRect,
}

impl CapturedDisplayEvidence {
    /// Constructs current-monitor evidence.
    #[must_use]
    pub const fn new(
        machine_label: Option<String>,
        full_bounds: PhysicalRect,
        work_area: PhysicalRect,
        scale: ScaleFactor,
        logical_full_bounds: ScreenRect,
        logical_work_area: ScreenRect,
    ) -> Self {
        Self {
            machine_label,
            full_bounds,
            work_area,
            scale,
            logical_full_bounds,
            logical_work_area,
        }
    }

    /// Returns the machine-provided label.
    #[must_use]
    pub const fn machine_label(&self) -> Option<&String> {
        self.machine_label.as_ref()
    }

    /// Returns raw physical full bounds.
    #[must_use]
    pub const fn full_bounds(&self) -> PhysicalRect {
        self.full_bounds
    }

    /// Returns raw physical work area.
    #[must_use]
    pub const fn work_area(&self) -> PhysicalRect {
        self.work_area
    }

    /// Returns validated scale evidence.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }

    /// Returns full bounds in the mapper's global logical plane.
    #[must_use]
    pub const fn logical_full_bounds(&self) -> ScreenRect {
        self.logical_full_bounds
    }

    /// Returns usable bounds in the mapper's global logical plane.
    #[must_use]
    pub const fn logical_work_area(&self) -> ScreenRect {
        self.logical_work_area
    }
}

/// Current-monitor observation outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CapturedDisplayAssociation {
    /// Tauri returned current-monitor facts without assigning a `DisplayId`.
    Observed {
        /// Raw correlation evidence for consumer policy.
        evidence: CapturedDisplayEvidence,
    },
    /// Tauri reported no current monitor.
    Unresolved,
}

/// Complete schema-opaque settled placement proposal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapturedWindowPlacement {
    window_id: WindowId,
    normal_placement: WindowPlacement,
    maximized: bool,
    display: CapturedDisplayAssociation,
}

impl CapturedWindowPlacement {
    /// Constructs one complete placement proposal.
    #[must_use]
    pub const fn new(
        window_id: WindowId,
        normal_placement: WindowPlacement,
        maximized: bool,
        display: CapturedDisplayAssociation,
    ) -> Self {
        Self {
            window_id,
            normal_placement,
            maximized,
            display,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns normal outer-origin plus inner-size placement.
    #[must_use]
    pub const fn normal_placement(&self) -> WindowPlacement {
        self.normal_placement
    }

    /// Returns captured maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns observed or explicitly unresolved current-monitor evidence.
    #[must_use]
    pub const fn display(&self) -> &CapturedDisplayAssociation {
        &self.display
    }

    /// Converts this capture into the shared serializable restore record.
    #[must_use]
    pub fn saved(&self, display_id: Option<DisplayId>) -> SavedWindowPlacement {
        let evidence = match &self.display {
            CapturedDisplayAssociation::Observed { evidence } => Some(SavedDisplayEvidence::new(
                evidence.logical_full_bounds(),
                evidence.logical_work_area(),
                evidence.scale(),
            )),
            CapturedDisplayAssociation::Unresolved => None,
        };
        SavedWindowPlacement::new(
            self.window_id.clone(),
            self.normal_placement,
            self.maximized,
            SavedDisplayAssociation::new(display_id, evidence),
        )
    }

    /// Converts this capture using unique exact evidence from an inventory.
    #[must_use]
    pub fn saved_with_inventory(
        &self,
        inventory: &longhorn_display::DisplayInventory,
    ) -> SavedWindowPlacement {
        let unresolved = self.saved(None);
        let display_id = resolve_saved_display_association(unresolved.display(), inventory);
        self.saved(display_id)
    }

    /// Converts this capture using unique exact evidence from a known registry.
    ///
    /// This is useful at persistence seams that retain the registry but not the
    /// ephemeral inventory from the last restore observation.
    #[must_use]
    pub fn saved_with_registry(
        &self,
        registry: &longhorn_display::KnownDisplayRegistry,
    ) -> SavedWindowPlacement {
        let unresolved = self.saved(None);
        let Some(evidence) = unresolved.display().evidence() else {
            return unresolved;
        };
        let mut matches = registry.iter().filter(|display| {
            display.facts().full_bounds() == evidence.full_bounds()
                && display.facts().work_area() == evidence.work_area()
                && display.facts().scale() == evidence.scale()
        });
        let display_id = matches.next().map(|display| display.id().clone());
        if matches.next().is_some() {
            return unresolved;
        }
        self.saved(display_id)
    }
}

/// One host wake requested by the pure lifecycle coordinator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScheduledWindowLifecycleWake {
    due_at: MonotonicMillis,
    event: WindowLifecycleEvent,
}

impl ScheduledWindowLifecycleWake {
    pub(crate) const fn new(due_at: MonotonicMillis, event: WindowLifecycleEvent) -> Self {
        Self { due_at, event }
    }

    /// Returns the caller-clock deadline.
    #[must_use]
    pub const fn due_at(&self) -> MonotonicMillis {
        self.due_at
    }

    /// Returns the event to deliver when the deadline fires.
    #[must_use]
    pub const fn event(&self) -> &WindowLifecycleEvent {
        &self.event
    }
}

/// One window included in a sink flush request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowFlushTarget {
    window_id: WindowId,
    generation: Option<CaptureGeneration>,
}

impl WindowFlushTarget {
    /// Constructs one exact sink flush target.
    #[must_use]
    pub const fn new(window_id: WindowId, generation: Option<CaptureGeneration>) -> Self {
        Self {
            window_id,
            generation,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns captured generation when known.
    #[must_use]
    pub const fn generation(&self) -> Option<CaptureGeneration> {
        self.generation
    }
}

/// Why and how widely the sink should flush.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowFlushScope {
    /// One Card 019 directive.
    Window {
        /// Pure coordinator reason.
        reason: FlushReason,
    },
    /// One bounded application-shutdown aggregate.
    ApplicationShutdown,
}

/// Exact bounded sink flush request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowFlushRequest {
    targets: Vec<WindowFlushTarget>,
    timeout: WindowLifecycleDuration,
    scope: WindowFlushScope,
}

impl WindowFlushRequest {
    /// Constructs one bounded sink request.
    #[must_use]
    pub fn new(
        targets: Vec<WindowFlushTarget>,
        timeout: WindowLifecycleDuration,
        scope: WindowFlushScope,
    ) -> Self {
        Self {
            targets,
            timeout,
            scope,
        }
    }

    /// Returns stable sorted flush targets.
    #[must_use]
    pub fn targets(&self) -> &[WindowFlushTarget] {
        &self.targets
    }

    /// Returns the maximum wait.
    #[must_use]
    pub const fn timeout(&self) -> WindowLifecycleDuration {
        self.timeout
    }

    /// Returns single-window or shutdown scope.
    #[must_use]
    pub const fn scope(&self) -> WindowFlushScope {
        self.scope
    }
}

/// Bounded flush terminal result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowFlushOutcome {
    /// Sink acknowledged success within the bound.
    Succeeded,
    /// Sink refused to start the request.
    RequestFailed {
        /// Sink diagnostic.
        detail: String,
    },
    /// Sink completed with failure.
    SinkFailed {
        /// Sink diagnostic.
        detail: String,
    },
    /// No acknowledgement arrived within the exact bound.
    TimedOut,
    /// Acknowledgement channel closed without a result.
    Disconnected,
}

/// One executed adapter action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TauriWindowLifecycleAction {
    /// Pure coordinator intentionally emitted no work.
    Ignored {
        /// Inspectable classification.
        reason: IgnoreReason,
    },
    /// Host accepted a capture or flush wake.
    Scheduled {
        /// Exact wake.
        wake: ScheduledWindowLifecycleWake,
    },
    /// Host scheduler rejected a wake.
    ScheduleFailed {
        /// Exact rejected wake.
        wake: ScheduledWindowLifecycleWake,
        /// Scheduler diagnostic.
        detail: String,
    },
    /// Complete placement was accepted by the injected sink.
    PlacementStaged {
        /// Capture generation.
        generation: CaptureGeneration,
        /// Capture trigger.
        reason: CaptureReason,
        /// Exact schema-opaque proposal.
        placement: CapturedWindowPlacement,
    },
    /// Complete native capture failed.
    CaptureFailed {
        /// Capture generation.
        generation: CaptureGeneration,
        /// Capture diagnostic.
        detail: String,
    },
    /// Sink rejected captured placement.
    PersistenceFailed {
        /// Capture generation.
        generation: CaptureGeneration,
        /// Sink diagnostic.
        detail: String,
    },
    /// One bounded flush reached a terminal outcome.
    Flushed {
        /// Exact request.
        request: WindowFlushRequest,
        /// Terminal outcome.
        outcome: WindowFlushOutcome,
    },
    /// One bounded flush left the event thread; its terminal outcome arrives
    /// as a later reporter receipt.
    FlushDeferred {
        /// Exact request.
        request: WindowFlushRequest,
    },
    /// Consumer user-close callback completed.
    UserCloseReported,
    /// Consumer user-close callback failed.
    UserCloseFailed {
        /// Consumer diagnostic.
        detail: String,
    },
    /// Destroy removed listener/capture state.
    Forgotten,
}

/// Complete result for one native or scheduled lifecycle input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TauriWindowLifecycleReceipt {
    window_id: WindowId,
    event: WindowLifecycleEventKind,
    actions: Vec<TauriWindowLifecycleAction>,
}

impl TauriWindowLifecycleReceipt {
    pub(crate) const fn new(
        window_id: WindowId,
        event: WindowLifecycleEventKind,
        actions: Vec<TauriWindowLifecycleAction>,
    ) -> Self {
        Self {
            window_id,
            event,
            actions,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns processed input category.
    #[must_use]
    pub const fn event(&self) -> WindowLifecycleEventKind {
        self.event
    }

    /// Returns ordered action outcomes.
    #[must_use]
    pub fn actions(&self) -> &[TauriWindowLifecycleAction] {
        &self.actions
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        WindowId,
        WindowLifecycleEventKind,
        Vec<TauriWindowLifecycleAction>,
    ) {
        (self.window_id, self.event, self.actions)
    }
}

/// Fatal adapter failure before a complete receipt was possible.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TauriWindowLifecycleError {
    /// Teardown has deactivated this host.
    InactiveHost,
    /// Stable id has no installed window.
    UnknownWindow {
        /// Missing identity.
        window_id: WindowId,
    },
    /// Native handle has no installed lifecycle window.
    UnknownWindowHandle {
        /// Missing transport identity.
        transport_handle: longhorn_windowing::HostWindowHandle,
    },
    /// Stable id already has an installed window.
    DuplicateWindow {
        /// Repeated identity.
        window_id: WindowId,
    },
    /// Native label is not a valid host transport handle.
    InvalidWindowLabel {
        /// Validation diagnostic.
        detail: String,
    },
    /// Shared state lock was poisoned.
    StateUnavailable {
        /// State category.
        state: String,
    },
    /// Scheduler could not bind the shared host target.
    SchedulerBinding {
        /// Scheduler diagnostic.
        detail: String,
    },
    /// Native event conversion failed.
    EventTranslation {
        /// Conversion diagnostic.
        detail: String,
    },
    /// Pure lifecycle coordinator rejected arithmetic or generation input.
    Coordination {
        /// Coordinator diagnostic.
        detail: String,
    },
    /// Apply evidence could not be installed before native mutation.
    ApplyRegistration {
        /// Apply generation.
        generation: ApplyGeneration,
        /// Registration diagnostic.
        detail: String,
    },
}

/// Asynchronously reported listener result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowLifecycleReport {
    window_id: WindowId,
    event: Option<WindowLifecycleEventKind>,
    result: Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError>,
}

impl WindowLifecycleReport {
    pub(crate) const fn new(
        window_id: WindowId,
        event: Option<WindowLifecycleEventKind>,
        result: Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError>,
    ) -> Self {
        Self {
            window_id,
            event,
            result,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns translated input category when translation succeeded.
    #[must_use]
    pub const fn event(&self) -> Option<WindowLifecycleEventKind> {
        self.event
    }

    /// Returns the typed listener result.
    pub const fn result(&self) -> &Result<TauriWindowLifecycleReceipt, TauriWindowLifecycleError> {
        &self.result
    }
}

/// Reveal gate state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowRevealStatus {
    /// One or both explicit gates remain false.
    Waiting {
        /// Consumer page-ready signal.
        page_ready: bool,
        /// Successful hidden-placement readback.
        placement_ready: bool,
    },
    /// Native show succeeded after both gates.
    Revealed,
    /// Window was already revealed.
    AlreadyRevealed,
    /// Native show failed.
    Failed {
        /// Host diagnostic.
        detail: String,
    },
}

/// Reveal transition receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowRevealReceipt {
    window_id: WindowId,
    status: WindowRevealStatus,
}

impl WindowRevealReceipt {
    pub(crate) const fn new(window_id: WindowId, status: WindowRevealStatus) -> Self {
        Self { window_id, status }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns gate or native-show result.
    #[must_use]
    pub const fn status(&self) -> &WindowRevealStatus {
        &self.status
    }
}

/// Complete bounded aggregate shutdown result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowShutdownReceipt {
    actions: Vec<TauriWindowLifecycleAction>,
    flush: Option<WindowFlushOutcome>,
}

impl WindowShutdownReceipt {
    pub(crate) const fn new(
        actions: Vec<TauriWindowLifecycleAction>,
        flush: Option<WindowFlushOutcome>,
    ) -> Self {
        Self { actions, flush }
    }

    /// Returns capture and scheduling work performed before aggregate flush.
    #[must_use]
    pub fn actions(&self) -> &[TauriWindowLifecycleAction] {
        &self.actions
    }

    /// Returns aggregate sink outcome when at least one target was flushable.
    #[must_use]
    pub const fn flush(&self) -> Option<&WindowFlushOutcome> {
        self.flush.as_ref()
    }
}
