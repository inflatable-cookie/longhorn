//! Private child-webview adapter contract tests over a deterministic fake runtime.

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
use longhorn_native_content_child_webview_prototype::{
    AdapterEvent, ChildWebviewAdapter, ChildWebviewError, ChildWebviewLabel, ChildWebviewRuntime,
    ChildWebviewSpec, DownloadPolicy, PopupPolicy, RemoteCapabilityPolicy, RuntimeAttachRequest,
    RuntimeEvent, RuntimeEventKind,
};
use longhorn_native_content_prototype::{
    ApplyPlan, AttachGeneration, AttachmentLifecycle, DesiredPresence, DesiredState, DesiredUpdate,
    DesiredVisibility, DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent,
    InputRoutingMode, MechanismCapabilities, NativeContentCoordinator, NativeContentIslandId,
    NativeContentKindId, NativeContentMechanism, ObservationUpdate, ObservedGeometry,
    ObservedReadiness, OperationOutcome, VisibilityReasonId,
};
use tauri::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
enum NativeCall {
    Attach { handle: u64, generation: u64 },
    Bounds { handle: u64, bounds: PhysicalRect },
    Show { handle: u64 },
    Hide { handle: u64 },
    Focus { handle: u64 },
    Close { handle: u64 },
    Evaluate { handle: u64, script: String },
}

#[derive(Default)]
struct FakeState {
    next_handle: u64,
    calls: Vec<NativeCall>,
    bounds: BTreeMap<u64, PhysicalRect>,
    callbacks: BTreeMap<u64, Arc<dyn Fn(RuntimeEvent) + Send + Sync>>,
    fail_bounds: bool,
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

    fn emit(&self, handle: u64, generation: u64, kind: RuntimeEventKind) {
        let callback = self
            .state
            .lock()
            .unwrap()
            .callbacks
            .get(&handle)
            .unwrap()
            .clone();
        callback(RuntimeEvent {
            island_id: island_id(),
            generation: AttachGeneration::new(generation),
            webview_label: "proof-child".to_string(),
            kind,
        });
    }
}

impl ChildWebviewRuntime for FakeRuntime {
    type Handle = u64;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, ChildWebviewError> {
        let handle = {
            let mut state = self.state.lock().unwrap();
            state.next_handle += 1;
            let handle = state.next_handle;
            state.calls.push(NativeCall::Attach {
                handle,
                generation: request.generation.get(),
            });
            state.callbacks.insert(handle, request.callback.clone());
            handle
        };
        self.timeline
            .lock()
            .unwrap()
            .push("runtime:attach".to_string());
        (request.callback)(RuntimeEvent {
            island_id: request.spec.island_id().clone(),
            generation: request.generation,
            webview_label: request.spec.webview_label().as_str().to_string(),
            kind: RuntimeEventKind::PageLoadStarted {
                url: request.spec.source().to_string(),
            },
        });
        Ok(handle)
    }

    fn set_bounds(
        &self,
        handle: &Self::Handle,
        bounds: PhysicalRect,
    ) -> Result<(), ChildWebviewError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(NativeCall::Bounds {
            handle: *handle,
            bounds,
        });
        if state.fail_bounds {
            return Err(ChildWebviewError::Native {
                operation: "bounds",
                detail: "injected bounds failure".to_string(),
            });
        }
        state.bounds.insert(*handle, bounds);
        Ok(())
    }

    fn show(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(NativeCall::Show { handle: *handle });
        Ok(())
    }

    fn hide(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(NativeCall::Hide { handle: *handle });
        Ok(())
    }

    fn focus(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(NativeCall::Focus { handle: *handle });
        Ok(())
    }

    fn close(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(NativeCall::Close { handle: *handle });
        Ok(())
    }

    fn bounds(&self, handle: &Self::Handle) -> Result<PhysicalRect, ChildWebviewError> {
        self.state
            .lock()
            .unwrap()
            .bounds
            .get(handle)
            .copied()
            .ok_or_else(|| ChildWebviewError::Native {
                operation: "observe",
                detail: "bounds unavailable".to_string(),
            })
    }

    fn evaluate(&self, handle: &Self::Handle, script: &str) -> Result<(), ChildWebviewError> {
        self.state.lock().unwrap().calls.push(NativeCall::Evaluate {
            handle: *handle,
            script: script.to_string(),
        });
        Ok(())
    }
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
        AttachGeneration::new(generation),
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
    NativeContentCoordinator::new(DesiredState::new(
        island_id(),
        NativeContentKindId::new("proof:controlled-page").unwrap(),
        MechanismCapabilities::new(
            NativeContentMechanism::ChildView,
            false,
            DetachPolicy::Reversible,
            false,
            false,
        ),
        desired_update(
            generation,
            DesiredPresence::Present,
            DesiredVisibility::Visible,
        ),
    ))
}

fn attached_control_plan(generation: u64, visible: bool) -> ApplyPlan {
    let mut coordinator = coordinator(generation);
    coordinator
        .admit_observation(
            coordinator.observed().revision(),
            ObservationUpdate::new(
                AttachGeneration::new(generation),
                AttachmentLifecycle::Attached,
                ObservedReadiness::Ready,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::ChildBounds {
                    bounds: physical_viewport(),
                },
                Some(InputRoutingMode::NativeDirect),
            ),
        )
        .unwrap();
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
    coordinator.plan().unwrap()
}

fn spec() -> ChildWebviewSpec {
    let source = Url::parse("http://127.0.0.1:43119/proof").unwrap();
    let origin = source.origin().ascii_serialization();
    ChildWebviewSpec::new(
        island_id(),
        host_window_id(),
        ChildWebviewLabel::new("host").unwrap(),
        ChildWebviewLabel::new("proof-child").unwrap(),
        source,
        Some(*b"longhorn-proof-1"),
        Arc::new(move |candidate| candidate.origin().ascii_serialization() == origin),
        PopupPolicy::Deny,
        DownloadPolicy::Deny,
        RemoteCapabilityPolicy::NoRemoteCapabilities,
    )
    .unwrap()
}

fn adapter(
    runtime: FakeRuntime,
    timeline: Arc<Mutex<Vec<String>>>,
) -> ChildWebviewAdapter<FakeRuntime> {
    ChildWebviewAdapter::new(
        runtime,
        spec(),
        Arc::new(move |event| {
            let label = match event {
                AdapterEvent::ListenerInstalled { .. } => "adapter:listener",
                AdapterEvent::AttachStarted { .. } => "adapter:attach-started",
                AdapterEvent::Attached { .. } => "adapter:attached",
                AdapterEvent::Runtime { .. } => "adapter:runtime",
                AdapterEvent::RendererUnmounted { .. } => "adapter:renderer-unmounted",
                AdapterEvent::DetachStarted { .. } => "adapter:detach-started",
                AdapterEvent::Detached { .. } => "adapter:detached",
                AdapterEvent::HostInvalidated { .. } => "adapter:host-invalidated",
            };
            timeline.lock().unwrap().push(label.to_string());
        }),
    )
}

#[test]
fn initial_plan_installs_callbacks_before_attach_and_applies_the_full_child_sequence() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline.clone());
    let plan = coordinator(1).plan().unwrap();

    let receipt = adapter.apply(&plan).unwrap();

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
                generation: 1,
            },
            NativeCall::Bounds {
                handle: 1,
                bounds: physical_viewport(),
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
fn observation_uses_native_physical_bounds_without_inventing_visibility_or_focus() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline.clone());
    adapter.apply(&coordinator(1).plan().unwrap()).unwrap();
    runtime.emit(
        1,
        1,
        RuntimeEventKind::PageLoadFinished {
            url: "http://127.0.0.1:43119/proof".to_string(),
        },
    );

    let observed = adapter.observe(AttachGeneration::new(1)).unwrap();
    let value = serde_json::to_value(observed).unwrap();

    assert_eq!(value["lifecycle"], "attached");
    assert_eq!(value["readiness"], "ready");
    assert_eq!(value["visibility"], "unknown");
    assert_eq!(value["focus"], "unknown");
    assert_eq!(value["input_routing"], "native_direct");
    assert_eq!(value["geometry"]["kind"], "child_bounds");
    assert_eq!(value["geometry"]["bounds"]["origin"]["x"], 12);
    assert_eq!(value["geometry"]["bounds"]["size"]["width"], 360);
}

#[test]
fn renderer_unmount_and_hide_show_cycles_reuse_one_native_handle() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline);
    let generation = AttachGeneration::new(1);
    adapter.apply(&coordinator(1).plan().unwrap()).unwrap();

    adapter.apply(&attached_control_plan(1, false)).unwrap();
    adapter.renderer_unmounted(generation).unwrap();
    adapter
        .evaluate(generation, "window.__longhornProofProbe('after-unmount')")
        .unwrap();
    adapter.apply(&attached_control_plan(1, true)).unwrap();

    assert!(adapter.is_attached(generation).unwrap());
    let calls = runtime.calls();
    assert!(calls.contains(&NativeCall::Hide { handle: 1 }));
    assert!(calls.contains(&NativeCall::Show { handle: 1 }));
    assert!(calls.contains(&NativeCall::Evaluate {
        handle: 1,
        script: "window.__longhornProofProbe('after-unmount')".to_string(),
    }));
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
fn reversible_close_replacement_and_host_destruction_enforce_generation_authority() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    let adapter = adapter(runtime.clone(), timeline.clone());
    let mut first = coordinator(1);
    adapter.apply(&first.plan().unwrap()).unwrap();
    first
        .admit_observation(
            first.observed().revision(),
            ObservationUpdate::new(
                AttachGeneration::new(1),
                AttachmentLifecycle::Attached,
                ObservedReadiness::Ready,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::ChildBounds {
                    bounds: physical_viewport(),
                },
                Some(InputRoutingMode::NativeDirect),
            ),
        )
        .unwrap();
    first
        .update_desired(
            first.desired().revision(),
            desired_update(1, DesiredPresence::Absent, DesiredVisibility::Visible),
        )
        .unwrap();
    adapter.apply(&first.plan().unwrap()).unwrap();
    assert!(!adapter.is_attached(AttachGeneration::new(1)).unwrap());
    let close_timeline = timeline.lock().unwrap().clone();
    let detach_started = close_timeline
        .iter()
        .position(|event| event == "adapter:detach-started")
        .unwrap();
    let detached = close_timeline
        .iter()
        .position(|event| event == "adapter:detached")
        .unwrap();
    assert!(detach_started < detached);

    adapter.apply(&coordinator(2).plan().unwrap()).unwrap();
    assert_eq!(
        adapter.admit_runtime_event(RuntimeEvent {
            island_id: island_id(),
            generation: AttachGeneration::new(1),
            webview_label: "proof-child".to_string(),
            kind: RuntimeEventKind::PageLoadFinished {
                url: "http://127.0.0.1:43119/stale".to_string(),
            },
        }),
        Err(ChildWebviewError::StaleGeneration {
            current: AttachGeneration::new(2),
            supplied: AttachGeneration::new(1),
        })
    );

    let invalidated = adapter.host_destroyed(&host_window_id()).unwrap().unwrap();
    assert_eq!(invalidated.generation(), AttachGeneration::new(2));
    assert_eq!(invalidated.island_id(), &island_id());
    assert_eq!(
        adapter.admit_runtime_event(RuntimeEvent {
            island_id: island_id(),
            generation: AttachGeneration::new(2),
            webview_label: "proof-child".to_string(),
            kind: RuntimeEventKind::PageLoadFinished {
                url: "http://127.0.0.1:43119/late".to_string(),
            },
        }),
        Err(ChildWebviewError::NotAttached)
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
fn native_failure_yields_exact_partial_receipt_and_dependency_skips() {
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeRuntime::with_timeline(timeline.clone());
    runtime.set_fail_bounds(true);
    let adapter = adapter(runtime, timeline);

    let receipt = adapter.apply(&coordinator(1).plan().unwrap()).unwrap();

    assert_eq!(receipt.steps().len(), 5);
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
}

#[test]
fn security_policy_is_closed_and_only_the_local_controller_has_a_capability() {
    let spec = spec();
    assert_eq!(spec.popup_policy(), PopupPolicy::Deny);
    assert_eq!(spec.download_policy(), DownloadPolicy::Deny);
    assert_eq!(
        spec.remote_capability_policy(),
        RemoteCapabilityPolicy::NoRemoteCapabilities
    );
    assert!(spec.allows_navigation(&Url::parse("http://127.0.0.1:43119/other").unwrap()));
    assert!(!spec.allows_navigation(&Url::parse("https://example.com/").unwrap()));

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../proof-app/src-tauri");
    let capability: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("capabilities/controller.json")).unwrap(),
    )
    .unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("tauri.conf.json")).unwrap()).unwrap();
    assert_eq!(capability["webviews"], serde_json::json!(["controller"]));
    assert_eq!(capability["permissions"], serde_json::json!([]));
    assert!(capability.get("windows").is_none());
    assert!(capability.get("remote").is_none());
    assert_eq!(config["app"]["withGlobalTauri"], false);
}
