use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalPoint, RoundingMode, ScaleFactor, WindowId,
};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, DesiredPresence, DesiredState, DesiredUpdate,
    DesiredVisibility, FocusIntent, HostDestroyOutcome, InputRoutingMode, NativeContentCoordinator,
    NativeContentIslandId, NativeContentKindId, OperationOutcome, VisibilityReasonId,
    viewport_to_physical,
};
use longhorn_native_content_backing_surface::{
    BACKING_SURFACE_CAPABILITIES, BackingSurfaceAdapter, BackingSurfaceAdapterEvent,
    BackingSurfaceDetachOutcome, BackingSurfaceError, BackingSurfaceHostDestroyOutcome,
    BackingSurfaceRuntimeEvent, BackingSurfaceRuntimeEventKind, BackingSurfaceSnapshot,
    BackingSurfaceSpec, InputAdmission, InputRejection,
};
use serde_json::json;
use tauri::{Manager, PhysicalPosition, PhysicalSize as TauriPhysicalSize, WebviewWindow, Wry};

use crate::{
    evidence::{Check, CheckStatus, EvidenceLog, ProofReport},
    runtime::TauriBackingRuntime,
};

const INITIAL_VIEWPORT: (f64, f64, f64, f64) = (120.0, 90.0, 420.0, 280.0);

type Adapter = BackingSurfaceAdapter<TauriBackingRuntime>;

pub(crate) fn run(
    app: tauri::AppHandle<Wry>,
    initial_scale: ScaleFactor,
    log: Arc<EvidenceLog>,
) -> Result<(), String> {
    let events = Arc::new(Mutex::new(Vec::<BackingSurfaceAdapterEvent>::new()));
    let runtime = TauriBackingRuntime::new(app.clone(), "controller");
    let event_log = log.clone();
    let event_store = events.clone();
    let adapter = BackingSurfaceAdapter::new(
        runtime.clone(),
        BackingSurfaceSpec::new(island_id()?, host_window_id()?),
        Arc::new(move |event| {
            event_store
                .lock()
                .expect("event store is not poisoned")
                .push(event.clone());
            let detail = serde_json::to_value(event)
                .unwrap_or_else(|error| json!({"serialization_error": error.to_string()}));
            let _ = event_log.record("adapter_event", detail);
        }),
    );
    let mut coordinator = coordinator(initial_scale)?;
    let mut checks = Vec::new();

    let initial_plan = coordinator.plan().map_err(string_error)?;
    let initial_receipt = adapter
        .apply(&coordinator, &initial_plan)
        .map_err(string_error)?;
    observe_and_admit(&adapter, &mut coordinator)?;
    let handle = runtime.only_handle().map_err(string_error)?;
    let initial = runtime.snapshot(handle).map_err(string_error)?;
    let initial_pixels = runtime.pixels(handle).map_err(string_error)?;
    log.record(
        "initial_attach",
        json!({"plan": initial_plan, "receipt": initial_receipt, "snapshot": initial, "pixels": initial_pixels}),
    )?;
    checks.push(Check::new(
        "packaged_macos_native_view_below_transparent_webview",
        status(
            std::env::consts::OS == "macos"
                && all_applied(&initial_receipt)
                && initial.native_storage_attached,
        ),
        json!({
            "platform": std::env::consts::OS,
            "native_storage_attached": initial.native_storage_attached,
            "transparent_webview": true,
            "native_order": "NSView below webview",
            "controlled_fixture": "full-host NSView plus deterministic consumer renderer",
        }),
    ));
    checks.push(Check::new(
        "full_host_storage_and_deterministic_clip_pixels_are_distinct",
        status(
            initial.storage_bounds != initial.clip
                && initial_pixels.lit_pixels == area(initial.clip)
                && initial_pixels.outside_clip_lit_pixels == 0,
        ),
        json!({"snapshot": initial, "pixels": initial_pixels}),
    ));

    let initial_storage = initial.storage_bounds;
    let initial_digest = initial_pixels.digest.clone();
    let moved = transition(
        &adapter,
        &runtime,
        &mut coordinator,
        update(
            viewport(180.0, 120.0, 300.0, 220.0)?,
            initial_scale,
            DesiredVisibility::Visible,
            InputRoutingMode::RendererForwarded,
        )?,
        "viewport_move_resize",
        &log,
    )?;
    let stale_move_plan = moved.plan.clone();
    checks.push(Check::new(
        "viewport_move_and_resize_change_clip_not_storage",
        status(
            moved.snapshot.storage_bounds == initial_storage
                && moved.snapshot.clip != initial.clip
                && moved.pixels.digest != initial_digest
                && moved.pixels.outside_clip_lit_pixels == 0,
        ),
        json!({"before": initial, "after": moved.snapshot, "pixels": moved.pixels}),
    ));

    let collapsed = transition(
        &adapter,
        &runtime,
        &mut coordinator,
        update(
            viewport(180.0, 120.0, 0.0, 0.0)?,
            initial_scale,
            DesiredVisibility::Visible,
            InputRoutingMode::RendererForwarded,
        )?,
        "viewport_zero_collapse",
        &log,
    )?;
    let collapsed_input = adapter
        .admit_input(generation(), PhysicalPoint::new(360, 240))
        .map_err(string_error)?;
    let restored = transition(
        &adapter,
        &runtime,
        &mut coordinator,
        update(
            initial_viewport()?,
            initial_scale,
            DesiredVisibility::Visible,
            InputRoutingMode::RendererForwarded,
        )?,
        "viewport_restore",
        &log,
    )?;
    checks.push(Check::new(
        "zero_viewport_suppresses_without_detach_then_restores",
        status(
            collapsed.snapshot.clip.size().is_empty()
                && collapsed.pixels.lit_pixels == 0
                && collapsed.snapshot.native_storage_attached
                && collapsed_input == InputAdmission::Rejected(InputRejection::EmptyViewport)
                && restored.snapshot.native_storage_attached
                && restored.pixels.lit_pixels > 0,
        ),
        json!({
            "collapsed": collapsed.snapshot,
            "collapsed_pixels": collapsed.pixels,
            "collapsed_input": collapsed_input,
            "restored": restored.snapshot,
            "restored_pixels": restored.pixels,
        }),
    ));

    #[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
    enum ConsumerAction {
        SelectNode { node: u32 },
    }
    let semantic_callbacks = Arc::new(Mutex::new(Vec::<ConsumerAction>::new()));
    adapter
        .update_host_focus(generation(), true)
        .map_err(string_error)?;
    let inside_point = PhysicalPoint::new(
        restored.snapshot.clip.origin().x().get().saturating_add(4),
        restored.snapshot.clip.origin().y().get().saturating_add(4),
    );
    let outside_point = PhysicalPoint::new(2, 2);
    let inside = adapter
        .admit_input(generation(), inside_point)
        .map_err(string_error)?;
    if inside == InputAdmission::Admitted {
        semantic_callbacks
            .lock()
            .map_err(|_| "semantic callback store is poisoned".to_string())?
            .push(ConsumerAction::SelectNode { node: 42 });
    }
    let outside = adapter
        .admit_input(generation(), outside_point)
        .map_err(string_error)?;
    checks.push(Check::new(
        "forwarded_input_gate_precedes_consumer_owned_payload",
        status(
            inside == InputAdmission::Admitted
                && outside == InputAdmission::Rejected(InputRejection::OutsideViewport)
                && *semantic_callbacks
                    .lock()
                    .map_err(|_| "semantic callback store is poisoned".to_string())?
                    == vec![ConsumerAction::SelectNode { node: 42 }],
        ),
        json!({
            "inside": inside,
            "outside": outside,
            "consumer_callbacks": semantic_callbacks.lock().map_err(|_| "semantic callback store is poisoned".to_string())?.clone(),
            "payload_boundary": "consumer callback invoked only after physical gate admission",
        }),
    ));

    let hidden = transition(
        &adapter,
        &runtime,
        &mut coordinator,
        update(
            initial_viewport()?,
            initial_scale,
            DesiredVisibility::Hidden {
                reason: VisibilityReasonId::new("proof:visibility-gate").map_err(string_error)?,
            },
            InputRoutingMode::RendererForwarded,
        )?,
        "visibility_hidden",
        &log,
    )?;
    let hidden_input = adapter
        .admit_input(generation(), inside_point)
        .map_err(string_error)?;
    let shown = transition(
        &adapter,
        &runtime,
        &mut coordinator,
        update(
            initial_viewport()?,
            initial_scale,
            DesiredVisibility::Visible,
            InputRoutingMode::RendererForwarded,
        )?,
        "visibility_restored",
        &log,
    )?;
    adapter
        .update_host_focus(generation(), false)
        .map_err(string_error)?;
    let unfocused_input = adapter
        .admit_input(generation(), inside_point)
        .map_err(string_error)?;
    adapter
        .update_host_focus(generation(), true)
        .map_err(string_error)?;
    checks.push(Check::new(
        "visibility_and_consumer_host_focus_gate_presentation_and_input",
        status(
            !hidden.snapshot.presentation_enabled
                && hidden.pixels.lit_pixels == 0
                && hidden_input == InputAdmission::Rejected(InputRejection::PresentationDisabled)
                && shown.snapshot.presentation_enabled
                && unfocused_input == InputAdmission::Rejected(InputRejection::HostUnfocused),
        ),
        json!({
            "hidden": hidden.snapshot,
            "hidden_input": hidden_input,
            "shown": shown.snapshot,
            "unfocused_input": unfocused_input,
            "native_focus_observation": "unknown; host focus is injected gate evidence only",
        }),
    ));

    let window = app
        .get_webview_window("controller")
        .ok_or_else(|| "controller window is missing".to_string())?;
    let before_resize = runtime.snapshot(handle).map_err(string_error)?;
    let inner = window.inner_size().map_err(string_error)?;
    window
        .set_size(TauriPhysicalSize::new(
            inner.width.saturating_add(120),
            inner.height.saturating_add(80),
        ))
        .map_err(string_error)?;
    thread::sleep(Duration::from_millis(350));
    let after_resize = adapter
        .refresh_host_geometry(generation())
        .map_err(string_error)?;
    checks.push(Check::new(
        "fresh_host_resize_changes_full_storage_not_desired_clip",
        status(
            after_resize.storage_bounds != before_resize.storage_bounds
                && after_resize.clip == before_resize.clip
                && after_resize.native_storage_attached,
        ),
        json!({"before": before_resize, "after": after_resize}),
    ));

    checks.push(exercise_available_scale(
        &window,
        &adapter,
        &runtime,
        &mut coordinator,
        initial_scale,
        &log,
    )?);

    let before_stale = runtime.snapshot(handle).map_err(string_error)?;
    let stale_plan_error = adapter.apply(&coordinator, &stale_move_plan).unwrap_err();
    let stale_event_error = adapter
        .admit_runtime_event(BackingSurfaceRuntimeEvent {
            island_id: island_id()?,
            host_window_id: host_window_id()?,
            generation: AttachGeneration::INITIAL,
            sequence: u64::MAX,
            kind: BackingSurfaceRuntimeEventKind::FramePresented { sequence: u64::MAX },
        })
        .unwrap_err();
    let after_stale = runtime.snapshot(handle).map_err(string_error)?;
    checks.push(Check::new(
        "stale_viewport_plan_and_native_callback_leave_exact_state_unchanged",
        status(
            matches!(stale_plan_error, BackingSurfaceError::Receipt(_))
                && matches!(
                    stale_event_error,
                    BackingSurfaceError::StaleGeneration { .. }
                )
                && before_stale == after_stale,
        ),
        json!({
            "stale_plan_error": stale_plan_error.to_string(),
            "stale_event_error": stale_event_error.to_string(),
            "before": before_stale,
            "after": after_stale,
        }),
    ));

    checks.push(boundary_audit());

    let coordinator_destroy = coordinator
        .host_destroyed(&host_window_id()?, coordinator.observed().revision())
        .map_err(string_error)?;
    let invalidated = adapter
        .host_destroyed(&host_window_id()?, generation())
        .map_err(string_error)?;
    let late = adapter
        .admit_runtime_event(BackingSurfaceRuntimeEvent {
            island_id: island_id()?,
            host_window_id: host_window_id()?,
            generation: generation(),
            sequence: u64::MAX,
            kind: BackingSurfaceRuntimeEventKind::FramePresented { sequence: u64::MAX },
        })
        .unwrap_err();
    let event_order = events
        .lock()
        .map_err(|_| "event store is poisoned".to_string())?;
    let invalidated_index = event_order
        .iter()
        .position(|event| matches!(event, BackingSurfaceAdapterEvent::HostInvalidated { .. }));
    let detached_index = event_order
        .iter()
        .position(|event| matches!(event, BackingSurfaceAdapterEvent::Detached { .. }));
    checks.push(Check::new(
        "destroy_invalidates_generation_before_reversible_detach",
        status(
            coordinator_destroy.outcome() == HostDestroyOutcome::Invalidated
                && invalidated.outcome() == BackingSurfaceHostDestroyOutcome::Invalidated
                && invalidated.detach() == BackingSurfaceDetachOutcome::Detached
                && invalidated_index
                    .zip(detached_index)
                    .is_some_and(|(left, right)| left < right)
                && matches!(late, BackingSurfaceError::GenerationInvalidated(_)),
        ),
        json!({
            "coordinator_outcome": coordinator_destroy.outcome(),
            "adapter_outcome": invalidated.outcome(),
            "detach_outcome": invalidated.detach(),
            "late_callback_error": late.to_string(),
            "invalidated_event_index": invalidated_index,
            "detached_event_index": detached_index,
        }),
    ));
    drop(event_order);

    let report = ProofReport::completed(log.root().to_path_buf(), checks);
    log.write_report(&report)?;
    log.record(
        "proof_completed",
        json!({"report": log.report_path(), "failed": report.failed()}),
    )?;
    if report.failed() {
        return Err("one or more backing-surface production checks failed".to_string());
    }
    app.exit(0);
    Ok(())
}

struct Transition {
    plan: ApplyPlan,
    snapshot: BackingSurfaceSnapshot,
    pixels: crate::deterministic_renderer::PixelEvidence,
}

fn transition(
    adapter: &Adapter,
    runtime: &TauriBackingRuntime,
    coordinator: &mut NativeContentCoordinator,
    update: DesiredUpdate,
    label: &'static str,
    log: &EvidenceLog,
) -> Result<Transition, String> {
    coordinator
        .update_desired(coordinator.desired().revision(), update)
        .map_err(string_error)?;
    let plan = coordinator.plan().map_err(string_error)?;
    let receipt = adapter.apply(coordinator, &plan).map_err(string_error)?;
    observe_and_admit(adapter, coordinator)?;
    let handle = runtime.only_handle().map_err(string_error)?;
    let snapshot = runtime.snapshot(handle).map_err(string_error)?;
    let pixels = runtime.pixels(handle).map_err(string_error)?;
    log.record(
        label,
        json!({"plan": plan, "receipt": receipt, "snapshot": snapshot, "pixels": pixels}),
    )?;
    if !all_applied(&receipt) {
        return Err(format!("{label} did not apply every planned operation"));
    }
    Ok(Transition {
        plan,
        snapshot,
        pixels,
    })
}

fn observe_and_admit(
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
) -> Result<(), String> {
    let observation = adapter.observe(generation()).map_err(string_error)?;
    coordinator
        .admit_observation(coordinator.observed().revision(), observation)
        .map_err(string_error)?;
    Ok(())
}

fn exercise_available_scale(
    window: &WebviewWindow<Wry>,
    adapter: &Adapter,
    runtime: &TauriBackingRuntime,
    coordinator: &mut NativeContentCoordinator,
    current_model_scale: ScaleFactor,
    log: &EvidenceLog,
) -> Result<Check, String> {
    let current_native = window.scale_factor().map_err(string_error)?;
    let original_position = window.outer_position().map_err(string_error)?;
    let monitors = window.available_monitors().map_err(string_error)?;
    let available: Vec<_> = monitors
        .iter()
        .map(|monitor| {
            json!({
                "name": monitor.name(),
                "scale": monitor.scale_factor(),
                "position": monitor.position(),
                "size": monitor.size(),
            })
        })
        .collect();
    let Some(target) = monitors
        .iter()
        .find(|monitor| (monitor.scale_factor() - current_native).abs() > f64::EPSILON)
    else {
        return Ok(Check::new(
            "available_native_scale_transition",
            CheckStatus::Unmet,
            json!({
                "reason": "no attached monitor exposes a distinct native scale",
                "current_native_scale": current_native,
                "current_model_scale": current_model_scale,
                "available_monitors": available,
                "simulation_used": false,
            }),
        ));
    };

    window
        .set_position(PhysicalPosition::new(
            target.position().x.saturating_add(24),
            target.position().y.saturating_add(24),
        ))
        .map_err(string_error)?;
    thread::sleep(Duration::from_millis(650));
    let transitioned_native = window.scale_factor().map_err(string_error)?;
    if (transitioned_native - current_native).abs() <= f64::EPSILON {
        window
            .set_position(original_position)
            .map_err(string_error)?;
        return Ok(Check::new(
            "available_native_scale_transition",
            CheckStatus::Unmet,
            json!({
                "reason": "distinct-scale monitor exists but the packaged window did not transition",
                "current_native_scale": current_native,
                "target_native_scale": target.scale_factor(),
                "observed_native_scale": transitioned_native,
                "available_monitors": available,
                "simulation_used": false,
            }),
        ));
    }

    let model_scale = model_scale(transitioned_native)?;
    let transitioned = transition(
        adapter,
        runtime,
        coordinator,
        update(
            initial_viewport()?,
            model_scale,
            DesiredVisibility::Visible,
            InputRoutingMode::RendererForwarded,
        )?,
        "native_scale_transition",
        log,
    )?;
    let expected = viewport_to_physical(initial_viewport()?, model_scale, RoundingMode::Nearest)
        .map_err(string_error)?;
    window
        .set_position(original_position)
        .map_err(string_error)?;
    Ok(Check::new(
        "available_native_scale_transition",
        status(
            transitioned.snapshot.native_scale == model_scale
                && transitioned.snapshot.clip == expected
                && transitioned.pixels.outside_clip_lit_pixels == 0,
        ),
        json!({
            "from_native_scale": current_native,
            "to_native_scale": transitioned_native,
            "model_scale": model_scale,
            "expected_clip": expected,
            "snapshot": transitioned.snapshot,
            "available_monitors": available,
            "simulation_used": false,
        }),
    ))
}

fn boundary_audit() -> Check {
    let adapter_source = concat!(
        include_str!("../../../../crates/longhorn-native-content-backing-surface/src/adapter.rs"),
        include_str!("../../../../crates/longhorn-native-content-backing-surface/src/runtime.rs"),
        include_str!("../../../../crates/longhorn-native-content-backing-surface/src/lib.rs"),
    )
    .to_ascii_lowercase();
    let adapter_manifest =
        include_str!("../../../../crates/longhorn-native-content-backing-surface/Cargo.toml")
            .to_ascii_lowercase();
    let frontend = include_str!("../../frontend/index.html").to_ascii_lowercase();
    let forbidden_types = ["wgpu", "scene graph", "camera", "picking", "gizmo"];
    let forbidden_edges = [
        "native-content-child-view",
        "native-content-isolated-window",
        "tauri",
        "poodle",
    ];
    let clean_types = forbidden_types
        .iter()
        .all(|term| !adapter_source.contains(term));
    let clean_edges = forbidden_edges
        .iter()
        .all(|term| !adapter_manifest.contains(term));
    Check::new(
        "adapter_payload_poodle_native_handle_and_dependency_boundary",
        status(clean_types && clean_edges && !frontend.contains("poodle")),
        json!({
            "adapter_forbidden_type_terms_absent": clean_types,
            "adapter_forbidden_dependency_edges_absent": clean_edges,
            "semantic_payload_in_adapter": false,
            "raw_native_handle_exposed_by_adapter": false,
            "poodle_private_dom_inspection": false,
            "visual_chrome": "ordinary controlled HTML/CSS fixture",
            "renderer_fixture_location": "packaged proof only",
        }),
    )
}

fn coordinator(scale: ScaleFactor) -> Result<NativeContentCoordinator, String> {
    let desired = DesiredState::new(
        island_id()?,
        NativeContentKindId::new("proof:deterministic-consumer-renderer").map_err(string_error)?,
        BACKING_SURFACE_CAPABILITIES,
        update(
            initial_viewport()?,
            scale,
            DesiredVisibility::Visible,
            InputRoutingMode::RendererForwarded,
        )?,
    )
    .map_err(string_error)?;
    Ok(NativeContentCoordinator::new(desired))
}

fn update(
    viewport: ClientRect,
    scale: ScaleFactor,
    visibility: DesiredVisibility,
    route: InputRoutingMode,
) -> Result<DesiredUpdate, String> {
    Ok(DesiredUpdate::new(
        generation(),
        host_window_id()?,
        viewport,
        scale,
        RoundingMode::Nearest,
        DesiredPresence::Present,
        visibility,
        FocusIntent::Unchanged,
        route,
    ))
}

fn viewport(x: f64, y: f64, width: f64, height: f64) -> Result<ClientRect, String> {
    Ok(ClientRect::new(
        ClientPoint::new(x, y).map_err(string_error)?,
        ClientSize::new(width, height).map_err(string_error)?,
    ))
}

fn initial_viewport() -> Result<ClientRect, String> {
    viewport(
        INITIAL_VIEWPORT.0,
        INITIAL_VIEWPORT.1,
        INITIAL_VIEWPORT.2,
        INITIAL_VIEWPORT.3,
    )
}

fn generation() -> AttachGeneration {
    AttachGeneration::new(2).expect("production proof generation is nonzero")
}

fn island_id() -> Result<NativeContentIslandId, String> {
    NativeContentIslandId::new("island:backing-surface-production-proof").map_err(string_error)
}

fn host_window_id() -> Result<WindowId, String> {
    WindowId::new("window:backing-surface-production-proof").map_err(string_error)
}

fn model_scale(value: f64) -> Result<ScaleFactor, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("invalid native scale {value}"));
    }
    ScaleFactor::from_thousandths((value * 1_000.0).round() as u32).map_err(string_error)
}

fn all_applied(receipt: &ApplyReceipt) -> bool {
    receipt
        .steps()
        .iter()
        .all(|step| matches!(step.outcome(), OperationOutcome::Applied))
}

fn area(rect: longhorn_core::PhysicalRect) -> u64 {
    rect.area()
}

const fn status(condition: bool) -> CheckStatus {
    if condition {
        CheckStatus::Pass
    } else {
        CheckStatus::Fail
    }
}

fn string_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
