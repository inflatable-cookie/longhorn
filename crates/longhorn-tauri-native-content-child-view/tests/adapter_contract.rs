//! Production child-view adapter contract tests over a deterministic runtime.

use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode,
    ScaleFactor, WindowId,
};
use longhorn_native_content::{
    ApplyPlan, AttachGeneration, AttachmentLifecycle, DesiredPresence, DesiredState, DesiredUpdate,
    DesiredVisibility, EffectiveFocus, EffectiveVisibility, FocusIntent, InputRoutingMode,
    NativeContentCoordinator, NativeContentIslandId, NativeContentKindId, ObservationUpdate,
    ObservedGeometry, ObservedReadiness, OperationOutcome, VisibilityReasonId,
};
use longhorn_tauri_native_content_child_view::{
    CHILD_VIEW_CAPABILITIES, ChildViewAdapter, ChildViewAdapterEvent, ChildViewError,
    ChildViewHostDestroyOutcome, ChildViewLabel, ChildViewNavigationOutcome, ChildViewPolicyHooks,
    ChildViewRuntime, ChildViewRuntimeEvent, ChildViewRuntimeEventKind, ChildViewSpec,
    ChildViewTeardownOutcome, RuntimeAttachRequest,
};
use tauri::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
enum NativeCall {
    Attach { handle: u64, generation: u64 },
    Bounds { handle: u64, bounds: PhysicalRect },
    Show { handle: u64 },
    Hide { handle: u64 },
    Focus { handle: u64 },
    CurrentUrl { handle: u64 },
    Navigate { handle: u64, url: String },
    Close { handle: u64 },
}

#[derive(Default)]
struct FakeState {
    next_handle: u64,
    calls: Vec<NativeCall>,
    bounds: BTreeMap<u64, PhysicalRect>,
    urls: BTreeMap<u64, Url>,
    callbacks: BTreeMap<u64, Arc<dyn Fn(ChildViewRuntimeEvent) + Send + Sync>>,
    fail_bounds: bool,
    fail_current_url: bool,
    fail_navigate: bool,
    fail_close_once: bool,
}

#[derive(Clone, Default)]
struct FakeRuntime {
    state: Arc<Mutex<FakeState>>,
    timeline: Arc<Mutex<Vec<String>>>,
}

impl FakeRuntime {
    fn with_timeline(timeline: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            state: Arc::default(),
            timeline,
        }
    }

    fn calls(&self) -> Vec<NativeCall> {
        self.state.lock().unwrap().calls.clone()
    }

    fn set_fail_bounds(&self, fail: bool) {
        self.state.lock().unwrap().fail_bounds = fail;
    }

    fn set_fail_close_once(&self) {
        self.state.lock().unwrap().fail_close_once = true;
    }

    fn set_fail_current_url(&self, fail: bool) {
        self.state.lock().unwrap().fail_current_url = fail;
    }

    fn set_fail_navigate(&self, fail: bool) {
        self.state.lock().unwrap().fail_navigate = fail;
    }

    fn emit(&self, handle: u64, generation: u64, kind: ChildViewRuntimeEventKind) {
        let callback = self
            .state
            .lock()
            .unwrap()
            .callbacks
            .get(&handle)
            .unwrap()
            .clone();
        callback(runtime_event(generation, kind));
    }
}

impl ChildViewRuntime for FakeRuntime {
    type Handle = u64;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, ChildViewError> {
        let source = request.spec.source().clone();
        let handle = {
            let mut state = self.state.lock().unwrap();
            state.next_handle += 1;
            let handle = state.next_handle;
            state.calls.push(NativeCall::Attach {
                handle,
                generation: request.generation.get(),
            });
            state.urls.insert(handle, source);
            state.callbacks.insert(handle, request.callback.clone());
            handle
        };
        self.timeline.lock().unwrap().push("runtime:attach".into());
        (request.callback)(ChildViewRuntimeEvent {
            island_id: request.spec.island_id().clone(),
            generation: request.generation,
            child_label: request.spec.child_label().clone(),
            kind: ChildViewRuntimeEventKind::PageLoadStarted,
        });
        Ok(handle)
    }

    fn set_bounds(
        &self,
        handle: &Self::Handle,
        bounds: PhysicalRect,
    ) -> Result<(), ChildViewError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(NativeCall::Bounds {
            handle: *handle,
            bounds,
        });
        if state.fail_bounds {
            return Err(native_error("bounds"));
        }
        state.bounds.insert(*handle, bounds);
        Ok(())
    }

    fn show(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(NativeCall::Show { handle: *handle });
        Ok(())
    }

    fn hide(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(NativeCall::Hide { handle: *handle });
        Ok(())
    }

    fn focus(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(NativeCall::Focus { handle: *handle });
        Ok(())
    }

    fn current_url(&self, handle: &Self::Handle) -> Result<Url, ChildViewError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(NativeCall::CurrentUrl { handle: *handle });
        if state.fail_current_url {
            return Err(native_error("current-url"));
        }
        state
            .urls
            .get(handle)
            .cloned()
            .ok_or_else(|| native_error("current-url"))
    }

    fn navigate(&self, handle: &Self::Handle, url: Url) -> Result<(), ChildViewError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(NativeCall::Navigate {
            handle: *handle,
            url: url.to_string(),
        });
        if state.fail_navigate {
            return Err(native_error("navigate"));
        }
        state.urls.insert(*handle, url);
        Ok(())
    }

    fn close(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(NativeCall::Close { handle: *handle });
        if state.fail_close_once {
            state.fail_close_once = false;
            return Err(native_error("close"));
        }
        Ok(())
    }

    fn bounds(&self, handle: &Self::Handle) -> Result<PhysicalRect, ChildViewError> {
        self.state
            .lock()
            .unwrap()
            .bounds
            .get(handle)
            .copied()
            .ok_or_else(|| native_error("observe"))
    }
}

fn native_error(operation: &'static str) -> ChildViewError {
    ChildViewError::Native {
        operation,
        detail: "injected failure".into(),
    }
}

fn generation(value: u64) -> AttachGeneration {
    AttachGeneration::new(value).unwrap()
}

fn island_id() -> NativeContentIslandId {
    NativeContentIslandId::new("island:child-proof").unwrap()
}

fn host_window_id() -> WindowId {
    WindowId::new("window:proof-host").unwrap()
}

fn viewport() -> ClientRect {
    ClientRect::new(
        ClientPoint::new(12.0, 18.0).unwrap(),
        ClientSize::new(360.0, 220.0).unwrap(),
    )
}

fn physical_viewport() -> PhysicalRect {
    PhysicalRect::new(PhysicalPoint::new(12, 18), PhysicalSize::new(360, 220))
}

fn desired_update(
    generation: u64,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
) -> DesiredUpdate {
    DesiredUpdate::new(
        self::generation(generation),
        host_window_id(),
        viewport(),
        ScaleFactor::from_thousandths(1000).unwrap(),
        RoundingMode::Nearest,
        presence,
        visibility,
        FocusIntent::Request,
        InputRoutingMode::NativeDirect,
    )
}

fn coordinator(generation: u64) -> NativeContentCoordinator {
    NativeContentCoordinator::new(
        DesiredState::new(
            island_id(),
            NativeContentKindId::new("proof:controlled-page").unwrap(),
            CHILD_VIEW_CAPABILITIES,
            desired_update(
                generation,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
            ),
        )
        .unwrap(),
    )
}

fn attached_coordinator(generation: u64) -> NativeContentCoordinator {
    let mut coordinator = coordinator(generation);
    coordinator
        .admit_observation(
            coordinator.observed().revision(),
            attached_observation(generation),
        )
        .unwrap();
    coordinator
}

fn attached_observation(generation: u64) -> ObservationUpdate {
    ObservationUpdate::new(
        self::generation(generation),
        AttachmentLifecycle::Attached,
        ObservedReadiness::Ready,
        EffectiveVisibility::Unknown,
        EffectiveFocus::Unknown,
        ObservedGeometry::ChildBounds {
            bounds: physical_viewport(),
        },
        Some(InputRoutingMode::NativeDirect),
    )
}

fn control_plan(generation: u64, visible: bool) -> (NativeContentCoordinator, ApplyPlan) {
    let mut coordinator = attached_coordinator(generation);
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                generation,
                DesiredPresence::Present,
                if visible {
                    DesiredVisibility::Visible
                } else {
                    DesiredVisibility::Hidden {
                        reason: VisibilityReasonId::new("proof:inactive").unwrap(),
                    }
                },
            ),
        )
        .unwrap();
    let plan = coordinator.plan().unwrap();
    (coordinator, plan)
}

fn spec() -> ChildViewSpec {
    let source = Url::parse("http://127.0.0.1:43119/proof").unwrap();
    let origin = source.origin().ascii_serialization();
    ChildViewSpec::new(
        island_id(),
        host_window_id(),
        ChildViewLabel::new("host").unwrap(),
        ChildViewLabel::new("proof-child").unwrap(),
        source,
        Some(*b"longhorn-proof-1"),
        Arc::new(move |candidate| candidate.origin().ascii_serialization() == origin),
        ChildViewPolicyHooks::new(None, Arc::new(|_| {})).unwrap(),
    )
    .unwrap()
}

fn runtime_event(generation: u64, kind: ChildViewRuntimeEventKind) -> ChildViewRuntimeEvent {
    ChildViewRuntimeEvent {
        island_id: island_id(),
        generation: self::generation(generation),
        child_label: ChildViewLabel::new("proof-child").unwrap(),
        kind,
    }
}

fn adapter(
    runtime: FakeRuntime,
    timeline: Arc<Mutex<Vec<String>>>,
) -> ChildViewAdapter<FakeRuntime> {
    ChildViewAdapter::new(
        runtime,
        spec(),
        Arc::new(move |event| {
            let label = match event {
                ChildViewAdapterEvent::ListenerInstalled { .. } => "adapter:listener",
                ChildViewAdapterEvent::AttachStarted { .. } => "adapter:attach-started",
                ChildViewAdapterEvent::Attached { .. } => "adapter:attached",
                ChildViewAdapterEvent::Runtime { .. } => "adapter:runtime",
                ChildViewAdapterEvent::RendererUnmounted { .. } => "adapter:renderer-unmounted",
                ChildViewAdapterEvent::DetachStarted { .. } => "adapter:detach-started",
                ChildViewAdapterEvent::Detached { .. } => "adapter:detached",
                ChildViewAdapterEvent::HostInvalidated { .. } => "adapter:host-invalidated",
            };
            timeline.lock().unwrap().push(label.into());
        }),
    )
}

#[test]
fn initial_plan_installs_callbacks_before_attach_and_applies_full_sequence() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline.clone());
    let coordinator = coordinator(1);
    let plan = coordinator.plan().unwrap();

    let receipt = adapter.apply(&coordinator, &plan).unwrap();

    assert_eq!(receipt.steps().len(), 5);
    assert!(
        receipt
            .steps()
            .iter()
            .all(|step| step.outcome() == &OperationOutcome::Applied)
    );
    assert_eq!(
        runtime.calls(),
        vec![
            NativeCall::Attach {
                handle: 1,
                generation: 1
            },
            NativeCall::Bounds {
                handle: 1,
                bounds: physical_viewport()
            },
            NativeCall::Show { handle: 1 },
            NativeCall::Focus { handle: 1 },
        ]
    );
    assert_eq!(
        *timeline.lock().unwrap(),
        [
            "adapter:listener",
            "adapter:attach-started",
            "runtime:attach",
            "adapter:runtime",
            "adapter:attached",
        ]
    );
}

#[test]
fn observation_reads_native_bounds_and_keeps_visibility_and_focus_unknown() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline);
    let coordinator = coordinator(1);
    let plan = coordinator.plan().unwrap();
    adapter.apply(&coordinator, &plan).unwrap();
    runtime.emit(1, 1, ChildViewRuntimeEventKind::PageLoadFinished);

    let value = serde_json::to_value(adapter.observe(generation(1)).unwrap()).unwrap();
    assert_eq!(value["lifecycle"], "attached");
    assert_eq!(value["readiness"], "ready");
    assert_eq!(value["visibility"], "unknown");
    assert_eq!(value["focus"], "unknown");
    assert_eq!(value["geometry"]["bounds"]["origin"]["x"], 12);
    assert_eq!(value["geometry"]["bounds"]["size"]["width"], 360);
}

#[test]
fn renderer_unmount_and_hide_show_reuse_one_native_handle() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline);
    let initial = coordinator(1);
    adapter.apply(&initial, &initial.plan().unwrap()).unwrap();

    let (hidden, hidden_plan) = control_plan(1, false);
    adapter.apply(&hidden, &hidden_plan).unwrap();
    adapter.renderer_unmounted(generation(1)).unwrap();
    let (visible, visible_plan) = control_plan(1, true);
    adapter.apply(&visible, &visible_plan).unwrap();

    let calls = runtime.calls();
    assert!(adapter.is_attached(generation(1)).unwrap());
    assert!(calls.contains(&NativeCall::Hide { handle: 1 }));
    assert!(calls.contains(&NativeCall::Show { handle: 1 }));
    assert_eq!(
        calls
            .iter()
            .filter(|call| matches!(call, NativeCall::Attach { .. }))
            .count(),
        1
    );
    assert!(
        !calls
            .iter()
            .any(|call| matches!(call, NativeCall::Close { .. }))
    );
}

#[test]
fn admitted_navigation_reuses_one_generation_and_same_url_is_unchanged() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline);
    let authority = coordinator(1);
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    runtime.emit(1, 1, ChildViewRuntimeEventKind::PageLoadFinished);

    let requested = Url::parse("http://127.0.0.1:43119/next").unwrap();
    let submitted = adapter.navigate(generation(1), requested.clone()).unwrap();
    assert_eq!(submitted.generation(), generation(1));
    assert_eq!(submitted.previous_url(), spec().source());
    assert_eq!(submitted.requested_url(), &requested);
    assert_eq!(submitted.outcome(), ChildViewNavigationOutcome::Submitted);

    runtime.emit(1, 1, ChildViewRuntimeEventKind::PageLoadStarted);
    assert_eq!(
        serde_json::to_value(adapter.observe(generation(1)).unwrap()).unwrap()["readiness"],
        "not_ready"
    );
    runtime.emit(1, 1, ChildViewRuntimeEventKind::PageLoadFinished);
    assert_eq!(
        serde_json::to_value(adapter.observe(generation(1)).unwrap()).unwrap()["readiness"],
        "ready"
    );

    let unchanged = adapter.navigate(generation(1), requested.clone()).unwrap();
    assert_eq!(unchanged.previous_url(), &requested);
    assert_eq!(unchanged.outcome(), ChildViewNavigationOutcome::Unchanged);
    assert_eq!(adapter.current_url(generation(1)).unwrap(), requested);

    let calls = runtime.calls();
    assert_eq!(
        calls
            .iter()
            .filter(|call| matches!(call, NativeCall::Navigate { .. }))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| matches!(call, NativeCall::Attach { .. }))
            .count(),
        1
    );
    assert!(
        !calls
            .iter()
            .any(|call| matches!(call, NativeCall::Close { .. }))
    );
}

#[test]
fn denied_stale_and_native_navigation_failures_preserve_attachment() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline);
    let authority = coordinator(2);
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();

    let calls_before = runtime.calls();
    let denied = Url::parse("https://example.com/denied").unwrap();
    assert_eq!(
        adapter.navigate(generation(2), denied.clone()),
        Err(ChildViewError::NavigationDenied(denied))
    );
    assert_eq!(
        adapter.navigate(
            generation(1),
            Url::parse("http://127.0.0.1:43119/stale").unwrap()
        ),
        Err(ChildViewError::StaleGeneration {
            current: generation(2),
            supplied: generation(1),
        })
    );
    assert_eq!(
        adapter.navigate(
            generation(3),
            Url::parse("http://127.0.0.1:43119/future").unwrap()
        ),
        Err(ChildViewError::FutureGeneration {
            current: generation(2),
            supplied: generation(3),
        })
    );
    assert_eq!(runtime.calls(), calls_before);

    runtime.set_fail_current_url(true);
    assert!(matches!(
        adapter.navigate(
            generation(2),
            Url::parse("http://127.0.0.1:43119/observe-failure").unwrap()
        ),
        Err(ChildViewError::Native {
            operation: "current-url",
            ..
        })
    ));
    runtime.set_fail_current_url(false);
    runtime.set_fail_navigate(true);
    assert!(matches!(
        adapter.navigate(
            generation(2),
            Url::parse("http://127.0.0.1:43119/native-failure").unwrap()
        ),
        Err(ChildViewError::Native {
            operation: "navigate",
            ..
        })
    ));
    assert!(adapter.is_attached(generation(2)).unwrap());
    assert_eq!(
        runtime
            .calls()
            .iter()
            .filter(|call| matches!(call, NativeCall::Attach { .. }))
            .count(),
        1
    );
    assert!(
        !runtime
            .calls()
            .iter()
            .any(|call| matches!(call, NativeCall::Close { .. }))
    );
}

#[test]
fn close_replacement_and_host_destroy_retire_exact_generations() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline);
    let first = coordinator(1);
    adapter.apply(&first, &first.plan().unwrap()).unwrap();

    let mut detach_authority = attached_coordinator(1);
    detach_authority
        .update_desired(
            detach_authority.desired().revision(),
            desired_update(1, DesiredPresence::Absent, DesiredVisibility::Visible),
        )
        .unwrap();
    adapter
        .apply(&detach_authority, &detach_authority.plan().unwrap())
        .unwrap();
    assert_eq!(
        adapter.apply(&first, &first.plan().unwrap()),
        Err(ChildViewError::GenerationRetired(generation(1)))
    );

    let second = coordinator(2);
    adapter.apply(&second, &second.plan().unwrap()).unwrap();
    assert_eq!(
        adapter.admit_runtime_event(runtime_event(
            1,
            ChildViewRuntimeEventKind::PageLoadFinished
        )),
        Err(ChildViewError::StaleGeneration {
            current: generation(2),
            supplied: generation(1)
        })
    );
    let destroyed = adapter
        .host_destroyed(&host_window_id(), generation(2))
        .unwrap();
    assert_eq!(
        destroyed.outcome(),
        ChildViewHostDestroyOutcome::Invalidated
    );
    assert_eq!(
        adapter
            .host_destroyed(&host_window_id(), generation(2))
            .unwrap()
            .outcome(),
        ChildViewHostDestroyOutcome::AlreadyInvalidated
    );
    assert_eq!(
        adapter.admit_runtime_event(runtime_event(
            2,
            ChildViewRuntimeEventKind::PageLoadFinished
        )),
        Err(ChildViewError::GenerationRetired(generation(2)))
    );
    assert_eq!(
        runtime
            .calls()
            .iter()
            .filter(|call| matches!(call, NativeCall::Close { .. }))
            .count(),
        1
    );
}

#[test]
fn failures_are_exact_and_stale_authority_runs_no_native_work() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    runtime.set_fail_bounds(true);
    let failing_adapter = adapter(runtime.clone(), timeline);
    let authority = coordinator(1);
    let receipt = failing_adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    assert_eq!(receipt.steps()[0].outcome(), &OperationOutcome::Applied);
    assert_eq!(
        serde_json::to_value(receipt.steps()[1].outcome()).unwrap()["code"],
        "child:bounds-failed"
    );
    assert!(
        receipt.steps()[2..]
            .iter()
            .all(|step| matches!(step.outcome(), OperationOutcome::DependencySkipped { .. }))
    );

    let clean_runtime = FakeRuntime::default();
    let clean_adapter = adapter(clean_runtime.clone(), Arc::default());
    let mut stale_authority = coordinator(1);
    let stale_plan = stale_authority.plan().unwrap();
    stale_authority
        .update_desired(
            stale_authority.desired().revision(),
            desired_update(
                1,
                DesiredPresence::Present,
                DesiredVisibility::Hidden {
                    reason: VisibilityReasonId::new("proof:stale").unwrap(),
                },
            ),
        )
        .unwrap();
    assert!(matches!(
        clean_adapter.apply(&stale_authority, &stale_plan),
        Err(ChildViewError::Receipt(_))
    ));
    assert!(clean_runtime.calls().is_empty());
}

#[test]
fn teardown_preserves_failed_close_for_retry_then_becomes_idempotent() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline);
    let authority = coordinator(1);
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    runtime.set_fail_close_once();

    assert!(matches!(
        adapter.teardown(),
        Err(ChildViewError::Native {
            operation: "close",
            ..
        })
    ));
    assert!(adapter.is_attached(generation(1)).unwrap());
    assert_eq!(
        adapter.teardown().unwrap().outcome(),
        ChildViewTeardownOutcome::Closed
    );
    assert_eq!(
        adapter.teardown().unwrap().outcome(),
        ChildViewTeardownOutcome::AlreadyDetached
    );
    assert_eq!(
        runtime
            .calls()
            .iter()
            .filter(|call| matches!(call, NativeCall::Close { .. }))
            .count(),
        2
    );
}

#[test]
fn unsupported_portable_input_disable_returns_exact_failure() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime, timeline);
    let authority = NativeContentCoordinator::new(
        DesiredState::new(
            island_id(),
            NativeContentKindId::new("proof:controlled-page").unwrap(),
            CHILD_VIEW_CAPABILITIES,
            DesiredUpdate::new(
                generation(1),
                host_window_id(),
                viewport(),
                ScaleFactor::from_thousandths(1000).unwrap(),
                RoundingMode::Nearest,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Unchanged,
                InputRoutingMode::Disabled,
            ),
        )
        .unwrap(),
    );

    let receipt = adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    let failure = receipt
        .steps()
        .iter()
        .find(|step| matches!(step.outcome(), OperationOutcome::Failed { .. }))
        .unwrap();
    assert_eq!(
        serde_json::to_value(failure.outcome()).unwrap()["code"],
        "child:input-mode"
    );
    assert!(adapter.is_attached(generation(1)).unwrap());
}

#[test]
fn construction_policy_and_capability_example_are_closed_by_default() {
    let spec = spec();
    assert!(spec.allows_navigation(&Url::parse("http://127.0.0.1:43119/other").unwrap()));
    assert!(!spec.allows_navigation(&Url::parse("https://example.com/").unwrap()));

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let capability: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("examples/capabilities/controller-only.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(capability["webviews"], serde_json::json!(["controller"]));
    assert_eq!(capability["permissions"], serde_json::json!([]));
    assert!(capability.get("remote").is_none());
    assert!(
        !capability["webviews"]
            .as_array()
            .unwrap()
            .iter()
            .any(|label| label == "proof-child")
    );
}
