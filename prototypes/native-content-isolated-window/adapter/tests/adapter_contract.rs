//! Contract tests for the private isolated native-window adapter.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalSize, RoundingMode, ScaleFactor, WindowId,
};
use longhorn_native_content_isolated_window_prototype::{
    AdapterEvent, ChildRequest, HelperEvent, HelperEventKind, IsolatedWindowAdapter,
    IsolatedWindowError, IsolatedWindowRuntime, IsolatedWindowSpec, RuntimeAttachRequest,
    RuntimeSnapshot, TeardownOutcome,
};
use longhorn_native_content_prototype::{
    AttachGeneration, ContentSizeDecision, DesiredPresence, DesiredState, DesiredUpdate,
    DesiredVisibility, DetachPolicy, FocusIntent, InputRoutingMode, MechanismCapabilities,
    NativeContentFailureCode, NativeContentIslandId, NativeContentKindId, NativeContentMechanism,
    NativeContentRevision, OperationOutcome,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Call {
    Attach(u64),
    Size(PhysicalSize),
    Show,
    Hide,
    Focus,
    ReleaseFocus,
    Resizable(bool),
    Script(ChildRequest),
    Observe,
    Teardown,
}

#[derive(Clone)]
struct FakeHandle {
    generation: AttachGeneration,
}

struct FakeState {
    snapshot: RuntimeSnapshot,
    callback: Option<Arc<dyn Fn(HelperEvent) + Send + Sync>>,
    calls: Vec<Call>,
    teardown: VecDeque<TeardownOutcome>,
    fail_size: bool,
}

#[derive(Clone)]
struct FakeRuntime {
    state: Arc<Mutex<FakeState>>,
}

impl FakeRuntime {
    fn new(teardown: impl IntoIterator<Item = TeardownOutcome>) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState {
                snapshot: RuntimeSnapshot {
                    content_size: PhysicalSize::new(480, 360),
                    visible: false,
                    focused: false,
                },
                callback: None,
                calls: Vec::new(),
                teardown: teardown.into_iter().collect(),
                fail_size: false,
            })),
        }
    }

    fn calls(&self) -> Vec<Call> {
        self.state.lock().unwrap().calls.clone()
    }

    fn emit(&self, generation: u64, kind: HelperEventKind) {
        let callback = self.state.lock().unwrap().callback.clone().unwrap();
        callback(HelperEvent {
            island_id: island_id(),
            generation: AttachGeneration::new(generation),
            kind,
        });
    }

    fn fail_size(&self) {
        self.state.lock().unwrap().fail_size = true;
    }
}

impl IsolatedWindowRuntime for FakeRuntime {
    type Handle = FakeHandle;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, IsolatedWindowError> {
        {
            let mut state = self.state.lock().unwrap();
            state.calls.push(Call::Attach(request.generation.get()));
            state.callback = Some(request.callback.clone());
        }
        (request.callback)(HelperEvent {
            island_id: request.island_id,
            generation: request.generation,
            kind: HelperEventKind::Ready {
                content_size: PhysicalSize::new(480, 360),
                process_id: 42,
                native_child_attached: true,
            },
        });
        Ok(FakeHandle {
            generation: request.generation,
        })
    }

    fn set_content_size(
        &self,
        handle: &Self::Handle,
        size: PhysicalSize,
    ) -> Result<(), IsolatedWindowError> {
        assert_eq!(handle.generation.get(), 1);
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Size(size));
        if state.fail_size {
            return Err(runtime_error("size", "scripted size failure"));
        }
        state.snapshot.content_size = size;
        Ok(())
    }

    fn show(&self, _handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Show);
        state.snapshot.visible = true;
        Ok(())
    }

    fn hide(&self, _handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Hide);
        state.snapshot.visible = false;
        Ok(())
    }

    fn focus(&self, _handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Focus);
        state.snapshot.focused = true;
        Ok(())
    }

    fn release_focus(&self, _handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::ReleaseFocus);
        state.snapshot.focused = false;
        Ok(())
    }

    fn set_resizable(
        &self,
        _handle: &Self::Handle,
        resizable: bool,
    ) -> Result<(), IsolatedWindowError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(Call::Resizable(resizable));
        Ok(())
    }

    fn script_request(
        &self,
        handle: &Self::Handle,
        request: ChildRequest,
    ) -> Result<(), IsolatedWindowError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(Call::Script(request.clone()));
        self.emit(
            handle.generation.get(),
            HelperEventKind::ChildRequest { request },
        );
        Ok(())
    }

    fn simulate_helper_loss(
        &self,
        handle: &Self::Handle,
    ) -> Result<Option<i32>, IsolatedWindowError> {
        self.emit(
            handle.generation.get(),
            HelperEventKind::HelperLost {
                code: NativeContentFailureCode::new("isolated:helper-exited").unwrap(),
                exit_status: Some(73),
            },
        );
        Ok(Some(73))
    }

    fn observe(&self, _handle: &Self::Handle) -> Result<RuntimeSnapshot, IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Observe);
        Ok(state.snapshot)
    }

    fn teardown(
        &self,
        _handle: &Self::Handle,
        _timeout: Duration,
    ) -> Result<TeardownOutcome, IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Teardown);
        state
            .teardown
            .pop_front()
            .ok_or_else(|| runtime_error("teardown", "missing scripted outcome"))
    }
}

#[test]
fn installs_listener_before_attach_and_reads_real_runtime_state() {
    let runtime = FakeRuntime::new([TeardownOutcome::Completed {
        exit_status: Some(0),
    }]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = adapter(runtime.clone(), events.clone());
    let mut coordinator = coordinator(1, 240.0, 180.0, 2_000);
    let receipt = adapter.apply(&coordinator.plan().unwrap()).unwrap();
    assert!(
        receipt
            .steps()
            .iter()
            .all(|step| matches!(step.outcome(), OperationOutcome::Applied))
    );
    let observation = adapter.observe(AttachGeneration::new(1)).unwrap();
    let value = serde_json::to_value(&observation).unwrap();
    assert_eq!(value["geometry"]["kind"], "isolated_content");
    assert_eq!(value["geometry"]["size"]["width"], 480);
    assert_eq!(value["visibility"], "visible");
    assert_eq!(value["focus"], "focused");
    coordinator
        .admit_observation(coordinator.observed().revision(), observation)
        .unwrap();
    let events = events.lock().unwrap();
    assert!(matches!(events[0], AdapterEvent::ListenerInstalled { .. }));
    assert!(matches!(events[1], AdapterEvent::AttachStarted { .. }));
    assert!(events.iter().any(|event| matches!(
        event,
        AdapterEvent::Runtime {
            event: HelperEventKind::Ready { .. },
            ..
        }
    )));
    assert!(runtime.calls().starts_with(&[
        Call::Attach(1),
        Call::Size(PhysicalSize::new(480, 360)),
        Call::Show,
    ]));
}

#[test]
fn child_resize_is_a_current_consumer_decision_and_echoes_are_suppressed() {
    let runtime = FakeRuntime::new([]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = adapter(runtime.clone(), events.clone());
    let coordinator = coordinator(1, 240.0, 180.0, 2_000);
    adapter.apply(&coordinator.plan().unwrap()).unwrap();

    adapter
        .script_request(
            AttachGeneration::new(1),
            ChildRequest::Resize {
                size: PhysicalSize::new(480, 360),
            },
        )
        .unwrap();
    assert!(
        adapter
            .take_requests(AttachGeneration::new(1))
            .unwrap()
            .is_empty()
    );
    assert!(
        events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, AdapterEvent::ResizeCycleSuppressed { .. }))
    );

    adapter
        .script_request(
            AttachGeneration::new(1),
            ChildRequest::Resize {
                size: PhysicalSize::new(1_600, 1_200),
            },
        )
        .unwrap();
    let requests = adapter.take_requests(AttachGeneration::new(1)).unwrap();
    assert_eq!(requests.len(), 1);
    let receipt = adapter
        .decide_resize(
            coordinator.desired(),
            PhysicalSize::new(1_600, 1_200),
            ContentSizeDecision::Constrained {
                size: ClientSize::new(640.0, 480.0).unwrap(),
            },
        )
        .unwrap();
    assert_eq!(
        receipt.accepted_size(),
        Some(ClientSize::new(640.0, 480.0).unwrap())
    );
    assert_eq!(
        coordinator.desired().revision(),
        NativeContentRevision::INITIAL
    );
}

#[test]
fn show_hide_focus_loss_and_resize_hint_remain_explicit_operations() {
    let runtime = FakeRuntime::new([]);
    let adapter = adapter(runtime.clone(), Arc::new(Mutex::new(Vec::new())));
    let mut coordinator = coordinator(1, 240.0, 180.0, 2_000);
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    let attached = adapter.observe(AttachGeneration::new(1)).unwrap();
    coordinator
        .admit_observation(coordinator.observed().revision(), attached)
        .unwrap();

    adapter
        .set_resizable(AttachGeneration::new(1), false)
        .unwrap();
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                1,
                240.0,
                180.0,
                2_000,
                DesiredPresence::Present,
                DesiredVisibility::Hidden {
                    reason: longhorn_native_content_prototype::VisibilityReasonId::new(
                        "proof:child-hide",
                    )
                    .unwrap(),
                },
                FocusIntent::ReleaseIfOwned,
            ),
        )
        .unwrap();
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    assert!(runtime.calls().contains(&Call::Resizable(false)));
    assert!(runtime.calls().contains(&Call::Hide));
    assert!(runtime.calls().contains(&Call::ReleaseFocus));
}

#[test]
fn helper_loss_is_terminal_and_stale_reports_cannot_mutate_it() {
    let runtime = FakeRuntime::new([]);
    let adapter = adapter(runtime.clone(), Arc::new(Mutex::new(Vec::new())));
    let coordinator = coordinator(1, 240.0, 180.0, 2_000);
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    runtime.emit(
        1,
        HelperEventKind::HelperLost {
            code: NativeContentFailureCode::new("isolated:helper-exited").unwrap(),
            exit_status: Some(73),
        },
    );
    let failed = serde_json::to_value(adapter.observe(AttachGeneration::new(1)).unwrap()).unwrap();
    assert_eq!(failed["lifecycle"], "failed");
    assert_eq!(
        adapter.apply(&coordinator.plan().unwrap()).unwrap_err(),
        IsolatedWindowError::FailedGeneration
    );
    assert!(matches!(
        adapter.admit_runtime_event(HelperEvent {
            island_id: island_id(),
            generation: AttachGeneration::new(0),
            kind: HelperEventKind::FocusChanged { focused: true },
        }),
        Err(IsolatedWindowError::StaleGeneration { .. })
    ));
}

#[test]
fn bounded_teardown_reports_timeout_then_owner_termination() {
    let runtime = FakeRuntime::new([
        TeardownOutcome::TimedOut { timeout_millis: 25 },
        TeardownOutcome::OwnerProcessTerminated {
            exit_status: Some(9),
        },
    ]);
    let adapter = adapter(runtime, Arc::new(Mutex::new(Vec::new())));
    let mut coordinator = coordinator(1, 240.0, 180.0, 2_000);
    adapter.apply(&coordinator.plan().unwrap()).unwrap();
    let attached = adapter.observe(AttachGeneration::new(1)).unwrap();
    coordinator
        .admit_observation(coordinator.observed().revision(), attached)
        .unwrap();
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                1,
                240.0,
                180.0,
                2_000,
                DesiredPresence::Absent,
                DesiredVisibility::Visible,
                FocusIntent::Unchanged,
            ),
        )
        .unwrap();
    let plan = coordinator.plan().unwrap();
    let first = adapter.apply(&plan).unwrap();
    assert!(matches!(
        first.steps()[0].outcome(),
        OperationOutcome::Failed { .. }
    ));
    let second = adapter.apply(&plan).unwrap();
    assert!(matches!(
        second.steps()[0].outcome(),
        OperationOutcome::Applied
    ));
    assert_eq!(
        adapter.teardown_reports().unwrap(),
        vec![
            (
                AttachGeneration::new(1),
                TeardownOutcome::TimedOut { timeout_millis: 25 }
            ),
            (
                AttachGeneration::new(1),
                TeardownOutcome::OwnerProcessTerminated {
                    exit_status: Some(9)
                }
            )
        ]
    );
}

#[test]
fn failed_size_produces_exact_partial_receipt() {
    let runtime = FakeRuntime::new([]);
    runtime.fail_size();
    let adapter = adapter(runtime, Arc::new(Mutex::new(Vec::new())));
    let coordinator = coordinator(1, 240.0, 180.0, 2_000);
    let receipt = adapter.apply(&coordinator.plan().unwrap()).unwrap();
    assert!(matches!(
        receipt.steps()[0].outcome(),
        OperationOutcome::Applied
    ));
    assert!(matches!(
        receipt.steps()[1].outcome(),
        OperationOutcome::Failed { .. }
    ));
    assert!(
        receipt.steps()[2..]
            .iter()
            .all(|step| matches!(step.outcome(), OperationOutcome::DependencySkipped { .. }))
    );
}

fn adapter(
    runtime: FakeRuntime,
    events: Arc<Mutex<Vec<AdapterEvent>>>,
) -> IsolatedWindowAdapter<FakeRuntime> {
    IsolatedWindowAdapter::new(
        runtime,
        IsolatedWindowSpec::new(island_id(), host_window_id(), Duration::from_millis(25)),
        Arc::new(move |event| events.lock().unwrap().push(event)),
    )
}

fn coordinator(
    generation: u64,
    width: f64,
    height: f64,
    scale: u32,
) -> longhorn_native_content_prototype::NativeContentCoordinator {
    longhorn_native_content_prototype::NativeContentCoordinator::new(DesiredState::new(
        island_id(),
        NativeContentKindId::new("proof:fake-native-child").unwrap(),
        MechanismCapabilities::new(
            NativeContentMechanism::IsolatedWindow,
            true,
            DetachPolicy::OwnerProcessTermination,
            true,
            true,
        ),
        desired_update(
            generation,
            width,
            height,
            scale,
            DesiredPresence::Present,
            DesiredVisibility::Visible,
            FocusIntent::Request,
        ),
    ))
}

#[allow(clippy::too_many_arguments)]
fn desired_update(
    generation: u64,
    width: f64,
    height: f64,
    scale: u32,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    focus: FocusIntent,
) -> DesiredUpdate {
    DesiredUpdate::new(
        AttachGeneration::new(generation),
        host_window_id(),
        ClientRect::new(
            ClientPoint::new(0.0, 0.0).unwrap(),
            ClientSize::new(width, height).unwrap(),
        ),
        ScaleFactor::from_thousandths(scale).unwrap(),
        RoundingMode::Nearest,
        presence,
        visibility,
        focus,
        InputRoutingMode::NativeDirect,
    )
}

fn island_id() -> NativeContentIslandId {
    NativeContentIslandId::new("proof-isolated-window").unwrap()
}

fn host_window_id() -> WindowId {
    WindowId::new("proof-isolated-host").unwrap()
}

fn runtime_error(operation: &'static str, detail: &str) -> IsolatedWindowError {
    IsolatedWindowError::Runtime {
        operation,
        detail: detail.to_string(),
    }
}
