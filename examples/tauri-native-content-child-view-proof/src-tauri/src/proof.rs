//! Automated packaged behavior matrix for the production child-view adapter.

use std::{
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use longhorn_core::{ClientPoint, ClientRect, ClientSize, RoundingMode, ScaleFactor, WindowId};
use longhorn_native_content::{
    ApplyReceipt, AttachGeneration, AttachmentLifecycle, DesiredPresence, DesiredState,
    DesiredUpdate, DesiredVisibility, EffectiveFocus, EffectiveVisibility, FocusIntent,
    InputRoutingMode, NativeContentCoordinator, NativeContentIslandId, NativeContentKindId,
    NativeContentOperation, ObservationUpdate, ObservedGeometry, ObservedReadiness,
    OperationOutcome, VisibilityReasonId, viewport_to_physical,
};
use longhorn_tauri_native_content_child_view::{
    CHILD_VIEW_CAPABILITIES, ChildViewAdapter, ChildViewAdapterEvent, ChildViewError,
    ChildViewHostDestroyOutcome, ChildViewLabel, ChildViewRuntimeEvent, ChildViewRuntimeEventKind,
    ChildViewSpec, ChildViewTeardownOutcome, TauriChildViewRuntime,
};
use serde_json::{Value, json};
use tauri::{AppHandle, Position, Window, Wry};

use crate::{
    evidence::{Check, CheckStatus, EvidenceLog, ProofReport},
    server::ProofServer,
};

const WAIT: Duration = Duration::from_secs(10);
const HOST_WINDOW_ID: &str = "window:proof-host";
const ISLAND_ID: &str = "island:child-proof";
const CHILD_LABEL: &str = "proof-child";

type Adapter = ChildViewAdapter<TauriChildViewRuntime<Wry>>;

pub(crate) fn run(
    app: AppHandle<Wry>,
    host: Window<Wry>,
    log: Arc<EvidenceLog>,
) -> Result<(), String> {
    let server = ProofServer::start(log.clone())?;
    let session = format!(
        "production-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let source = tauri::Url::parse(&server.page_url(&session)).map_err(string_error)?;
    let allowed_origin = server.origin().to_string();
    let spec = ChildViewSpec::new(
        island_id()?,
        host_window_id()?,
        ChildViewLabel::new("host").map_err(string_error)?,
        ChildViewLabel::new(CHILD_LABEL).map_err(string_error)?,
        source,
        Some(*b"longhorn-proof-1"),
        Arc::new(move |candidate| candidate.origin().ascii_serialization() == allowed_origin),
    )
    .map_err(string_error)?;
    let event_log = log.clone();
    let adapter = ChildViewAdapter::new(
        TauriChildViewRuntime::new(app.clone()),
        spec,
        Arc::new(move |event: ChildViewAdapterEvent| {
            let detail = serde_json::to_value(event)
                .unwrap_or_else(|error| json!({"serialization_error": error.to_string()}));
            let _ = event_log.record("adapter_event", detail);
        }),
    );

    let mut scale = scale_from_native(host.scale_factor().map_err(string_error)?)?;
    let mut viewport = viewport_rect(24.0, 30.0, 480.0, 280.0)?;
    let mut coordinator = coordinator_for(1, viewport, scale)?;
    let initial_plan = coordinator.plan().map_err(string_error)?;
    let initial_receipt = adapter
        .apply(&coordinator, &initial_plan)
        .map_err(string_error)?;
    log.record(
        "apply_receipt",
        json!({"phase": "initial", "receipt": initial_receipt}),
    )?;
    let loaded = log.wait_for("content_event", WAIT, |detail| {
        detail["name"] == "loaded" && detail["session"] == session
    })?;
    let page_finished = wait_for_runtime(&log, 1, "page_load_finished")?;
    let initial_observation = observe_and_admit(&adapter, &mut coordinator, 1, "initial", &log)?;
    let expected_initial =
        viewport_to_physical(viewport, scale, RoundingMode::Nearest).map_err(string_error)?;
    let mut checks = vec![
        Check::new(
            "packaged_child_creation_and_readiness",
            pass(loaded && page_finished && receipt_applied(&initial_receipt)),
            json!({"loaded": loaded, "page_finished": page_finished, "receipt": initial_receipt}),
        ),
        Check::new(
            "fresh_physical_bounds_converge",
            pass(
                observed_bounds(&initial_observation)
                    == serde_json::to_value(expected_initial).map_err(string_error)?,
            ),
            json!({"scale_thousandths": scale.thousandths(), "expected": expected_initial, "observed": initial_observation}),
        ),
        Check::new(
            "focus_and_visibility_observation_remain_unknown",
            observed_unknown(
                initial_observation["focus"] == "unknown"
                    && initial_observation["visibility"] == "unknown",
            ),
            json!({
                "focus_request_applied": initial_plan.operations().iter().any(|step| matches!(step.operation(), NativeContentOperation::RequestFocus)),
                "focus": initial_observation["focus"],
                "visibility": initial_observation["visibility"],
            }),
        ),
    ];

    thread::sleep(Duration::from_millis(400));
    let denied_popup = log.count("http_request", |detail| detail["path"] == "/popup")? == 0;
    let download_requests = log.count("http_request", |detail| detail["path"] == "/download")?;
    let same_origin = adapter.spec().allows_navigation(
        &tauri::Url::parse(&format!("{}/allowed", server.origin())).map_err(string_error)?,
    );
    let cross_origin = adapter.spec().allows_navigation(
        &tauri::Url::parse("https://example.invalid/blocked").map_err(string_error)?,
    );
    checks.push(Check::new(
        "closed_browser_policy_and_remote_capability_posture",
        pass(denied_popup && download_requests == 1 && same_origin && !cross_origin),
        json!({
            "popup_request_reached_server": !denied_popup,
            "download_response_requested_before_native_persistence_denial": download_requests == 1,
            "download_persistence_policy": "deny",
            "same_origin_allowed": same_origin,
            "cross_origin_allowed": cross_origin,
            "remote_capabilities": [],
        }),
    ));

    let heartbeat_before = heartbeat_count(&log, &session)?;
    transition(
        &adapter,
        &mut coordinator,
        1,
        viewport,
        scale,
        DesiredPresence::Present,
        DesiredVisibility::Hidden {
            reason: VisibilityReasonId::new("proof:inactive").map_err(string_error)?,
        },
        "hide",
        &log,
    )?;
    adapter
        .renderer_unmounted(generation(1)?)
        .map_err(string_error)?;
    thread::sleep(Duration::from_millis(350));
    transition(
        &adapter,
        &mut coordinator,
        1,
        viewport,
        scale,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        "show",
        &log,
    )?;
    thread::sleep(Duration::from_millis(350));
    let heartbeat_after = heartbeat_count(&log, &session)?;
    let page_loads = log.count("http_request", |detail| detail["path"] == "/proof")?;
    checks.push(Check::new(
        "hide_show_and_renderer_unmount_reuse_one_child",
        pass(heartbeat_after > heartbeat_before && page_loads == 1 && adapter.is_attached(generation(1)?).map_err(string_error)?),
        json!({"heartbeat_before": heartbeat_before, "heartbeat_after": heartbeat_after, "page_loads": page_loads}),
    ));

    viewport = viewport_rect(86.0, 72.0, 520.0, 310.0)?;
    let moved_observation = transition(
        &adapter,
        &mut coordinator,
        1,
        viewport,
        scale,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        "viewport_move",
        &log,
    )?;
    let expected_moved =
        viewport_to_physical(viewport, scale, RoundingMode::Nearest).map_err(string_error)?;
    checks.push(Check::new(
        "explicit_scale_and_rounding_drive_moved_bounds",
        pass(
            observed_bounds(&moved_observation)
                == serde_json::to_value(expected_moved).map_err(string_error)?,
        ),
        json!({"expected": expected_moved, "observed": moved_observation}),
    ));

    let fixture = viewport_rect(10.25, 20.5, 320.0, 180.0)?;
    let one = viewport_to_physical(
        fixture,
        ScaleFactor::from_thousandths(1000).map_err(string_error)?,
        RoundingMode::Nearest,
    )
    .map_err(string_error)?;
    let two = viewport_to_physical(
        fixture,
        ScaleFactor::from_thousandths(2000).map_err(string_error)?,
        RoundingMode::Nearest,
    )
    .map_err(string_error)?;
    checks.push(Check::new(
        "deterministic_one_x_two_x_conversion",
        pass(
            serde_json::to_value(one).map_err(string_error)?
                == json!({"origin":{"x":10,"y":21},"size":{"width":320,"height":180}})
                && serde_json::to_value(two).map_err(string_error)?
                    == json!({"origin":{"x":21,"y":41},"size":{"width":640,"height":360}}),
        ),
        json!({"one_x": one, "two_x": two}),
    ));

    let (scale_check, resulting_scale, resulting_observation) =
        attempt_native_scale_transition(&host, &adapter, &mut coordinator, viewport, scale, &log)?;
    scale = resulting_scale;
    if let Some(observation) = resulting_observation {
        log.record("native_scale_observation", observation)?;
    }
    checks.push(scale_check);

    let detached = transition(
        &adapter,
        &mut coordinator,
        1,
        viewport,
        scale,
        DesiredPresence::Absent,
        DesiredVisibility::Visible,
        "generation_one_close",
        &log,
    )?;
    let first_absent = detached["lifecycle"] == "absent"
        && !adapter.is_attached(generation(1)?).map_err(string_error)?;

    let mut second = coordinator_for(2, viewport, scale)?;
    let second_plan = second.plan().map_err(string_error)?;
    let second_receipt = adapter.apply(&second, &second_plan).map_err(string_error)?;
    wait_for_runtime(&log, 2, "page_load_finished")?;
    observe_and_admit(&adapter, &mut second, 2, "replacement", &log)?;
    let teardown = adapter.teardown().map_err(string_error)?;

    let mut third = coordinator_for(3, viewport, scale)?;
    let third_plan = third.plan().map_err(string_error)?;
    let third_receipt = adapter.apply(&third, &third_plan).map_err(string_error)?;
    wait_for_runtime(&log, 3, "page_load_finished")?;
    observe_and_admit(&adapter, &mut third, 3, "before_host_destroy", &log)?;
    host.destroy().map_err(string_error)?;
    let invalidated = adapter
        .host_destroyed(&host_window_id()?, generation(3)?)
        .map_err(string_error)?;
    let late = adapter.admit_runtime_event(ChildViewRuntimeEvent {
        island_id: island_id()?,
        generation: generation(3)?,
        child_label: ChildViewLabel::new(CHILD_LABEL).map_err(string_error)?,
        kind: ChildViewRuntimeEventKind::PageLoadFinished,
    });
    checks.push(Check::new(
        "close_replacement_teardown_and_host_destroy_are_exact",
        pass(
            first_absent
                && receipt_applied(&second_receipt)
                && teardown.outcome() == ChildViewTeardownOutcome::Closed
                && receipt_applied(&third_receipt)
                && invalidated.outcome() == ChildViewHostDestroyOutcome::Invalidated
                && matches!(late, Err(ChildViewError::GenerationRetired(_))),
        ),
        json!({
            "generation_one_absent": first_absent,
            "replacement_receipt": second_receipt,
            "teardown": teardown,
            "host_destroy": invalidated,
            "late_callback": format!("{late:?}"),
        }),
    ));

    let report = ProofReport::completed(platform_description(), log.root().to_path_buf(), checks);
    log.write_report(&report)?;
    log.record("proof_completed", json!({"report": log.report_path()}))?;
    app.exit(0);
    drop(server);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn transition(
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
    generation: u64,
    viewport: ClientRect,
    scale: ScaleFactor,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    phase: &str,
    log: &EvidenceLog,
) -> Result<Value, String> {
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(generation, viewport, scale, presence, visibility)?,
        )
        .map_err(string_error)?;
    let plan = coordinator.plan().map_err(string_error)?;
    let receipt = adapter.apply(coordinator, &plan).map_err(string_error)?;
    log.record("apply_receipt", json!({"phase": phase, "receipt": receipt}))?;
    if presence == DesiredPresence::Absent {
        let detach_started = log.wait_for("adapter_event", WAIT, |detail| {
            detail["kind"] == "detach_started" && detail["generation"] == generation
        })?;
        if !detach_started {
            return Err("adapter did not record detach start".to_string());
        }
        coordinator
            .admit_observation(
                coordinator.observed().revision(),
                ObservationUpdate::new(
                    self::generation(generation)?,
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
            "observation",
            json!({
                "phase": format!("{phase}_detach_started"),
                "observation": {
                    "generation": generation,
                    "lifecycle": "detaching",
                    "source": "adapter_event_before_native_close"
                }
            }),
        )?;
    }
    observe_and_admit(adapter, coordinator, generation, phase, log)
}

fn observe_and_admit(
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
    generation: u64,
    phase: &str,
    log: &EvidenceLog,
) -> Result<Value, String> {
    let observation = adapter
        .observe(self::generation(generation)?)
        .map_err(string_error)?;
    let value = serde_json::to_value(&observation).map_err(string_error)?;
    coordinator
        .admit_observation(coordinator.observed().revision(), observation)
        .map_err(string_error)?;
    log.record("observation", json!({"phase": phase, "observation": value}))?;
    Ok(value)
}

fn coordinator_for(
    generation: u64,
    viewport: ClientRect,
    scale: ScaleFactor,
) -> Result<NativeContentCoordinator, String> {
    Ok(NativeContentCoordinator::new(
        DesiredState::new(
            island_id()?,
            NativeContentKindId::new("proof:controlled-page").map_err(string_error)?,
            CHILD_VIEW_CAPABILITIES,
            desired_update(
                generation,
                viewport,
                scale,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
            )?,
        )
        .map_err(string_error)?,
    ))
}

fn desired_update(
    generation: u64,
    viewport: ClientRect,
    scale: ScaleFactor,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
) -> Result<DesiredUpdate, String> {
    Ok(DesiredUpdate::new(
        self::generation(generation)?,
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

fn attempt_native_scale_transition(
    host: &Window<Wry>,
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
    viewport: ClientRect,
    initial: ScaleFactor,
    log: &EvidenceLog,
) -> Result<(Check, ScaleFactor, Option<Value>), String> {
    let monitors = host.available_monitors().map_err(string_error)?;
    let scales = monitors
        .iter()
        .map(|monitor| monitor.scale_factor())
        .collect::<Vec<_>>();
    let Some(target) = monitors.iter().find(|monitor| {
        scale_from_native(monitor.scale_factor()).is_ok_and(|scale| scale != initial)
    }) else {
        return Ok((
            Check::new(
                "native_scale_switch",
                CheckStatus::Unmet,
                json!({"reason": "no monitor with a distinct scale was attached", "available_scales": scales}),
            ),
            initial,
            None,
        ));
    };
    let position = target.position();
    host.set_position(Position::Physical(tauri::PhysicalPosition::new(
        position.x + 40,
        position.y + 40,
    )))
    .map_err(string_error)?;
    let mut changed = None;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        let candidate = scale_from_native(host.scale_factor().map_err(string_error)?)?;
        if candidate != initial {
            changed = Some(candidate);
            break;
        }
    }
    let Some(changed) = changed else {
        return Ok((
            Check::new(
                "native_scale_switch",
                CheckStatus::Unmet,
                json!({"reason": "host did not report the distinct target scale", "available_scales": scales}),
            ),
            initial,
            None,
        ));
    };
    let observation = transition(
        adapter,
        coordinator,
        1,
        viewport,
        changed,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        "native_scale_switch",
        log,
    )?;
    let expected =
        viewport_to_physical(viewport, changed, RoundingMode::Nearest).map_err(string_error)?;
    let matches =
        observed_bounds(&observation) == serde_json::to_value(expected).map_err(string_error)?;
    Ok((
        Check::new(
            "native_scale_switch",
            pass(matches),
            json!({"from": initial.thousandths(), "to": changed.thousandths(), "expected": expected, "observed": observation}),
        ),
        changed,
        Some(observation),
    ))
}

fn wait_for_runtime(log: &EvidenceLog, generation: u64, kind: &str) -> Result<bool, String> {
    log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "runtime" && detail["generation"] == generation && detail["event"] == kind
    })
}

fn heartbeat_count(log: &EvidenceLog, session: &str) -> Result<usize, String> {
    log.count("content_event", |detail| {
        detail["name"] == "heartbeat" && detail["session"] == session
    })
}

fn receipt_applied(receipt: &ApplyReceipt) -> bool {
    receipt
        .steps()
        .iter()
        .all(|step| step.outcome() == &OperationOutcome::Applied)
}

fn observed_bounds(observation: &Value) -> Value {
    observation["geometry"]["bounds"].clone()
}

fn viewport_rect(x: f64, y: f64, width: f64, height: f64) -> Result<ClientRect, String> {
    Ok(ClientRect::new(
        ClientPoint::new(x, y).map_err(string_error)?,
        ClientSize::new(width, height).map_err(string_error)?,
    ))
}

fn generation(value: u64) -> Result<AttachGeneration, String> {
    AttachGeneration::new(value).map_err(string_error)
}

fn island_id() -> Result<NativeContentIslandId, String> {
    NativeContentIslandId::new(ISLAND_ID).map_err(string_error)
}

fn host_window_id() -> Result<WindowId, String> {
    WindowId::new(HOST_WINDOW_ID).map_err(string_error)
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

fn platform_description() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
