//! Production isolated-window contract tests over a deterministic owner runtime.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, NativeContentRequestId, PhysicalSize, RoundingMode,
    ScaleFactor, WindowId,
};
use longhorn_native_content::{
    AttachGeneration, AttachmentLifecycle, ContentSizeDecision, DesiredPresence, DesiredState,
    DesiredUpdate, DesiredVisibility, EffectiveFocus, EffectiveVisibility, FocusIntent,
    InputRoutingMode, NativeContentCoordinator, NativeContentFailureCode, NativeContentIslandId,
    NativeContentKindId, ObservationUpdate, ObservedGeometry, ObservedReadiness, OperationOutcome,
};
use longhorn_native_content_isolated_window::{
    HelperCommand, HelperCommandKind, HelperMessage, HelperMessageKind, HelperSnapshot,
    ISOLATED_WINDOW_CAPABILITIES, IsolatedContentRequest, IsolatedContentRequestKind,
    IsolatedWindowAdapter, IsolatedWindowError, IsolatedWindowHelperProtocolVersion,
    IsolatedWindowRuntime, IsolatedWindowRuntimeEvent, IsolatedWindowRuntimeEventKind,
    IsolatedWindowSpec, RuntimeAttachRequest, TeardownOutcome,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Call {
    Attach { handle: u64, generation: u64 },
    Size { handle: u64, size: PhysicalSize },
    Show(u64),
    Hide(u64),
    Focus(u64),
    ReleaseFocus(u64),
    Resizable { handle: u64, value: bool },
    Observe(u64),
    Teardown(u64),
}

struct FakeState {
    next_handle: u64,
    calls: Vec<Call>,
    callback: Option<Arc<dyn Fn(IsolatedWindowRuntimeEvent) + Send + Sync>>,
    snapshot: HelperSnapshot,
    teardown: VecDeque<TeardownOutcome>,
    fail_size: bool,
}

impl Default for FakeState {
    fn default() -> Self {
        Self {
            next_handle: 0,
            calls: Vec::new(),
            callback: None,
            snapshot: snapshot(),
            teardown: VecDeque::from([TeardownOutcome::Completed {
                exit_status: Some(0),
            }]),
            fail_size: false,
        }
    }
}

#[derive(Clone, Default)]
struct FakeRuntime {
    state: Arc<Mutex<FakeState>>,
}

impl FakeRuntime {
    fn calls(&self) -> Vec<Call> {
        self.state.lock().unwrap().calls.clone()
    }

    fn emit(&self, generation: u64, kind: IsolatedWindowRuntimeEventKind) {
        let callback = {
            let mut state = self.state.lock().unwrap();
            match &kind {
                IsolatedWindowRuntimeEventKind::FocusChanged { focused } => {
                    state.snapshot.focused = *focused;
                }
                IsolatedWindowRuntimeEventKind::VisibilityChanged { visible } => {
                    state.snapshot.visible = *visible;
                }
                _ => {}
            }
            state.callback.clone().unwrap()
        };
        callback(runtime_event(generation, kind));
    }

    fn set_teardown(&self, outcomes: impl IntoIterator<Item = TeardownOutcome>) {
        self.state.lock().unwrap().teardown = outcomes.into_iter().collect();
    }
}

impl IsolatedWindowRuntime for FakeRuntime {
    type Handle = u64;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, IsolatedWindowError> {
        let handle = {
            let mut state = self.state.lock().unwrap();
            state.next_handle += 1;
            let handle = state.next_handle;
            state.calls.push(Call::Attach {
                handle,
                generation: request.generation.get(),
            });
            state.callback = Some(request.callback.clone());
            handle
        };
        (request.callback)(IsolatedWindowRuntimeEvent {
            island_id: request.spec.island_id().clone(),
            generation: request.generation,
            kind: IsolatedWindowRuntimeEventKind::Progress {
                phase: "owner_started".into(),
            },
        });
        (request.callback)(IsolatedWindowRuntimeEvent {
            island_id: request.spec.island_id().clone(),
            generation: request.generation,
            kind: IsolatedWindowRuntimeEventKind::Ready {
                snapshot: snapshot(),
                owner_process_id: 42,
                native_content_attached: true,
            },
        });
        Ok(handle)
    }

    fn set_content_size(
        &self,
        handle: &Self::Handle,
        size: PhysicalSize,
        _: Duration,
    ) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Size {
            handle: *handle,
            size,
        });
        if state.fail_size {
            return Err(runtime_error("size"));
        }
        state.snapshot.content_size = size;
        Ok(())
    }

    fn show(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Show(*handle));
        state.snapshot.visible = true;
        Ok(())
    }

    fn hide(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Hide(*handle));
        state.snapshot.visible = false;
        Ok(())
    }

    fn focus(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Focus(*handle));
        state.snapshot.focused = true;
        Ok(())
    }

    fn release_focus(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::ReleaseFocus(*handle));
        state.snapshot.focused = false;
        Ok(())
    }

    fn set_resizable(
        &self,
        handle: &Self::Handle,
        resizable: bool,
        _: Duration,
    ) -> Result<(), IsolatedWindowError> {
        self.state.lock().unwrap().calls.push(Call::Resizable {
            handle: *handle,
            value: resizable,
        });
        Ok(())
    }

    fn observe(
        &self,
        handle: &Self::Handle,
        _: Duration,
    ) -> Result<HelperSnapshot, IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Observe(*handle));
        Ok(state.snapshot)
    }

    fn teardown(
        &self,
        handle: &Self::Handle,
        _: Duration,
    ) -> Result<TeardownOutcome, IsolatedWindowError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Teardown(*handle));
        state
            .teardown
            .pop_front()
            .ok_or_else(|| runtime_error("teardown"))
    }
}

#[test]
fn initial_plan_is_listener_first_and_uses_content_area_only() {
    let runtime = FakeRuntime::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = adapter(runtime.clone(), Arc::clone(&events));
    let authority = coordinator(1);
    let receipt = adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();

    assert!(
        receipt
            .steps()
            .iter()
            .all(|step| step.outcome() == &OperationOutcome::Applied)
    );
    assert_eq!(
        runtime.calls(),
        [
            Call::Attach {
                handle: 1,
                generation: 1
            },
            Call::Size {
                handle: 1,
                size: PhysicalSize::new(720, 440)
            },
            Call::Show(1),
            Call::Focus(1),
        ]
    );
    let events = events.lock().unwrap();
    assert!(matches!(
        events[0],
        longhorn_native_content_isolated_window::IsolatedWindowAdapterEvent::ListenerInstalled { .. }
    ));
    assert!(matches!(
        events[1],
        longhorn_native_content_isolated_window::IsolatedWindowAdapterEvent::AttachStarted { .. }
    ));
}

#[test]
fn resize_requests_are_correlated_non_mutating_decisions_and_echoes_are_suppressed() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let authority = coordinator(1);
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    let revision = authority.desired().revision();

    runtime.emit(
        1,
        request_event(
            "content:resize-1",
            IsolatedContentRequestKind::Resize {
                size: PhysicalSize::new(800, 500),
            },
        ),
    );
    let request = adapter.take_requests(generation(1)).unwrap().pop().unwrap();
    let receipt = adapter
        .decide_resize(
            &authority,
            generation(1),
            &request,
            ContentSizeDecision::Constrained {
                size: ClientSize::new(380.0, 240.0).unwrap(),
            },
        )
        .unwrap();
    assert_eq!(
        receipt.accepted_size(),
        Some(ClientSize::new(380.0, 240.0).unwrap())
    );
    assert_eq!(authority.desired().revision(), revision);
    assert_eq!(
        authority.desired().viewport().size(),
        ClientSize::new(360.0, 220.0).unwrap()
    );

    runtime.emit(
        1,
        request_event(
            "content:echo-1",
            IsolatedContentRequestKind::Resize {
                size: PhysicalSize::new(720, 440),
            },
        ),
    );
    assert!(adapter.take_requests(generation(1)).unwrap().is_empty());
    assert_eq!(
        adapter.admit_runtime_event(runtime_event(
            1,
            request_event("content:resize-1", IsolatedContentRequestKind::Show)
        )),
        Err(IsolatedWindowError::DuplicateCorrelation)
    );
}

#[test]
fn lifecycle_requests_focus_loss_and_observation_remain_explicit() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let authority = coordinator(1);
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();

    for (id, request) in [
        ("content:show", IsolatedContentRequestKind::Show),
        ("content:hide", IsolatedContentRequestKind::Hide),
        ("content:close", IsolatedContentRequestKind::Close),
        (
            "content:hint",
            IsolatedContentRequestKind::ResizeHint { resizable: false },
        ),
    ] {
        runtime.emit(1, request_event(id, request));
    }
    runtime.emit(
        1,
        IsolatedWindowRuntimeEventKind::FocusChanged { focused: false },
    );
    assert_eq!(adapter.take_requests(generation(1)).unwrap().len(), 4);
    adapter.set_resizable(generation(1), false).unwrap();
    let observed = serde_json::to_value(adapter.observe(generation(1)).unwrap()).unwrap();
    assert_eq!(observed["geometry"]["size"]["width"], 720);
    assert_eq!(observed["visibility"], "visible");
    assert_eq!(observed["focus"], "unfocused");
    assert!(runtime.calls().contains(&Call::Resizable {
        handle: 1,
        value: false
    }));
}

#[test]
fn helper_loss_is_terminal_and_stale_generation_cannot_mutate_replacement() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let first = coordinator(1);
    adapter.apply(&first, &first.plan().unwrap()).unwrap();
    runtime.emit(
        1,
        IsolatedWindowRuntimeEventKind::HelperLost {
            code: NativeContentFailureCode::new("isolated:owner-exited").unwrap(),
            exit_status: Some(9),
        },
    );
    let failed = serde_json::to_value(adapter.observe(generation(1)).unwrap()).unwrap();
    assert_eq!(failed["lifecycle"], "failed");

    let second = coordinator(2);
    adapter.apply(&second, &second.plan().unwrap()).unwrap();
    assert_eq!(
        adapter.admit_runtime_event(runtime_event(
            1,
            IsolatedWindowRuntimeEventKind::VisibilityChanged { visible: false }
        )),
        Err(IsolatedWindowError::StaleGeneration {
            current: generation(2),
            supplied: generation(1)
        })
    );
    assert!(adapter.is_attached(generation(2)).unwrap());
}

#[test]
fn bounded_timeout_keeps_owner_for_retry_then_reports_termination() {
    let runtime = FakeRuntime::default();
    runtime.set_teardown([
        TeardownOutcome::TimedOut { timeout_millis: 25 },
        TeardownOutcome::OwnerProcessTerminated {
            exit_status: Some(9),
        },
    ]);
    let adapter = adapter(runtime, Arc::default());
    let initial = coordinator(1);
    adapter.apply(&initial, &initial.plan().unwrap()).unwrap();
    let detached = detached_coordinator(1);
    let plan = detached.plan().unwrap();

    let timeout = adapter.apply(&detached, &plan).unwrap();
    assert!(matches!(
        timeout.steps()[0].outcome(),
        OperationOutcome::Failed { .. }
    ));
    assert!(adapter.is_attached(generation(1)).unwrap());
    let terminated = adapter.apply(&detached, &plan).unwrap();
    assert_eq!(terminated.steps()[0].outcome(), &OperationOutcome::Applied);
    assert_eq!(adapter.teardown_reports().unwrap().len(), 2);
    assert!(!adapter.is_attached(generation(1)).unwrap());
    assert_eq!(
        adapter.admit_runtime_event(runtime_event(
            1,
            IsolatedWindowRuntimeEventKind::VisibilityChanged { visible: false }
        )),
        Err(IsolatedWindowError::GenerationRetired(generation(1)))
    );
}

#[test]
fn runtime_failure_is_an_exact_partial_receipt_and_stale_plan_runs_no_work() {
    let runtime = FakeRuntime::default();
    runtime.state.lock().unwrap().fail_size = true;
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut authority = coordinator(1);
    let stale_plan = authority.plan().unwrap();
    authority
        .update_desired(
            authority.desired().revision(),
            desired_update(1, DesiredPresence::Present),
        )
        .unwrap();
    assert!(matches!(
        adapter.apply(&authority, &stale_plan),
        Err(IsolatedWindowError::Receipt(_))
    ));
    assert!(runtime.calls().is_empty());

    let clean = coordinator(1);
    let receipt = adapter.apply(&clean, &clean.plan().unwrap()).unwrap();
    assert_eq!(receipt.steps()[0].outcome(), &OperationOutcome::Applied);
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

#[test]
fn helper_protocol_round_trips_bounded_correlation_without_outer_position() {
    let command = HelperCommand {
        protocol_version: IsolatedWindowHelperProtocolVersion::CURRENT,
        generation: generation(3),
        request_id: request_id("owner:command-3"),
        command: HelperCommandKind::SetContentSize {
            size: PhysicalSize::new(900, 600),
        },
    };
    let encoded = serde_json::to_string(&command).unwrap();
    assert_eq!(
        serde_json::from_str::<HelperCommand>(&encoded).unwrap(),
        command
    );
    assert!(!encoded.contains("position"));
    assert!(!encoded.contains("pointer"));

    let message = HelperMessage {
        protocol_version: IsolatedWindowHelperProtocolVersion::CURRENT,
        generation: generation(3),
        message: HelperMessageKind::Acknowledged {
            request_id: request_id("owner:command-3"),
            applied: true,
            failure: None,
            snapshot: Some(snapshot()),
        },
    };
    assert_eq!(
        serde_json::from_str::<HelperMessage>(&serde_json::to_string(&message).unwrap()).unwrap(),
        message
    );
    assert!(NativeContentRequestId::new("x".repeat(129)).is_err());
    assert!(
        serde_json::from_str::<HelperCommand>(
            &encoded.replace("\"protocol_version\":1", "\"protocol_version\":2")
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<HelperCommand>(&encoded.replace("{", "{\"unexpected\":true,"))
            .is_err()
    );
}

fn adapter(
    runtime: FakeRuntime,
    events: Arc<Mutex<Vec<longhorn_native_content_isolated_window::IsolatedWindowAdapterEvent>>>,
) -> IsolatedWindowAdapter<FakeRuntime> {
    IsolatedWindowAdapter::new(
        runtime,
        spec(),
        Arc::new(move |event| events.lock().unwrap().push(event)),
    )
}

fn spec() -> IsolatedWindowSpec {
    IsolatedWindowSpec::new(
        island_id(),
        host_window_id(),
        Duration::from_millis(25),
        Duration::from_millis(25),
    )
}

fn coordinator(value: u64) -> NativeContentCoordinator {
    NativeContentCoordinator::new(
        DesiredState::new(
            island_id(),
            NativeContentKindId::new("proof:fake-native-content").unwrap(),
            ISOLATED_WINDOW_CAPABILITIES,
            desired_update(value, DesiredPresence::Present),
        )
        .unwrap(),
    )
}

fn detached_coordinator(value: u64) -> NativeContentCoordinator {
    let mut value = coordinator(value);
    value
        .admit_observation(
            value.observed().revision(),
            ObservationUpdate::new(
                value.desired().generation(),
                AttachmentLifecycle::Attached,
                ObservedReadiness::Ready,
                EffectiveVisibility::Visible,
                EffectiveFocus::Focused,
                ObservedGeometry::IsolatedContent {
                    size: PhysicalSize::new(720, 440),
                },
                Some(InputRoutingMode::NativeDirect),
            ),
        )
        .unwrap();
    value
        .update_desired(
            value.desired().revision(),
            desired_update(value.desired().generation().get(), DesiredPresence::Absent),
        )
        .unwrap();
    value
}

fn desired_update(value: u64, presence: DesiredPresence) -> DesiredUpdate {
    DesiredUpdate::new(
        generation(value),
        host_window_id(),
        ClientRect::new(
            ClientPoint::new(20.0, 30.0).unwrap(),
            ClientSize::new(360.0, 220.0).unwrap(),
        ),
        ScaleFactor::from_thousandths(2000).unwrap(),
        RoundingMode::Nearest,
        presence,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        InputRoutingMode::NativeDirect,
    )
}

fn request_event(id: &str, request: IsolatedContentRequestKind) -> IsolatedWindowRuntimeEventKind {
    IsolatedWindowRuntimeEventKind::ContentRequest {
        request: IsolatedContentRequest {
            request_id: request_id(id),
            request,
        },
    }
}

fn runtime_event(value: u64, kind: IsolatedWindowRuntimeEventKind) -> IsolatedWindowRuntimeEvent {
    IsolatedWindowRuntimeEvent {
        island_id: island_id(),
        generation: generation(value),
        kind,
    }
}

fn snapshot() -> HelperSnapshot {
    HelperSnapshot {
        content_size: PhysicalSize::new(640, 400),
        visible: false,
        focused: false,
    }
}

fn generation(value: u64) -> AttachGeneration {
    AttachGeneration::new(value).unwrap()
}

fn request_id(value: &str) -> NativeContentRequestId {
    NativeContentRequestId::new(value).unwrap()
}

fn island_id() -> NativeContentIslandId {
    NativeContentIslandId::new("island:isolated-proof").unwrap()
}

fn host_window_id() -> WindowId {
    WindowId::new("window:isolated-proof").unwrap()
}

fn runtime_error(operation: &'static str) -> IsolatedWindowError {
    IsolatedWindowError::Runtime {
        operation,
        detail: "injected failure".into(),
    }
}
