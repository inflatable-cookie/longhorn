//! Backing-surface adapter contracts over a deterministic consumer runtime.

use std::sync::{Arc, Mutex};

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode,
    ScaleFactor, WindowId,
};
use longhorn_native_content_backing_surface_prototype::{
    AdapterEvent, BackingSurfaceAdapter, BackingSurfaceError, BackingSurfaceRuntime,
    BackingSurfaceSpec, DetachOutcome, InputAdmission, InputRejection, RuntimeAttachRequest,
    RuntimeEvent, RuntimeEventKind, RuntimeSnapshot,
};
use longhorn_native_content_prototype::{
    AttachGeneration, DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility,
    DetachPolicy, FocusIntent, InputRoutingMode, MechanismCapabilities, NativeContentCoordinator,
    NativeContentIslandId, NativeContentKindId, NativeContentMechanism,
};

#[derive(Clone)]
struct FakeRuntime {
    state: Arc<Mutex<FakeState>>,
}

struct FakeState {
    snapshot: RuntimeSnapshot,
    next_handle: u64,
    attached_handle: Option<u64>,
    detach_count: u64,
    detach_outcome: DetachOutcome,
}

impl Default for FakeRuntime {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState {
                snapshot: snapshot(rect(0, 0, 1_600, 1_000), rect(0, 0, 0, 0), scale(2_000)),
                next_handle: 0,
                attached_handle: None,
                detach_count: 0,
                detach_outcome: DetachOutcome::Detached,
            })),
        }
    }
}

impl FakeRuntime {
    fn current(&self) -> RuntimeSnapshot {
        self.state.lock().unwrap().snapshot.clone()
    }

    fn set_storage(&self, storage: PhysicalRect, native_scale: ScaleFactor) {
        let mut state = self.state.lock().unwrap();
        state.snapshot.storage_bounds = storage;
        state.snapshot.native_scale = native_scale;
    }

    fn detach_count(&self) -> u64 {
        self.state.lock().unwrap().detach_count
    }

    fn mutate(
        &self,
        handle: &u64,
        change: impl FnOnce(&mut RuntimeSnapshot),
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        let mut state = self.state.lock().unwrap();
        if state.attached_handle != Some(*handle) {
            return Err(BackingSurfaceError::Runtime {
                operation: "fake-handle",
                detail: "handle is not attached".to_string(),
            });
        }
        change(&mut state.snapshot);
        state.snapshot.frame_sequence += 1;
        Ok(state.snapshot.clone())
    }
}

impl BackingSurfaceRuntime for FakeRuntime {
    type Handle = u64;

    fn attach(
        &self,
        _request: RuntimeAttachRequest,
    ) -> Result<(Self::Handle, RuntimeSnapshot), BackingSurfaceError> {
        let mut state = self.state.lock().unwrap();
        state.next_handle += 1;
        let handle = state.next_handle;
        state.attached_handle = Some(handle);
        state.snapshot.native_view_attached = true;
        Ok((handle, state.snapshot.clone()))
    }

    fn set_viewport(
        &self,
        handle: &Self::Handle,
        clip: PhysicalRect,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        self.mutate(handle, |snapshot| snapshot.clip = clip)
    }

    fn set_presentation_enabled(
        &self,
        handle: &Self::Handle,
        enabled: bool,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        self.mutate(handle, |snapshot| snapshot.presentation_enabled = enabled)
    }

    fn set_input_routing(
        &self,
        handle: &Self::Handle,
        mode: InputRoutingMode,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        self.mutate(handle, |snapshot| snapshot.input_routing = mode)
    }

    fn refresh(&self, handle: &Self::Handle) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        self.mutate(handle, |_| {})
    }

    fn detach(&self, handle: &Self::Handle) -> Result<DetachOutcome, BackingSurfaceError> {
        let mut state = self.state.lock().unwrap();
        if state.attached_handle != Some(*handle) {
            return Err(BackingSurfaceError::NotAttached);
        }
        state.attached_handle = None;
        state.snapshot.native_view_attached = false;
        state.detach_count += 1;
        Ok(state.detach_outcome)
    }
}

fn snapshot(
    storage_bounds: PhysicalRect,
    clip: PhysicalRect,
    native_scale: ScaleFactor,
) -> RuntimeSnapshot {
    RuntimeSnapshot {
        storage_bounds,
        clip,
        presentation_enabled: false,
        input_routing: InputRoutingMode::Disabled,
        native_scale,
        native_view_attached: false,
        frame_sequence: 0,
    }
}

fn island_id() -> NativeContentIslandId {
    NativeContentIslandId::new("island:backing-proof").unwrap()
}

fn host_window_id() -> WindowId {
    WindowId::new("window:proof-host").unwrap()
}

fn scale(thousandths: u32) -> ScaleFactor {
    ScaleFactor::from_thousandths(thousandths).unwrap()
}

fn viewport(x: f64, y: f64, width: f64, height: f64) -> ClientRect {
    ClientRect::new(
        ClientPoint::new(x, y).unwrap(),
        ClientSize::new(width, height).unwrap(),
    )
}

fn rect(x: i32, y: i32, width: u32, height: u32) -> PhysicalRect {
    PhysicalRect::new(PhysicalPoint::new(x, y), PhysicalSize::new(width, height))
}

fn desired_update(
    generation: u64,
    viewport: ClientRect,
    scale: ScaleFactor,
    visibility: DesiredVisibility,
    route: InputRoutingMode,
) -> DesiredUpdate {
    DesiredUpdate::new(
        AttachGeneration::new(generation),
        host_window_id(),
        viewport,
        scale,
        RoundingMode::Nearest,
        DesiredPresence::Present,
        visibility,
        FocusIntent::Unchanged,
        route,
    )
}

fn coordinator() -> NativeContentCoordinator {
    NativeContentCoordinator::new(DesiredState::new(
        island_id(),
        NativeContentKindId::new("proof:consumer-renderer").unwrap(),
        MechanismCapabilities::new(
            NativeContentMechanism::BackingSurface,
            false,
            DetachPolicy::Reversible,
            false,
            false,
        ),
        desired_update(
            1,
            viewport(120.0, 90.0, 420.0, 280.0),
            scale(2_000),
            DesiredVisibility::Visible,
            InputRoutingMode::RendererForwarded,
        ),
    ))
}

fn adapter(
    runtime: FakeRuntime,
    events: Arc<Mutex<Vec<AdapterEvent>>>,
) -> BackingSurfaceAdapter<FakeRuntime> {
    BackingSurfaceAdapter::new(
        runtime,
        BackingSurfaceSpec::new(island_id(), host_window_id(), DetachPolicy::Reversible),
        Arc::new(move |event| events.lock().unwrap().push(event)),
    )
}

fn attach(
    adapter: &BackingSurfaceAdapter<FakeRuntime>,
    coordinator: &mut NativeContentCoordinator,
) {
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    let observation = adapter.observe(AttachGeneration::new(1)).unwrap();
    coordinator
        .admit_observation(coordinator.observed().revision(), observation)
        .unwrap();
}

#[test]
fn full_host_storage_remains_distinct_from_viewport_clip() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut coordinator = coordinator();

    attach(&adapter, &mut coordinator);

    let current = runtime.current();
    assert_eq!(current.storage_bounds, rect(0, 0, 1_600, 1_000));
    assert_eq!(current.clip, rect(240, 180, 840, 560));
    assert!(current.presentation_enabled);
    assert_eq!(current.input_routing, InputRoutingMode::RendererForwarded);
    assert!(current.native_view_attached);
}

#[test]
fn viewport_move_zero_and_restore_never_move_backing_storage() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut coordinator = coordinator();
    attach(&adapter, &mut coordinator);
    let storage = runtime.current().storage_bounds;

    for target in [
        viewport(180.0, 120.0, 300.0, 220.0),
        viewport(180.0, 120.0, 0.0, 0.0),
        viewport(120.0, 90.0, 420.0, 280.0),
    ] {
        coordinator
            .update_desired(
                coordinator.desired().revision(),
                desired_update(
                    1,
                    target,
                    scale(2_000),
                    DesiredVisibility::Visible,
                    InputRoutingMode::RendererForwarded,
                ),
            )
            .unwrap();
        adapter.apply(&coordinator.plan().unwrap()).unwrap();
        let observation = adapter.observe(AttachGeneration::new(1)).unwrap();
        coordinator
            .admit_observation(coordinator.observed().revision(), observation)
            .unwrap();
        assert_eq!(runtime.current().storage_bounds, storage);
        assert!(runtime.current().native_view_attached);
    }

    assert_eq!(runtime.current().clip, rect(240, 180, 840, 560));
}

#[test]
fn input_gate_never_interprets_consumer_payload() {
    #[derive(Debug, Eq, PartialEq)]
    enum ConsumerAction {
        SelectTrack(u32),
    }

    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime, Arc::default());
    let mut coordinator = coordinator();
    attach(&adapter, &mut coordinator);
    let generation = AttachGeneration::new(1);
    let callbacks = Arc::new(Mutex::new(Vec::new()));

    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(300, 300))
            .unwrap(),
        InputAdmission::Rejected(InputRejection::HostUnfocused)
    );
    adapter.update_host_focus(generation, true).unwrap();
    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(100, 100))
            .unwrap(),
        InputAdmission::Rejected(InputRejection::OutsideViewport)
    );
    if adapter
        .admit_input(generation, PhysicalPoint::new(300, 300))
        .unwrap()
        == InputAdmission::Admitted
    {
        callbacks
            .lock()
            .unwrap()
            .push(ConsumerAction::SelectTrack(17));
    }
    assert_eq!(
        *callbacks.lock().unwrap(),
        vec![ConsumerAction::SelectTrack(17)]
    );

    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                1,
                viewport(120.0, 90.0, 420.0, 280.0),
                scale(2_000),
                DesiredVisibility::Visible,
                InputRoutingMode::Disabled,
            ),
        )
        .unwrap();
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(300, 300))
            .unwrap(),
        InputAdmission::Rejected(InputRejection::RoutingDisabled)
    );
}

#[test]
fn stale_plan_and_runtime_event_leave_exact_state_unchanged() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut coordinator = coordinator();
    attach(&adapter, &mut coordinator);

    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                1,
                viewport(140.0, 100.0, 360.0, 240.0),
                scale(2_000),
                DesiredVisibility::Visible,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap();
    let stale = coordinator.plan().unwrap();
    adapter.apply(&stale).unwrap();
    let observation = adapter.observe(AttachGeneration::new(1)).unwrap();
    coordinator
        .admit_observation(coordinator.observed().revision(), observation)
        .unwrap();
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                1,
                viewport(180.0, 120.0, 300.0, 220.0),
                scale(2_000),
                DesiredVisibility::Visible,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap();
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    let before = runtime.current();

    assert!(matches!(
        adapter.apply(&stale),
        Err(BackingSurfaceError::StalePlan { .. })
    ));
    assert!(matches!(
        adapter.admit_runtime_event(RuntimeEvent {
            island_id: island_id(),
            host_window_id: host_window_id(),
            generation: AttachGeneration::new(0),
            kind: RuntimeEventKind::FramePresented { sequence: 99 },
        }),
        Err(BackingSurfaceError::StaleGeneration { .. })
    ));
    assert_eq!(runtime.current(), before);
}

#[test]
fn fresh_host_geometry_changes_storage_without_rewriting_clip() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut coordinator = coordinator();
    attach(&adapter, &mut coordinator);
    let clip = runtime.current().clip;

    runtime.set_storage(rect(0, 0, 1_920, 1_200), scale(2_000));
    let refreshed = adapter
        .refresh_host_geometry(AttachGeneration::new(1))
        .unwrap();

    assert_eq!(refreshed.storage_bounds, rect(0, 0, 1_920, 1_200));
    assert_eq!(refreshed.clip, clip);
}

#[test]
fn host_destroy_invalidates_before_reversible_detach_and_rejects_late_callback() {
    let runtime = FakeRuntime::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = adapter(runtime.clone(), events.clone());
    let mut coordinator = coordinator();
    attach(&adapter, &mut coordinator);

    let invalidated = adapter.host_destroyed(&host_window_id()).unwrap().unwrap();
    assert_eq!(invalidated.generation(), AttachGeneration::new(1));
    assert_eq!(invalidated.detach_outcome(), DetachOutcome::Detached);
    assert_eq!(runtime.detach_count(), 1);
    assert!(matches!(
        adapter.admit_runtime_event(RuntimeEvent {
            island_id: island_id(),
            host_window_id: host_window_id(),
            generation: AttachGeneration::new(1),
            kind: RuntimeEventKind::FramePresented { sequence: 100 },
        }),
        Err(BackingSurfaceError::NotAttached)
    ));

    let events = events.lock().unwrap();
    let invalidated_index = events
        .iter()
        .position(|event| matches!(event, AdapterEvent::HostInvalidated { .. }))
        .unwrap();
    let detached_index = events
        .iter()
        .position(|event| matches!(event, AdapterEvent::Detached { .. }))
        .unwrap();
    assert!(invalidated_index < detached_index);
}

#[test]
fn viewport_conversion_tracks_explicit_one_and_two_x_scale() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut coordinator = coordinator();
    attach(&adapter, &mut coordinator);

    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                1,
                viewport(120.0, 90.0, 420.0, 280.0),
                scale(1_000),
                DesiredVisibility::Visible,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap();
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    assert_eq!(runtime.current().clip, rect(120, 90, 420, 280));
    let observation = adapter.observe(AttachGeneration::new(1)).unwrap();
    coordinator
        .admit_observation(coordinator.observed().revision(), observation)
        .unwrap();

    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                1,
                viewport(120.0, 90.0, 420.0, 280.0),
                scale(2_000),
                DesiredVisibility::Visible,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap();
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    assert_eq!(runtime.current().clip, rect(240, 180, 840, 560));
}
