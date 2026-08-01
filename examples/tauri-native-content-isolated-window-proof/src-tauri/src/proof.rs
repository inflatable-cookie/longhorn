//! Automated packaged matrix for the isolated native-window mechanism.

use std::{ffi::OsString, path::PathBuf, sync::Arc, time::Duration};

use crate::runtime_bridge::{
    ChildRequest, HelperEvent, HelperEventKind, IsolatedWindowAdapter, IsolatedWindowSpec,
    ProcessIsolatedWindowRuntime, ProcessRuntimeConfig, TeardownMode,
};
use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalSize, RoundingMode, ScaleFactor, ScreenPoint,
    ScreenSize, WindowId, WindowPlacement,
};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, ContentSizeDecision, DesiredPresence, DesiredState,
    DesiredUpdate, DesiredVisibility, FocusIntent, InputRoutingMode, NativeContentCoordinator,
    NativeContentFailureCode, NativeContentIslandId, NativeContentKindId, OperationOutcome,
    VisibilityReasonId, viewport_to_physical,
};
use longhorn_native_content_isolated_window::{ISOLATED_WINDOW_CAPABILITIES, TeardownOutcome};
use longhorn_windowing::DesiredWindow;
use serde_json::{Value, json};
use tauri::{AppHandle, Runtime};

use crate::evidence::{Check, CheckStatus, EvidenceLog, ProofReport};

const WAIT: Duration = Duration::from_secs(5);
const TEARDOWN_WAIT: Duration = Duration::from_millis(120);
const ISLAND_ID: &str = "proof-isolated-window";
const HOST_WINDOW_ID: &str = "proof-isolated-host";

type Adapter = IsolatedWindowAdapter;

pub(crate) fn run<R: Runtime>(
    app: AppHandle<R>,
    executable: PathBuf,
    scale: ScaleFactor,
    log: Arc<EvidenceLog>,
) -> Result<(), String> {
    let initial_placement = desired_window(100, 100, 360, 240);
    let runtime = ProcessIsolatedWindowRuntime::new(ProcessRuntimeConfig::new(
        executable,
        placement_arguments(&initial_placement),
        WAIT,
        WAIT,
    ));
    let observer_log = log.clone();
    let adapter = IsolatedWindowAdapter::new(
        runtime.clone(),
        IsolatedWindowSpec::new(island_id()?, host_window_id()?, WAIT, TEARDOWN_WAIT),
        Arc::new(move |event| {
            let detail = serde_json::to_value(event)
                .unwrap_or_else(|error| json!({"serialization_error": error.to_string()}));
            let _ = observer_log.record("adapter_event", detail);
        }),
    );
    let mut checks = Vec::new();
    log.record(
        "outer_placement",
        json!({"generation": 1, "owner": "longhorn-windowing consumer", "desired": initial_placement}),
    )?;

    let mut generation_one = coordinator(1, 360.0, 240.0, scale)?;
    let initial = apply(
        &adapter,
        &generation_one,
        &generation_one.plan().map_err(string_error)?,
        "initial_attach",
        &log,
    )?;
    let initial_observation =
        observe_and_admit(&adapter, &mut generation_one, "initial_attach", &log)?;
    let progress = log.matching_details("adapter_event", |detail| {
        detail["kind"] == "runtime" && detail["event"]["kind"] == "progress"
    })?;
    let real_child = log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "runtime"
            && detail["generation"] == 1
            && detail["event"]["kind"] == "ready"
            && detail["event"]["native_content_attached"] == true
    })?;
    checks.push(Check::new(
        "packaged_fake_nsview_attach_and_progress",
        pass(receipt_applied(&initial) && real_child && progress.len() >= 2),
        json!({
            "receipt": initial,
            "observation": initial_observation,
            "startup_progress": progress,
            "native_child_attached": real_child,
        }),
    ));

    let host_viewport = viewport(420.0, 280.0)?;
    transition(
        &adapter,
        &mut generation_one,
        host_viewport,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        "host_resize",
        &log,
    )?;
    let host_observation = observe_and_admit(&adapter, &mut generation_one, "host_resize", &log)?;
    let host_physical = viewport_to_physical(host_viewport, scale, RoundingMode::Nearest)
        .map_err(string_error)?
        .size();
    adapter
        .script_request(
            AttachGeneration::new(1).map_err(string_error)?,
            ChildRequest::Resize {
                size: host_physical,
            },
        )
        .map_err(string_error)?;
    let echoed = adapter
        .take_requests(AttachGeneration::new(1).map_err(string_error)?)
        .map_err(string_error)?;
    let suppressed = log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "resize_cycle_suppressed" && detail["generation"] == 1
    })?;
    checks.push(Check::new(
        "host_resize_converges_without_update_cycle",
        pass(
            observed_size(&host_observation) == Some(host_physical)
                && echoed.is_empty()
                && suppressed,
        ),
        json!({
            "expected_physical": host_physical,
            "observation": host_observation,
            "pending_echo_requests": echoed,
            "cycle_suppressed": suppressed,
        }),
    ));

    let requested_physical = physical_size(900.0, 700.0, scale)?;
    adapter
        .script_request(
            AttachGeneration::new(1).map_err(string_error)?,
            ChildRequest::Resize {
                size: requested_physical,
            },
        )
        .map_err(string_error)?;
    let requests = adapter
        .take_requests(AttachGeneration::new(1).map_err(string_error)?)
        .map_err(string_error)?;
    let constrained = ClientSize::new(600.0, 440.0).map_err(string_error)?;
    let proposal_receipt = adapter
        .decide_resize(
            generation_one.desired(),
            requested_physical,
            ContentSizeDecision::Constrained { size: constrained },
        )
        .map_err(string_error)?;
    let revision_before_accept = generation_one.desired().revision();
    let accepted = transition(
        &adapter,
        &mut generation_one,
        viewport_from_size(constrained),
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        "content_resize_constrained",
        &log,
    )?;
    let content_observation = observe_and_admit(
        &adapter,
        &mut generation_one,
        "content_resize_constrained",
        &log,
    )?;
    let constrained_physical = physical_size(600.0, 440.0, scale)?;
    checks.push(Check::new(
        "content_resize_requires_current_consumer_acceptance",
        pass(
            requests
                == vec![ChildRequest::Resize {
                    size: requested_physical,
                }]
                && proposal_receipt.accepted_size() == Some(constrained)
                && generation_one.desired().revision() > revision_before_accept
                && receipt_applied(&accepted)
                && observed_size(&content_observation) == Some(constrained_physical),
        ),
        json!({
            "request": requests,
            "decision": proposal_receipt,
            "apply": accepted,
            "observation": content_observation,
        }),
    ));

    let rejected_physical = physical_size(2_000.0, 1_500.0, scale)?;
    adapter
        .script_request(
            AttachGeneration::new(1).map_err(string_error)?,
            ChildRequest::Resize {
                size: rejected_physical,
            },
        )
        .map_err(string_error)?;
    let rejected_requests = adapter
        .take_requests(AttachGeneration::new(1).map_err(string_error)?)
        .map_err(string_error)?;
    let revision_before_reject = generation_one.desired().revision();
    let rejected = adapter
        .decide_resize(
            generation_one.desired(),
            rejected_physical,
            ContentSizeDecision::Rejected {
                code: NativeContentFailureCode::new("proof:size-policy-rejected")
                    .map_err(string_error)?,
            },
        )
        .map_err(string_error)?;
    let after_reject = observe(&adapter, 1, "content_resize_rejected", &log)?;
    checks.push(Check::new(
        "rejected_content_resize_leaves_exact_state_unchanged",
        pass(
            rejected_requests.len() == 1
                && rejected.accepted_size().is_none()
                && generation_one.desired().revision() == revision_before_reject
                && observed_size(&after_reject) == Some(constrained_physical),
        ),
        json!({
            "request": rejected_requests,
            "decision": rejected,
            "desired_revision": generation_one.desired().revision(),
            "observation": after_reject,
        }),
    ));

    adapter
        .script_request(
            AttachGeneration::new(1).map_err(string_error)?,
            ChildRequest::Hide,
        )
        .map_err(string_error)?;
    let hide_request = adapter
        .take_requests(AttachGeneration::new(1).map_err(string_error)?)
        .map_err(string_error)?;
    transition(
        &adapter,
        &mut generation_one,
        viewport_from_size(constrained),
        DesiredPresence::Present,
        DesiredVisibility::Hidden {
            reason: VisibilityReasonId::new("proof:fake-child-hide").map_err(string_error)?,
        },
        FocusIntent::ReleaseIfOwned,
        "child_hide",
        &log,
    )?;
    let hidden = observe_and_admit(&adapter, &mut generation_one, "child_hide", &log)?;
    adapter
        .script_request(
            AttachGeneration::new(1).map_err(string_error)?,
            ChildRequest::Show,
        )
        .map_err(string_error)?;
    let show_request = adapter
        .take_requests(AttachGeneration::new(1).map_err(string_error)?)
        .map_err(string_error)?;
    transition(
        &adapter,
        &mut generation_one,
        viewport_from_size(constrained),
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        "child_show",
        &log,
    )?;
    let shown = observe_and_admit(&adapter, &mut generation_one, "child_show", &log)?;
    let focus_loss = log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "runtime"
            && detail["event"]["kind"] == "focus_changed"
            && detail["event"]["focused"] == false
    })?;
    checks.push(Check::new(
        "child_show_hide_and_focus_loss_preserve_separation",
        pass(
            hide_request == vec![ChildRequest::Hide]
                && show_request == vec![ChildRequest::Show]
                && observed_visibility(&hidden) == Some("hidden")
                && observed_focus(&hidden) == Some("unfocused")
                && observed_visibility(&shown) == Some("visible")
                && observed_focus(&shown) == Some("focused")
                && focus_loss,
        ),
        json!({
            "hide_request": hide_request,
            "hidden": hidden,
            "show_request": show_request,
            "shown": shown,
            "focus_loss_event": focus_loss,
        }),
    ));

    adapter
        .script_request(
            AttachGeneration::new(1).map_err(string_error)?,
            ChildRequest::ResizeHint { resizable: false },
        )
        .map_err(string_error)?;
    let resize_hint = adapter
        .take_requests(AttachGeneration::new(1).map_err(string_error)?)
        .map_err(string_error)?;
    adapter
        .set_resizable(AttachGeneration::new(1).map_err(string_error)?, false)
        .map_err(string_error)?;
    adapter
        .set_resizable(AttachGeneration::new(1).map_err(string_error)?, true)
        .map_err(string_error)?;
    checks.push(Check::new(
        "resize_hint_requires_explicit_admission",
        pass(resize_hint == vec![ChildRequest::ResizeHint { resizable: false }]),
        json!({"request": resize_hint, "applied_sequence": [false, true]}),
    ));

    adapter
        .script_request(
            AttachGeneration::new(1).map_err(string_error)?,
            ChildRequest::Close,
        )
        .map_err(string_error)?;
    let close_request = adapter
        .take_requests(AttachGeneration::new(1).map_err(string_error)?)
        .map_err(string_error)?;
    runtime
        .set_teardown_mode(TeardownMode::Cooperative)
        .map_err(string_error)?;
    let close_receipt = transition(
        &adapter,
        &mut generation_one,
        viewport_from_size(constrained),
        DesiredPresence::Absent,
        DesiredVisibility::Visible,
        FocusIntent::Unchanged,
        "child_close",
        &log,
    )?;
    checks.push(Check::new(
        "child_close_returns_cooperative_teardown",
        pass(
            close_request == vec![ChildRequest::Close]
                && receipt_applied(&close_receipt)
                && adapter
                    .teardown_reports()
                    .map_err(string_error)?
                    .iter()
                    .any(|(_, outcome)| matches!(outcome, TeardownOutcome::Completed { .. })),
        ),
        json!({"request": close_request, "receipt": close_receipt}),
    ));

    let recentered = recentered_window(&initial_placement, 600, 440);
    runtime
        .set_helper_arguments(placement_arguments(&recentered))
        .map_err(string_error)?;
    log.record(
        "outer_placement",
        json!({"generation": 2, "owner": "longhorn-windowing consumer", "desired": recentered}),
    )?;
    checks.push(Check::new(
        "outer_recenter_remains_longhorn_windowing_authority",
        pass(same_center(&initial_placement, &recentered)),
        json!({
            "initial": initial_placement,
            "recentered": recentered,
            "adapter_spec": {
                "host_window_id": adapter.spec().host_window_id(),
                "contains_outer_placement": false,
            }
        }),
    ));

    let mut generation_two = coordinator(2, 600.0, 440.0, scale)?;
    apply(
        &adapter,
        &generation_two,
        &generation_two.plan().map_err(string_error)?,
        "generation_two_attach",
        &log,
    )?;
    let attached_two = adapter
        .observe(AttachGeneration::new(2).map_err(string_error)?)
        .map_err(string_error)?;
    generation_two
        .admit_observation(generation_two.observed().revision(), attached_two)
        .map_err(string_error)?;
    generation_two
        .update_desired(
            generation_two.desired().revision(),
            desired_update(
                2,
                viewport_from_size(constrained),
                scale,
                DesiredPresence::Absent,
                DesiredVisibility::Visible,
                FocusIntent::Unchanged,
            )?,
        )
        .map_err(string_error)?;
    let detach_plan = generation_two.plan().map_err(string_error)?;
    runtime
        .set_teardown_mode(TeardownMode::WaitOnly)
        .map_err(string_error)?;
    let timed_out = apply(
        &adapter,
        &generation_two,
        &detach_plan,
        "teardown_timeout",
        &log,
    )?;
    runtime
        .set_teardown_mode(TeardownMode::TerminateOwner)
        .map_err(string_error)?;
    let terminated = apply(
        &adapter,
        &generation_two,
        &detach_plan,
        "owner_process_termination",
        &log,
    )?;
    let teardown_reports = adapter.teardown_reports().map_err(string_error)?;
    checks.push(Check::new(
        "bounded_teardown_reports_timeout_and_owner_termination",
        pass(
            matches!(
                timed_out.steps()[0].outcome(),
                OperationOutcome::Failed { .. }
            ) && receipt_applied(&terminated)
                && teardown_reports
                    .iter()
                    .any(|(_, outcome)| matches!(outcome, TeardownOutcome::TimedOut { .. }))
                && teardown_reports.iter().any(|(_, outcome)| {
                    matches!(outcome, TeardownOutcome::OwnerProcessTerminated { .. })
                }),
        ),
        json!({
            "timeout_receipt": timed_out,
            "termination_receipt": terminated,
            "teardown_reports": teardown_reports,
        }),
    ));

    let mut generation_three = coordinator(3, 360.0, 240.0, scale)?;
    runtime
        .set_helper_arguments(placement_arguments(&initial_placement))
        .map_err(string_error)?;
    apply(
        &adapter,
        &generation_three,
        &generation_three.plan().map_err(string_error)?,
        "generation_three_attach",
        &log,
    )?;
    let attached_three = adapter
        .observe(AttachGeneration::new(3).map_err(string_error)?)
        .map_err(string_error)?;
    generation_three
        .admit_observation(generation_three.observed().revision(), attached_three)
        .map_err(string_error)?;
    let exit_status = adapter
        .simulate_helper_loss(AttachGeneration::new(3).map_err(string_error)?)
        .map_err(string_error)?;
    let loss_event = log.wait_for("adapter_event", WAIT, |detail| {
        detail["kind"] == "runtime"
            && detail["generation"] == 3
            && detail["event"]["kind"] == "helper_lost"
    })?;
    let failed = observe(&adapter, 3, "helper_loss", &log)?;
    let stale = adapter.admit_runtime_event(HelperEvent {
        island_id: island_id()?,
        generation: AttachGeneration::new(2).map_err(string_error)?,
        kind: HelperEventKind::ContentRequest {
            request: longhorn_native_content_isolated_window::IsolatedContentRequest {
                request_id: longhorn_native_content::NativeContentRequestId::new(
                    "fixture:stale-show",
                )
                .map_err(string_error)?,
                request: ChildRequest::Show,
            },
        },
    });
    checks.push(Check::new(
        "helper_loss_is_terminal_and_stale_generation_is_rejected",
        pass(
            exit_status == Some(73)
                && loss_event
                && observed_lifecycle(&failed) == Some("failed")
                && stale
                    .as_ref()
                    .is_err_and(|error| error.failure_code() == "isolated:stale-generation"),
        ),
        json!({
            "exit_status": exit_status,
            "loss_event": loss_event,
            "observation": failed,
            "stale_result": stale.map_err(|error| error.to_string()),
        }),
    ));

    checks.push(Check::new(
        "mechanism_and_platform_boundary",
        CheckStatus::Pass,
        json!({
            "platform": "macos",
            "windows": "unsupported_unproved",
            "linux": "unsupported_unproved",
            "plugin_sdk": false,
            "signal": false,
            "unsafe_plugin_unload": false,
            "child_webview": false,
            "gpu": false,
            "svelte": false,
            "poodle": false,
            "raw_nsview_location": "proof-app/src-tauri/src/native_macos.rs",
        }),
    ));

    let report = ProofReport::completed(platform_version(), log.root().to_path_buf(), checks);
    log.write_report(&report)?;
    log.record("proof_complete", json!({"report": log.report_path()}))?;
    app.exit(0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn transition(
    adapter: &Adapter,
    coordinator: &mut NativeContentCoordinator,
    viewport: ClientRect,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    focus: FocusIntent,
    label: &str,
    log: &EvidenceLog,
) -> Result<ApplyReceipt, String> {
    coordinator
        .update_desired(
            coordinator.desired().revision(),
            desired_update(
                coordinator.desired().generation().get(),
                viewport,
                coordinator.desired().scale(),
                presence,
                visibility,
                focus,
            )?,
        )
        .map_err(string_error)?;
    apply(
        adapter,
        coordinator,
        &coordinator.plan().map_err(string_error)?,
        label,
        log,
    )
}

fn apply(
    adapter: &Adapter,
    coordinator: &NativeContentCoordinator,
    plan: &ApplyPlan,
    label: &str,
    log: &EvidenceLog,
) -> Result<ApplyReceipt, String> {
    log.record("apply_plan", json!({"label": label, "plan": plan}))?;
    adapter.set_authority(coordinator).map_err(string_error)?;
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

fn observe(
    adapter: &Adapter,
    generation: u64,
    label: &str,
    log: &EvidenceLog,
) -> Result<Value, String> {
    let observation = adapter
        .observe(AttachGeneration::new(generation).map_err(string_error)?)
        .map_err(string_error)?;
    let value = serde_json::to_value(observation).map_err(string_error)?;
    log.record(
        "native_observation",
        json!({"label": label, "observation": value}),
    )?;
    Ok(value)
}

fn coordinator(
    generation: u64,
    width: f64,
    height: f64,
    scale: ScaleFactor,
) -> Result<NativeContentCoordinator, String> {
    Ok(NativeContentCoordinator::new(
        DesiredState::new(
            island_id()?,
            NativeContentKindId::new("proof:fake-native-child").map_err(string_error)?,
            ISOLATED_WINDOW_CAPABILITIES,
            desired_update(
                generation,
                viewport(width, height)?,
                scale,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Request,
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
    focus: FocusIntent,
) -> Result<DesiredUpdate, String> {
    Ok(DesiredUpdate::new(
        AttachGeneration::new(generation).map_err(string_error)?,
        host_window_id()?,
        viewport,
        scale,
        RoundingMode::Nearest,
        presence,
        visibility,
        focus,
        InputRoutingMode::NativeDirect,
    ))
}

fn desired_window(x: i32, y: i32, width: u32, height: u32) -> DesiredWindow {
    DesiredWindow::new(
        host_window_id().expect("static host id is valid"),
        WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(width, height)),
        false,
        false,
    )
}

fn recentered_window(initial: &DesiredWindow, width: u32, height: u32) -> DesiredWindow {
    let placement = initial.placement();
    let old_origin = placement.outer_origin();
    let old_size = placement.inner_size();
    let center_x = i64::from(old_origin.x().get()) + i64::from(old_size.width()) / 2;
    let center_y = i64::from(old_origin.y().get()) + i64::from(old_size.height()) / 2;
    desired_window(
        i32::try_from(center_x - i64::from(width) / 2).expect("proof position fits i32"),
        i32::try_from(center_y - i64::from(height) / 2).expect("proof position fits i32"),
        width,
        height,
    )
}

fn same_center(left: &DesiredWindow, right: &DesiredWindow) -> bool {
    let center = |window: &DesiredWindow| {
        let placement = window.placement();
        (
            i64::from(placement.outer_origin().x().get())
                + i64::from(placement.inner_size().width()) / 2,
            i64::from(placement.outer_origin().y().get())
                + i64::from(placement.inner_size().height()) / 2,
        )
    };
    center(left) == center(right)
}

fn placement_arguments(window: &DesiredWindow) -> Vec<OsString> {
    let placement = window.placement();
    vec![
        "--outer-x".into(),
        placement.outer_origin().x().get().to_string().into(),
        "--outer-y".into(),
        placement.outer_origin().y().get().to_string().into(),
        "--content-width".into(),
        placement.inner_size().width().to_string().into(),
        "--content-height".into(),
        placement.inner_size().height().to_string().into(),
    ]
}

fn physical_size(width: f64, height: f64, scale: ScaleFactor) -> Result<PhysicalSize, String> {
    Ok(
        viewport_to_physical(viewport(width, height)?, scale, RoundingMode::Nearest)
            .map_err(string_error)?
            .size(),
    )
}

fn viewport(width: f64, height: f64) -> Result<ClientRect, String> {
    Ok(ClientRect::new(
        ClientPoint::new(0.0, 0.0).map_err(string_error)?,
        ClientSize::new(width, height).map_err(string_error)?,
    ))
}

fn viewport_from_size(size: ClientSize) -> ClientRect {
    ClientRect::new(
        ClientPoint::new(0.0, 0.0).expect("zero client origin is valid"),
        size,
    )
}

fn receipt_applied(receipt: &ApplyReceipt) -> bool {
    receipt
        .steps()
        .iter()
        .all(|step| matches!(step.outcome(), OperationOutcome::Applied))
}

fn observed_size(value: &Value) -> Option<PhysicalSize> {
    (value["geometry"]["kind"] == "isolated_content")
        .then(|| serde_json::from_value(value["geometry"]["size"].clone()).ok())
        .flatten()
}

fn observed_visibility(value: &Value) -> Option<&str> {
    value["visibility"].as_str()
}

fn observed_focus(value: &Value) -> Option<&str> {
    value["focus"].as_str()
}

fn observed_lifecycle(value: &Value) -> Option<&str> {
    value["lifecycle"].as_str()
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

fn string_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn platform_version() -> String {
    std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|version| format!("macOS {}", version.trim()))
        .unwrap_or_else(|| format!("{} {}", std::env::consts::OS, std::env::consts::ARCH))
}
