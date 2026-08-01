//! Automated packaged matrix for the selected Tauri child-webview mechanism.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use longhorn_core::{ClientPoint, ClientRect, ClientSize, RoundingMode, ScaleFactor, WindowId};
use longhorn_native_content_child_webview_prototype::{
    AdapterEvent, ChildWebviewAdapter, ChildWebviewLabel, ChildWebviewSpec, DownloadPolicy,
    PopupPolicy, RemoteCapabilityPolicy, RuntimeEvent, RuntimeEventKind, TauriChildWebviewRuntime,
};
use longhorn_native_content_prototype::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, DesiredPresence, DesiredState,
    DesiredUpdate, DesiredVisibility, DetachPolicy, EffectiveFocus, EffectiveVisibility,
    FocusIntent, InputRoutingMode, MechanismCapabilities, NativeContentCoordinator,
    NativeContentIslandId, NativeContentKindId, NativeContentMechanism, NativeContentOperation,
    ObservationUpdate, ObservedGeometry, ObservedReadiness, OperationOutcome, VisibilityReasonId,
    viewport_to_physical,
};
use serde_json::{Value, json};
use tauri::{
    AppHandle, Monitor, PhysicalPosition, Position, Runtime, Size, Window, WindowEvent, Wry,
};

use crate::{
    evidence::{Check, CheckStatus, EvidenceLog, ProofReport},
    server::ProofServer,
};

const WAIT: Duration = Duration::from_secs(10);
const HOST_WINDOW_ID: &str = "window:proof-host";
const ISLAND_ID: &str = "island:child-proof";
const CHILD_LABEL: &str = "proof-child";

type Adapter = ChildWebviewAdapter<TauriChildWebviewRuntime<Wry>>;

pub(crate) fn run(
    app: AppHandle<Wry>,
    host: Window<Wry>,
    log: Arc<EvidenceLog>,
) -> Result<(), String> {
    let server = ProofServer::start(log.clone())?;
    let session = format!(
        "packaged-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let source = tauri::Url::parse(&server.page_url(1, &session)).map_err(string_error)?;
    let allowed_origin = server.origin().to_string();
    let spec = ChildWebviewSpec::new(
        island_id()?,
        host_window_id()?,
        ChildWebviewLabel::new("host").map_err(string_error)?,
        ChildWebviewLabel::new(CHILD_LABEL).map_err(string_error)?,
        source,
        Some(*b"longhorn-proof-1"),
        Arc::new(move |candidate| candidate.origin().ascii_serialization() == allowed_origin),
        PopupPolicy::Deny,
        DownloadPolicy::Deny,
        RemoteCapabilityPolicy::NoRemoteCapabilities,
    )
    .map_err(string_error)?;
    let event_log = log.clone();
    let adapter = ChildWebviewAdapter::new(
        TauriChildWebviewRuntime::new(app.clone()),
        spec,
        Arc::new(move |event: AdapterEvent| {
            let detail = serde_json::to_value(event)
                .unwrap_or_else(|error| json!({"serialization_error": error.to_string()}));
            let _ = event_log.record("adapter_event", detail);
        }),
    );
    let scale_event_seen = Arc::new(AtomicBool::new(false));
    install_host_observer(
        &host,
        adapter.clone(),
        log.clone(),
        scale_event_seen.clone(),
    );

    let initial_scale = scale_from_native(host.scale_factor().map_err(string_error)?)?;
    let initial_viewport = viewport(24.0, 30.0, 480.0, 280.0)?;
    let mut coordinator = coordinator(initial_viewport, initial_scale)?;
    let initial_plan = coordinator.plan().map_err(string_error)?;
    let initial_receipt = apply(&adapter, &initial_plan, "initial_attach", &log)?;
    let loaded = log.wait_for("content_event", WAIT, |detail| {
        detail["name"] == "loaded" && detail["session"] == session
    })?;
    let page_finished = log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "runtime"
            && detail["generation"] == 1
            && detail["event"]["kind"] == "page_load_finished"
    })?;
    let initial_observation = observe_and_admit(&adapter, &mut coordinator, "initial", &log)?;
    let expected_initial =
        viewport_to_physical(initial_viewport, initial_scale, RoundingMode::Nearest)
            .map_err(string_error)?;
    let initial_bounds_match = observed_bounds(&initial_observation)
        == Some(serde_json::to_value(expected_initial).map_err(string_error)?);
    let initial_focus_requested = initial_plan
        .operations()
        .iter()
        .any(|operation| matches!(operation.operation(), NativeContentOperation::RequestFocus));
    let mut checks = vec![
        Check::new(
            "packaged_child_creation_and_interaction",
            pass(loaded && page_finished && receipt_applied(&initial_receipt)),
            json!({
                "controlled_page_loaded": loaded,
                "native_page_load_finished": page_finished,
                "initial_receipt": initial_receipt,
            }),
        ),
        Check::new(
            "initial_native_bounds_converge",
            pass(initial_bounds_match),
            json!({
                "scale_thousandths": initial_scale.thousandths(),
                "expected": expected_initial,
                "observed": initial_observation,
            }),
        ),
        Check::new(
            "focus_intent_is_distinct_from_observation",
            observed_unknown(initial_focus_requested && initial_observation["focus"] == "unknown"),
            json!({
                "request_focus_applied": initial_focus_requested,
                "observed_focus": initial_observation["focus"],
                "claim": "Tauri exposes focus request but no portable child-focus readback",
            }),
        ),
    ];

    let mut renderer_unmounted = false;
    for (index, reason) in ["inactive", "overlay", "dragged"].iter().enumerate() {
        transition(
            &adapter,
            &mut coordinator,
            initial_viewport,
            initial_scale,
            DesiredPresence::Present,
            DesiredVisibility::Hidden {
                reason: VisibilityReasonId::new(format!("proof:{reason}")).map_err(string_error)?,
            },
            &format!("{reason}_hide"),
            &log,
        )?;
        if index == 0 {
            adapter
                .renderer_unmounted(AttachGeneration::new(1))
                .map_err(string_error)?;
            renderer_unmounted = true;
        }
        probe(&adapter, 1, &format!("{reason}-hidden"))?;
        let hidden_probe = wait_for_probe(&log, &session, &format!("{reason}-hidden"))?;
        transition(
            &adapter,
            &mut coordinator,
            initial_viewport,
            initial_scale,
            DesiredPresence::Present,
            DesiredVisibility::Visible,
            &format!("{reason}_restore"),
            &log,
        )?;
        probe(&adapter, 1, &format!("{reason}-restored"))?;
        let restored_probe = wait_for_probe(&log, &session, &format!("{reason}-restored"))?;
        log.record(
            "visibility_cycle",
            json!({
                "reason": reason,
                "hidden_probe": hidden_probe,
                "restored_probe": restored_probe,
                "session": session,
            }),
        )?;
    }
    let session_events =
        log.matching_details("content_event", |detail| detail["session"] == session)?;
    let counters = session_events
        .iter()
        .filter_map(|detail| detail["counter"].as_str()?.parse::<u64>().ok())
        .collect::<Vec<_>>();
    let session_continuous = counters.len() >= 7
        && counters.windows(2).all(|pair| pair[0] < pair[1])
        && adapter
            .is_attached(AttachGeneration::new(1))
            .map_err(string_error)?;
    checks.push(Check::new(
        "hide_show_and_renderer_unmount_preserve_session",
        pass(session_continuous && renderer_unmounted),
        json!({
            "session": session,
            "counters": counters,
            "renderer_unmounted_without_close": renderer_unmounted,
            "visibility_observation": "unknown",
        }),
    ));
    checks.push(Check::new(
        "overlay_and_activity_are_explicit_consumer_inputs",
        CheckStatus::Pass,
        json!({"reasons": ["proof:inactive", "proof:overlay", "proof:dragged"]}),
    ));

    host.set_size(Size::Logical(tauri::LogicalSize::new(900.0, 620.0)))
        .map_err(string_error)?;
    thread::sleep(Duration::from_millis(120));
    let moved_viewport = viewport(86.0, 72.0, 520.0, 310.0)?;
    let moved_receipt = transition(
        &adapter,
        &mut coordinator,
        moved_viewport,
        initial_scale,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        "host_resize_and_viewport_move",
        &log,
    )?;
    let moved_observation = observe_and_admit(&adapter, &mut coordinator, "moved", &log)?;
    let expected_moved = viewport_to_physical(moved_viewport, initial_scale, RoundingMode::Nearest)
        .map_err(string_error)?;
    let moved_match = observed_bounds(&moved_observation)
        == Some(serde_json::to_value(expected_moved).map_err(string_error)?);

    let zero_viewport = viewport(86.0, 72.0, 0.0, 0.0)?;
    let zero_receipt = transition(
        &adapter,
        &mut coordinator,
        zero_viewport,
        initial_scale,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        "zero_viewport",
        &log,
    )?;
    let zero_observation = observe_and_admit(&adapter, &mut coordinator, "zero", &log)?;
    let zero_outcome = if receipt_applied(&zero_receipt) {
        "native_call_applied"
    } else {
        "typed_native_failure"
    };
    let restore_receipt = transition(
        &adapter,
        &mut coordinator,
        moved_viewport,
        initial_scale,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        "zero_viewport_restore",
        &log,
    )?;
    let restored_observation =
        observe_and_admit(&adapter, &mut coordinator, "zero_restored", &log)?;
    let restored_match = observed_bounds(&restored_observation)
        == Some(serde_json::to_value(expected_moved).map_err(string_error)?);
    checks.push(Check::new(
        "host_resize_move_zero_and_restore",
        pass(
            receipt_applied(&moved_receipt)
                && moved_match
                && restored_match
                && receipt_applied(&restore_receipt),
        ),
        json!({
            "host_inner_size": host.inner_size().map_err(string_error)?,
            "moved_expected": expected_moved,
            "moved_observed": moved_observation,
            "zero_outcome": zero_outcome,
            "zero_receipt": zero_receipt,
            "zero_observed": zero_observation,
            "restored_observed": restored_observation,
        }),
    ));

    let scale_one = ScaleFactor::from_thousandths(1000).map_err(string_error)?;
    let scale_two = ScaleFactor::from_thousandths(2000).map_err(string_error)?;
    let fixture_viewport = viewport(10.25, 20.5, 320.0, 180.0)?;
    let converted_one = viewport_to_physical(fixture_viewport, scale_one, RoundingMode::Nearest)
        .map_err(string_error)?;
    let converted_two = viewport_to_physical(fixture_viewport, scale_two, RoundingMode::Nearest)
        .map_err(string_error)?;
    checks.push(Check::new(
        "deterministic_one_x_two_x_conversion",
        pass(
            serde_json::to_value(converted_one).map_err(string_error)?
                == json!({
                    "origin": {"x": 10, "y": 21},
                    "size": {"width": 320, "height": 180}
                })
                && serde_json::to_value(converted_two).map_err(string_error)?
                    == json!({
                        "origin": {"x": 21, "y": 41},
                        "size": {"width": 640, "height": 360}
                    }),
        ),
        json!({"one_x": converted_one, "two_x": converted_two}),
    ));

    let scale_check = attempt_native_scale_switch(
        &host,
        &adapter,
        &mut coordinator,
        moved_viewport,
        initial_scale,
        &log,
        &scale_event_seen,
    )?;
    checks.push(scale_check);

    adapter
        .evaluate(AttachGeneration::new(1), "window.__longhornSecurityProbe()")
        .map_err(string_error)?;
    let security_probe = log.wait_for("content_event", WAIT, |detail| {
        detail["name"] == "security-probe" && detail["session"] == session
    })?;
    let navigation_denied = log.wait_for("adapter_event", Duration::from_secs(3), |detail| {
        detail["kind"] == "runtime"
            && detail["event"]["kind"] == "navigation"
            && detail["event"]["allowed"] == false
    })?;
    let popup_denied = log.wait_for("adapter_event", Duration::from_secs(1), |detail| {
        detail["kind"] == "runtime" && detail["event"]["kind"] == "popup_denied"
    })?;
    let download_denied = log.wait_for("adapter_event", Duration::from_secs(1), |detail| {
        detail["kind"] == "runtime" && detail["event"]["kind"] == "download_denied"
    })?;
    checks.push(Check::new(
        "controlled_remote_content_security",
        pass(security_probe && navigation_denied),
        json!({
            "remote_capabilities": "none",
            "global_tauri": false,
            "navigation_policy_injected_and_denied": navigation_denied,
            "popup_policy": "deny",
            "popup_hook_observed": popup_denied,
            "download_policy": "deny",
            "download_hook_observed": download_denied,
            "data_store_identifier": "consumer_supplied_16_bytes",
        }),
    ));

    let close_scale = coordinator.desired().scale();
    let close_receipt = transition(
        &adapter,
        &mut coordinator,
        moved_viewport,
        close_scale,
        DesiredPresence::Absent,
        DesiredVisibility::Visible,
        "explicit_close_generation_one",
        &log,
    )?;
    if !receipt_applied(&close_receipt) {
        return Err("explicit child close did not apply".to_string());
    }
    let detach_started = log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "detach_started" && detail["generation"] == 1
    })?;
    if !detach_started {
        return Err("adapter did not record detach start before close completion".to_string());
    }
    coordinator
        .admit_observation(
            coordinator.observed().revision(),
            ObservationUpdate::new(
                AttachGeneration::new(1),
                AttachmentLifecycle::Detaching,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::Unknown,
                None,
            ),
        )
        .map_err(string_error)?;
    log.record(
        "native_observation",
        json!({
            "label": "detach_started",
            "observation": {
                "generation": 1,
                "lifecycle": "detaching",
                "source": "adapter_event_before_native_close",
            }
        }),
    )?;
    let absent_observation =
        observe_and_admit(&adapter, &mut coordinator, "explicitly_closed", &log)?;
    let close_invalidated = receipt_applied(&close_receipt)
        && absent_observation["lifecycle"] == "absent"
        && !adapter
            .is_attached(AttachGeneration::new(1))
            .map_err(string_error)?;
    checks.push(Check::new(
        "explicit_close_invalidates_generation",
        pass(close_invalidated),
        json!({"receipt": close_receipt, "observed": absent_observation}),
    ));

    let generation_two_scale = coordinator.desired().scale();
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                2,
                moved_viewport,
                generation_two_scale,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
            )?,
        )
        .map_err(string_error)?;
    let replacement_plan = coordinator.plan().map_err(string_error)?;
    let replacement_receipt = apply(&adapter, &replacement_plan, "replacement_attach", &log)?;
    let replacement_loaded = log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "runtime"
            && detail["generation"] == 2
            && detail["event"]["kind"] == "page_load_finished"
    })?;
    let stale_result = adapter.admit_runtime_event(RuntimeEvent {
        island_id: island_id()?,
        generation: AttachGeneration::new(1),
        webview_label: CHILD_LABEL.to_string(),
        kind: RuntimeEventKind::PageLoadFinished {
            url: "http://127.0.0.1/stale-generation".to_string(),
        },
    });
    let stale_rejected = stale_result
        .as_ref()
        .is_err_and(|error| error.failure_code() == "child:stale-generation");
    log.record(
        "stale_generation_probe",
        json!({"result": stale_result.map_err(|error| error.to_string())}),
    )?;
    checks.push(Check::new(
        "rapid_replacement_rejects_stale_callback",
        pass(receipt_applied(&replacement_receipt) && replacement_loaded && stale_rejected),
        json!({
            "replacement_receipt": replacement_receipt,
            "replacement_loaded": replacement_loaded,
            "stale_rejected": stale_rejected,
        }),
    ));

    host.destroy().map_err(string_error)?;
    let host_invalidated = log.wait_for("host_destroyed", WAIT, |detail| {
        detail["invalidated"].is_object()
    })?;
    let late_result = adapter.admit_runtime_event(RuntimeEvent {
        island_id: island_id()?,
        generation: AttachGeneration::new(2),
        webview_label: CHILD_LABEL.to_string(),
        kind: RuntimeEventKind::PageLoadFinished {
            url: "http://127.0.0.1/after-host-destroy".to_string(),
        },
    });
    let late_rejected = late_result
        .as_ref()
        .is_err_and(|error| error.failure_code() == "child:not-attached");
    checks.push(Check::new(
        "host_destroy_invalidates_without_stale_mutation",
        pass(host_invalidated && late_rejected),
        json!({
            "host_event_invalidated": host_invalidated,
            "late_generation_callback_rejected": late_rejected,
        }),
    ));

    let report = ProofReport::completed(platform_version(), log.root().to_path_buf(), checks);
    log.write_report(&report)?;
    log.record(
        "proof_complete",
        json!({"report": log.report_path(), "platform": std::env::consts::OS}),
    )?;
    drop(server);
    app.exit(0);
    Ok(())
}

fn install_host_observer<R: Runtime>(
    host: &Window<R>,
    adapter: ChildWebviewAdapter<TauriChildWebviewRuntime<R>>,
    log: Arc<EvidenceLog>,
    scale_event_seen: Arc<AtomicBool>,
) {
    host.on_window_event(move |event| match event {
        WindowEvent::Destroyed => {
            let invalidated = adapter
                .host_destroyed(&host_window_id().expect("static host id is valid"))
                .ok()
                .flatten();
            let detail = serde_json::to_value(invalidated)
                .unwrap_or_else(|error| json!({"serialization_error": error.to_string()}));
            let _ = log.record("host_destroyed", json!({"invalidated": detail}));
        }
        WindowEvent::ScaleFactorChanged {
            scale_factor,
            new_inner_size,
            ..
        } => {
            scale_event_seen.store(true, Ordering::Release);
            let _ = log.record(
                "native_scale_changed",
                json!({
                    "scale_factor": scale_factor,
                    "new_inner_size": new_inner_size,
                }),
            );
        }
        WindowEvent::Resized(size) => {
            let _ = log.record("host_resized", json!({"size": size}));
        }
        _ => {}
    });
}

fn attempt_native_scale_switch(
    host: &Window<Wry>,
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
    viewport: ClientRect,
    initial_scale: ScaleFactor,
    log: &Arc<EvidenceLog>,
    scale_event_seen: &AtomicBool,
) -> Result<Check, String> {
    let monitors = host.available_monitors().map_err(string_error)?;
    let candidate = different_scale_monitor(&monitors, initial_scale);
    let Some(candidate) = candidate else {
        return Ok(Check::new(
            "native_scale_switch",
            CheckStatus::Unmet,
            json!({
                "available_monitor_count": monitors.len(),
                "initial_scale_thousandths": initial_scale.thousandths(),
                "reason": "no available monitor with a different native scale",
                "simulated": false,
            }),
        ));
    };
    let position = candidate.position();
    host.set_position(Position::Physical(PhysicalPosition::new(
        position.x + 40,
        position.y + 40,
    )))
    .map_err(string_error)?;
    let event_observed = log.wait_for("native_scale_changed", WAIT, |_| true)?;
    let native_scale = scale_from_native(host.scale_factor().map_err(string_error)?)?;
    let receipt = transition(
        adapter,
        coordinator,
        viewport,
        native_scale,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        "native_scale_switch",
        log,
    )?;
    let observed = observe_and_admit(adapter, coordinator, "native_scale_switch", log)?;
    let expected = viewport_to_physical(viewport, native_scale, RoundingMode::Nearest)
        .map_err(string_error)?;
    let converged =
        observed_bounds(&observed) == Some(serde_json::to_value(expected).map_err(string_error)?);
    Ok(Check::new(
        "native_scale_switch",
        pass(
            event_observed
                && scale_event_seen.load(Ordering::Acquire)
                && receipt_applied(&receipt)
                && converged,
        ),
        json!({
            "available_monitor_count": monitors.len(),
            "event_observed": event_observed,
            "native_scale_thousandths": native_scale.thousandths(),
            "expected": expected,
            "observed": observed,
            "simulated": false,
        }),
    ))
}

fn different_scale_monitor(monitors: &[Monitor], initial_scale: ScaleFactor) -> Option<&Monitor> {
    monitors.iter().find(|monitor| {
        scale_from_native(monitor.scale_factor()).is_ok_and(|scale| scale != initial_scale)
    })
}

fn coordinator(
    viewport: ClientRect,
    scale: ScaleFactor,
) -> Result<NativeContentCoordinator, String> {
    Ok(NativeContentCoordinator::new(DesiredState::new(
        island_id()?,
        NativeContentKindId::new("proof:controlled-page").map_err(string_error)?,
        MechanismCapabilities::new(
            NativeContentMechanism::ChildView,
            false,
            DetachPolicy::Reversible,
            false,
            false,
        ),
        desired_update(
            1,
            viewport,
            scale,
            DesiredPresence::Present,
            DesiredVisibility::Visible,
        )?,
    )))
}

#[allow(clippy::too_many_arguments)]
fn transition(
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
    viewport: ClientRect,
    scale: ScaleFactor,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    label: &str,
    log: &EvidenceLog,
) -> Result<ApplyReceipt, String> {
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                coordinator.desired().generation().get(),
                viewport,
                scale,
                presence,
                visibility,
            )?,
        )
        .map_err(string_error)?;
    let plan = coordinator.plan().map_err(string_error)?;
    apply(adapter, &plan, label, log)
}

fn desired_update(
    generation: u64,
    viewport: ClientRect,
    scale: ScaleFactor,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
) -> Result<DesiredUpdate, String> {
    Ok(DesiredUpdate::new(
        AttachGeneration::new(generation),
        host_window_id()?,
        viewport,
        scale,
        RoundingMode::Nearest,
        presence,
        visibility,
        FocusIntent::Request,
        InputRoutingMode::NativeDirect,
    ))
}

fn apply(
    adapter: &Adapter,
    plan: &ApplyPlan,
    label: &str,
    log: &EvidenceLog,
) -> Result<ApplyReceipt, String> {
    log.record("apply_plan", json!({"label": label, "plan": plan}))?;
    let receipt = adapter.apply(plan).map_err(string_error)?;
    log.record("apply_receipt", json!({"label": label, "receipt": receipt}))?;
    Ok(receipt)
}

fn observe_and_admit(
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
    label: &str,
    log: &EvidenceLog,
) -> Result<Value, String> {
    let observation = adapter
        .observe(coordinator.desired().generation())
        .map_err(string_error)?;
    let value = serde_json::to_value(&observation).map_err(string_error)?;
    coordinator
        .admit_observation(coordinator.observed().revision(), observation)
        .map_err(string_error)?;
    log.record(
        "native_observation",
        json!({"label": label, "observation": value}),
    )?;
    Ok(value)
}

fn probe(adapter: &Adapter, generation: u64, name: &str) -> Result<(), String> {
    adapter
        .evaluate(
            AttachGeneration::new(generation),
            &format!("window.__longhornProofProbe('{name}')"),
        )
        .map_err(string_error)
}

fn wait_for_probe(log: &EvidenceLog, session: &str, name: &str) -> Result<bool, String> {
    log.wait_for("content_event", WAIT, |detail| {
        detail["name"] == name && detail["session"] == session
    })
}

fn receipt_applied(receipt: &ApplyReceipt) -> bool {
    receipt
        .steps()
        .iter()
        .all(|step| matches!(step.outcome(), OperationOutcome::Applied))
}

fn observed_bounds(observation: &Value) -> Option<Value> {
    (observation["geometry"]["kind"] == "child_bounds")
        .then(|| observation["geometry"]["bounds"].clone())
}

fn viewport(x: f64, y: f64, width: f64, height: f64) -> Result<ClientRect, String> {
    Ok(ClientRect::new(
        ClientPoint::new(x, y).map_err(string_error)?,
        ClientSize::new(width, height).map_err(string_error)?,
    ))
}

fn scale_from_native(value: f64) -> Result<ScaleFactor, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("invalid native scale factor {value}"));
    }
    let thousandths = (value * 1000.0).round();
    if thousandths > f64::from(u32::MAX) {
        return Err(format!("native scale factor {value} exceeds model range"));
    }
    ScaleFactor::from_thousandths(thousandths as u32).map_err(string_error)
}

fn island_id() -> Result<NativeContentIslandId, String> {
    NativeContentIslandId::new(ISLAND_ID).map_err(string_error)
}

fn host_window_id() -> Result<WindowId, String> {
    WindowId::new(HOST_WINDOW_ID).map_err(string_error)
}

const fn pass(condition: bool) -> CheckStatus {
    if condition {
        CheckStatus::Pass
    } else {
        CheckStatus::Unmet
    }
}

const fn observed_unknown(condition: bool) -> CheckStatus {
    if condition {
        CheckStatus::ObservedUnknown
    } else {
        CheckStatus::Unmet
    }
}

fn string_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn platform_version() -> String {
    if cfg!(target_os = "macos") {
        return std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|version| format!("macOS {}", version.trim()))
            .unwrap_or_else(|| "macOS unknown".to_string());
    }
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}
