use std::sync::{Arc, Mutex};

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode,
    ScaleFactor, WindowId,
};
use longhorn_native_content::{
    AttachGeneration, DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility, FocusIntent,
    InputRoutingMode, NativeContentCoordinator, NativeContentIslandId, NativeContentKindId,
};
use longhorn_native_content_backing_surface::{
    BACKING_SURFACE_CAPABILITIES, BackingSurfaceAdapter, BackingSurfaceAdapterEvent,
    BackingSurfaceError, BackingSurfaceRuntime, BackingSurfaceRuntimeEvent,
    BackingSurfaceRuntimeEventKind, BackingSurfaceSnapshot, BackingSurfaceSpec,
    RuntimeAttachRequest,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Call {
    Attach { handle: u64, generation: u64 },
    Clip { handle: u64, clip: PhysicalRect },
    Presentation { handle: u64, enabled: bool },
    Input { handle: u64, mode: InputRoutingMode },
    Observe(u64),
    Detach(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PixelEvidence {
    pub(crate) lit_pixels: u64,
    pub(crate) outside_clip_lit_pixels: u64,
}

struct FakeState {
    next_handle: u64,
    attached_handle: Option<u64>,
    snapshot: BackingSurfaceSnapshot,
    pixels: PixelEvidence,
    calls: Vec<Call>,
    callback: Option<Arc<dyn Fn(BackingSurfaceRuntimeEvent) + Send + Sync>>,
    fail_clip: bool,
    detach_failures: usize,
}

#[derive(Clone)]
pub(crate) struct FakeRuntime {
    state: Arc<Mutex<FakeState>>,
}

impl Default for FakeRuntime {
    fn default() -> Self {
        let snapshot = snapshot(rect(0, 0, 1_600, 1_000), rect(0, 0, 0, 0), scale(2_000));
        Self {
            state: Arc::new(Mutex::new(FakeState {
                next_handle: 0,
                attached_handle: None,
                pixels: render(&snapshot),
                snapshot,
                calls: Vec::new(),
                callback: None,
                fail_clip: false,
                detach_failures: 0,
            })),
        }
    }
}

impl FakeRuntime {
    pub(crate) fn calls(&self) -> Vec<Call> {
        self.state.lock().unwrap().calls.clone()
    }

    pub(crate) fn current(&self) -> BackingSurfaceSnapshot {
        self.state.lock().unwrap().snapshot.clone()
    }

    pub(crate) fn pixels(&self) -> PixelEvidence {
        self.state.lock().unwrap().pixels
    }

    pub(crate) fn fail_next_clip(&self) {
        self.state.lock().unwrap().fail_clip = true;
    }

    pub(crate) fn fail_detach_times(&self, count: usize) {
        self.state.lock().unwrap().detach_failures = count;
    }

    pub(crate) fn set_storage(&self, bounds: PhysicalRect, native_scale: ScaleFactor) {
        let mut state = self.state.lock().unwrap();
        state.snapshot.storage_bounds = bounds;
        state.snapshot.native_scale = native_scale;
        state.pixels = render(&state.snapshot);
    }

    pub(crate) fn set_frame_sequence(&self, sequence: u64) {
        self.state.lock().unwrap().snapshot.frame_sequence = sequence;
    }

    pub(crate) fn emit(
        &self,
        event_sequence: u64,
        generation: u64,
        kind: BackingSurfaceRuntimeEventKind,
    ) {
        let callback = self.state.lock().unwrap().callback.clone().unwrap();
        callback(BackingSurfaceRuntimeEvent {
            island_id: island_id(),
            host_window_id: host_window_id(),
            generation: attach_generation(generation),
            sequence: event_sequence,
            kind,
        });
    }

    fn mutate(
        &self,
        handle: &u64,
        call: Call,
        change: impl FnOnce(&mut BackingSurfaceSnapshot),
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError> {
        let mut state = self.state.lock().unwrap();
        if state.attached_handle != Some(*handle) {
            return Err(runtime_error("handle"));
        }
        state.calls.push(call);
        change(&mut state.snapshot);
        state.snapshot.frame_sequence += 1;
        state.pixels = render(&state.snapshot);
        Ok(state.snapshot.clone())
    }
}

impl BackingSurfaceRuntime for FakeRuntime {
    type Handle = u64;

    fn attach(
        &self,
        request: RuntimeAttachRequest,
    ) -> Result<(Self::Handle, BackingSurfaceSnapshot), BackingSurfaceError> {
        let (handle, snapshot, callback) = {
            let mut state = self.state.lock().unwrap();
            state.next_handle += 1;
            let handle = state.next_handle;
            state.attached_handle = Some(handle);
            state.snapshot.native_storage_attached = true;
            state.snapshot.frame_sequence += 1;
            state.pixels = render(&state.snapshot);
            state.calls.push(Call::Attach {
                handle,
                generation: request.generation.get(),
            });
            state.callback = Some(Arc::clone(&request.callback));
            (handle, state.snapshot.clone(), request.callback)
        };
        callback(BackingSurfaceRuntimeEvent {
            island_id: request.spec.island_id().clone(),
            host_window_id: request.spec.host_window_id().clone(),
            generation: request.generation,
            sequence: 1,
            kind: BackingSurfaceRuntimeEventKind::FramePresented {
                sequence: snapshot.frame_sequence,
            },
        });
        Ok((handle, snapshot))
    }

    fn set_viewport(
        &self,
        handle: &Self::Handle,
        clip: PhysicalRect,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError> {
        {
            let mut state = self.state.lock().unwrap();
            if state.fail_clip {
                state.fail_clip = false;
                state.calls.push(Call::Clip {
                    handle: *handle,
                    clip,
                });
                return Err(runtime_error("clip"));
            }
        }
        self.mutate(
            handle,
            Call::Clip {
                handle: *handle,
                clip,
            },
            |snapshot| snapshot.clip = clip,
        )
    }

    fn set_presentation_enabled(
        &self,
        handle: &Self::Handle,
        enabled: bool,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError> {
        self.mutate(
            handle,
            Call::Presentation {
                handle: *handle,
                enabled,
            },
            |snapshot| snapshot.presentation_enabled = enabled,
        )
    }

    fn set_input_routing(
        &self,
        handle: &Self::Handle,
        mode: InputRoutingMode,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError> {
        self.mutate(
            handle,
            Call::Input {
                handle: *handle,
                mode,
            },
            |snapshot| snapshot.input_routing = mode,
        )
    }

    fn observe(
        &self,
        handle: &Self::Handle,
    ) -> Result<BackingSurfaceSnapshot, BackingSurfaceError> {
        let mut state = self.state.lock().unwrap();
        if state.attached_handle != Some(*handle) {
            return Err(runtime_error("observe"));
        }
        state.calls.push(Call::Observe(*handle));
        Ok(state.snapshot.clone())
    }

    fn detach(&self, handle: &Self::Handle) -> Result<(), BackingSurfaceError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Detach(*handle));
        if state.detach_failures > 0 {
            state.detach_failures -= 1;
            return Err(runtime_error("detach"));
        }
        if state.attached_handle != Some(*handle) {
            return Err(BackingSurfaceError::NotAttached);
        }
        state.attached_handle = None;
        state.snapshot.native_storage_attached = false;
        Ok(())
    }
}

pub(crate) fn adapter(
    runtime: FakeRuntime,
    events: Arc<Mutex<Vec<BackingSurfaceAdapterEvent>>>,
) -> BackingSurfaceAdapter<FakeRuntime> {
    BackingSurfaceAdapter::new(
        runtime,
        BackingSurfaceSpec::new(island_id(), host_window_id()),
        Arc::new(move |event| events.lock().unwrap().push(event)),
    )
}

pub(crate) fn coordinator(generation: u64) -> NativeContentCoordinator {
    NativeContentCoordinator::new(
        DesiredState::new(
            island_id(),
            NativeContentKindId::new("proof:consumer-renderer").unwrap(),
            BACKING_SURFACE_CAPABILITIES,
            desired_update(
                generation,
                viewport(120.0, 90.0, 420.0, 280.0),
                scale(2_000),
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap(),
    )
}

pub(crate) fn desired_update(
    generation: u64,
    viewport: ClientRect,
    scale: ScaleFactor,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    input_routing: InputRoutingMode,
) -> DesiredUpdate {
    DesiredUpdate::new(
        attach_generation(generation),
        host_window_id(),
        viewport,
        scale,
        RoundingMode::Nearest,
        presence,
        visibility,
        FocusIntent::Unchanged,
        input_routing,
    )
}

pub(crate) fn snapshot(
    storage_bounds: PhysicalRect,
    clip: PhysicalRect,
    native_scale: ScaleFactor,
) -> BackingSurfaceSnapshot {
    BackingSurfaceSnapshot {
        storage_bounds,
        clip,
        presentation_enabled: false,
        input_routing: InputRoutingMode::Disabled,
        native_scale,
        native_storage_attached: false,
        frame_sequence: 0,
    }
}

pub(crate) fn render(snapshot: &BackingSurfaceSnapshot) -> PixelEvidence {
    let lit_pixels = if snapshot.presentation_enabled {
        snapshot
            .storage_bounds
            .intersection(&snapshot.clip)
            .map_or(0, |bounds| bounds.area())
    } else {
        0
    };
    PixelEvidence {
        lit_pixels,
        outside_clip_lit_pixels: 0,
    }
}

pub(crate) fn viewport(x: f64, y: f64, width: f64, height: f64) -> ClientRect {
    ClientRect::new(
        ClientPoint::new(x, y).unwrap(),
        ClientSize::new(width, height).unwrap(),
    )
}

pub(crate) fn rect(x: i32, y: i32, width: u32, height: u32) -> PhysicalRect {
    PhysicalRect::new(PhysicalPoint::new(x, y), PhysicalSize::new(width, height))
}

pub(crate) fn scale(value: u32) -> ScaleFactor {
    ScaleFactor::from_thousandths(value).unwrap()
}

pub(crate) fn attach_generation(value: u64) -> AttachGeneration {
    AttachGeneration::new(value).unwrap()
}

pub(crate) fn island_id() -> NativeContentIslandId {
    NativeContentIslandId::new("island:backing-proof").unwrap()
}

pub(crate) fn host_window_id() -> WindowId {
    WindowId::new("window:backing-proof").unwrap()
}

pub(crate) fn runtime_error(operation: &'static str) -> BackingSurfaceError {
    BackingSurfaceError::Runtime {
        operation,
        detail: "injected failure".into(),
    }
}
