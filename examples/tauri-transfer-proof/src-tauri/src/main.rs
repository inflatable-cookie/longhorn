//! Packaged multi-window proof for Longhorn transfer handlers.

mod domain;
mod evidence;
mod host;
mod matrix;
#[cfg(feature = "surface-mode")]
mod surface;

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use domain::{
    LAYOUT_DOMAIN_ID, MAIN_REGION_ID, ProofClock, ProofDomains, SOURCE_BINDING_ID,
    SOURCE_CONTAINER_ID, SOURCE_PANEL_ID, TARGET_BINDING_ID, TARGET_CONTAINER_ID, binding_kind,
};
use evidence::EvidenceLog;
#[cfg(feature = "surface-mode")]
use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferCommand,
    SurfaceTransferResponse,
};
use longhorn_tauri_transfer::TauriTransferState;
#[cfg(feature = "surface-mode")]
use longhorn_tauri_transfer::{
    ManagedTransferRuntime, TauriSurfaceTransferState, TauriTransferRuntime,
};
#[cfg(feature = "surface-mode")]
use longhorn_tauri_windowing::UniformScaleMapper;
use longhorn_transfer::{
    PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse, TransferCancelRequest,
    TransferCancelResponse, TransferClientSnapshot, TransferLeaseRequest, TransferLeaseResponse,
    TransferSessionResponse,
};
#[cfg(feature = "surface-mode")]
use longhorn_windowing::HostWindowHandle;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager, State, WebviewWindow, Wry};

const TAURI_VERSION: &str = "2.10.3";

struct ProofState {
    host: host::ProofHost,
    domains: Arc<ProofDomains>,
    evidence: Arc<EvidenceLog>,
    target_ready: Mutex<Option<Value>>,
    autorun: bool,
}

#[tauri::command]
fn longhorn_transfer_snapshot(
    window: WebviewWindow<Wry>,
    state: State<'_, TauriTransferState>,
) -> Result<TransferClientSnapshot, String> {
    longhorn_tauri_transfer::longhorn_transfer_snapshot(window, state)
}

#[tauri::command]
fn longhorn_transfer_start_panel(
    window: WebviewWindow<Wry>,
    state: State<'_, TauriTransferState>,
    request: PanelSessionStartRequest,
) -> Result<TransferSessionResponse, String> {
    longhorn_tauri_transfer::longhorn_transfer_start_panel(window, state, request)
}

#[tauri::command]
fn longhorn_transfer_publish_lease(
    window: WebviewWindow<Wry>,
    state: State<'_, TauriTransferState>,
    request: TransferLeaseRequest,
) -> Result<TransferLeaseResponse, String> {
    longhorn_tauri_transfer::longhorn_transfer_publish_lease(window, state, request)
}

#[tauri::command]
fn longhorn_transfer_commit_panel(
    window: WebviewWindow<Wry>,
    state: State<'_, TauriTransferState>,
    request: PanelTransferCommand,
) -> Result<PanelTransferResponse, String> {
    longhorn_tauri_transfer::longhorn_transfer_commit_panel(window, state, request)
}

#[tauri::command]
fn longhorn_transfer_cancel(
    window: WebviewWindow<Wry>,
    state: State<'_, TauriTransferState>,
    request: TransferCancelRequest,
) -> Result<TransferCancelResponse, String> {
    longhorn_tauri_transfer::longhorn_transfer_cancel(window, state, request)
}

#[cfg(feature = "surface-mode")]
#[tauri::command]
fn longhorn_transfer_start_surface(
    window: WebviewWindow<Wry>,
    state: State<'_, TauriSurfaceTransferState>,
    request: SurfaceSessionStartRequest,
) -> Result<SurfaceSessionResponse, String> {
    longhorn_tauri_transfer::longhorn_transfer_start_surface(window, state, request)
}

#[cfg(feature = "surface-mode")]
#[tauri::command]
fn longhorn_transfer_commit_surface(
    window: WebviewWindow<Wry>,
    state: State<'_, TauriSurfaceTransferState>,
    request: SurfaceTransferCommand,
) -> Result<SurfaceTransferResponse, String> {
    longhorn_tauri_transfer::longhorn_transfer_commit_surface(window, state, request)
}

#[tauri::command]
fn proof_bootstrap(state: State<'_, ProofState>) -> Result<Value, String> {
    let layout = state.domains.layout_snapshot()?;
    let bootstrap = json!({
        "mode": proof_mode(),
        "protocol_version": 1,
        "layout": {
            "domain_id": LAYOUT_DOMAIN_ID,
            "revision": layout.revision().get(),
            "source_panel_id": SOURCE_PANEL_ID,
            "source_surface_id": SOURCE_CONTAINER_ID,
            "target_surface_id": TARGET_CONTAINER_ID,
            "target_region_id": MAIN_REGION_ID,
            "source_binding_id": SOURCE_BINDING_ID,
            "target_binding_id": TARGET_BINDING_ID,
            "binding_kind": format!("{:?}", binding_kind()),
        },
        "paths": {
            "root": state.evidence.root(),
            "report": state.evidence.report_path(),
        },
    });
    #[cfg(feature = "surface-mode")]
    let bootstrap = {
        let mut bootstrap = bootstrap;
        let surface = state.domains.surface_snapshot()?;
        bootstrap["surface"] = json!({
            "domain_id": surface::SURFACE_DOMAIN_ID,
            "revision": surface.revision().get(),
            "source_surface_id": surface::SOURCE_SURFACE_ID,
            "second_surface_id": surface::SECOND_SURFACE_ID,
            "target_binding_id": TARGET_BINDING_ID,
            "empty_drop_point": state.host.screen_policy.drop_point(),
            "display_bounds": state.host.screen_policy.display_bounds(),
            "provisioned_placement": state.host.screen_policy.placement(),
            "provisioned_window_id": surface::PROVISIONED_WINDOW_ID,
        });
        bootstrap
    };
    Ok(bootstrap)
}

#[tauri::command]
fn proof_target_ready(state: State<'_, ProofState>, evidence: Value) -> Result<(), String> {
    state.evidence.record("target_ready", evidence.clone())?;
    *state
        .target_ready
        .lock()
        .map_err(|_| "target-ready lock is poisoned".to_string())? = Some(evidence);
    Ok(())
}

#[tauri::command]
fn proof_target_status(state: State<'_, ProofState>) -> Result<Option<Value>, String> {
    state
        .target_ready
        .lock()
        .map_err(|_| "target-ready lock is poisoned".to_string())
        .map(|value| value.clone())
}

#[tauri::command]
fn proof_run_matrix(app: AppHandle<Wry>, state: State<'_, ProofState>) -> Result<Value, String> {
    let evidence = matrix::run(&app, &state.host, state.evidence.root())?;
    state.evidence.record("transfer_matrix", evidence.clone())?;
    Ok(evidence)
}

#[tauri::command]
fn proof_complete(
    app: AppHandle<Wry>,
    state: State<'_, ProofState>,
    renderer_evidence: Value,
) -> Result<Value, String> {
    let layout = state.domains.layout_snapshot()?;
    let committed = renderer_evidence
        .pointer("/panel_commit/status")
        .and_then(Value::as_str)
        == Some("committed");
    let revision_advanced = layout.revision().get() == 8;
    let matrix_passed = renderer_evidence
        .pointer("/matrix/result")
        .and_then(Value::as_str)
        == Some("passed");
    let (surface_report, surface_passed) = surface_closeout(&app, &state, &renderer_evidence)?;
    let passed = committed && revision_advanced && matrix_passed && surface_passed;
    let report = json!({
        "schema_version": 1,
        "result": if passed { "passed" } else { "failed" },
        "mode": proof_mode(),
        "artifact": {
            "application": "Longhorn Transfer Proof",
            "longhorn": env!("CARGO_PKG_VERSION"),
            "tauri": TAURI_VERSION,
            "rustc": env!("PROOF_RUSTC_VERSION"),
            "arch": std::env::consts::ARCH,
            "os": std::env::consts::OS,
        },
        "checks": {
            "real_source_and_target_renderer_snapshots": renderer_evidence.get("source_snapshot").is_some()
                && renderer_evidence.get("target_snapshot").is_some(),
            "target_renderer_published_lease": renderer_evidence.pointer("/target_lease/status").and_then(Value::as_str) == Some("published"),
            "source_renderer_started_session": renderer_evidence.pointer("/panel_start/status").and_then(Value::as_str) == Some("started"),
            "panel_commit_succeeded": committed,
            "registered_layout_revision_advanced_once": revision_advanced,
            "failure_and_geometry_matrix_passed": matrix_passed,
            "surface_mode_passed": surface_passed,
        },
        "renderer_evidence": renderer_evidence,
        "authoritative_layout": layout,
        "surface": surface_report,
    });
    state.evidence.record("proof_complete", report.clone())?;
    state.evidence.write_report(&report)?;
    if state.autorun {
        app.exit(if passed { 0 } else { 1 });
    }
    Ok(report)
}

#[tauri::command]
fn proof_failed(
    app: AppHandle<Wry>,
    state: State<'_, ProofState>,
    detail: String,
) -> Result<(), String> {
    let report = json!({
        "schema_version": 1,
        "result": "failed",
        "mode": proof_mode(),
        "detail": detail,
    });
    state.evidence.record("proof_failed", report.clone())?;
    state.evidence.write_report(&report)?;
    if state.autorun {
        app.exit(1);
    }
    Ok(())
}

#[tauri::command]
fn proof_status(state: State<'_, ProofState>) -> Result<Value, String> {
    Ok(json!({
        "mode": proof_mode(),
        "layout": state.domains.layout_snapshot()?,
        "target_ready": state.target_ready.lock().map_err(|_| "target-ready lock is poisoned".to_string())?.clone(),
        "report_path": state.evidence.report_path(),
    }))
}

fn setup(app: &mut tauri::App<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let output = proof_output(app)?;
    let evidence = Arc::new(EvidenceLog::new(output.clone())?);
    let domains = Arc::new(ProofDomains::new(&output.join("domains"), binding_kind())?);
    let assembled = host::assemble(app.handle(), domains.clone(), ProofClock::new())?;
    app.manage(TauriTransferState::new(assembled.transfer.clone()));
    #[cfg(feature = "surface-mode")]
    app.manage(TauriSurfaceTransferState::new(
        assembled.surface_transfer.clone(),
    ));
    evidence.record(
        "proof_initialized",
        json!({
            "mode": proof_mode(),
            "output": output,
            "managed_windows": ["source", "target"],
        }),
    )?;
    app.manage(ProofState {
        host: assembled,
        domains,
        evidence,
        target_ready: Mutex::new(None),
        autorun: std::env::var_os("LONGHORN_TRANSFER_PROOF_AUTORUN").is_some(),
    });
    Ok(())
}

#[cfg(not(feature = "surface-mode"))]
fn surface_closeout(
    _app: &AppHandle<Wry>,
    _state: &ProofState,
    _renderer_evidence: &Value,
) -> Result<(Value, bool), String> {
    Ok((Value::Null, true))
}

#[cfg(feature = "surface-mode")]
fn surface_closeout(
    app: &AppHandle<Wry>,
    state: &ProofState,
    renderer_evidence: &Value,
) -> Result<(Value, bool), String> {
    let document = state.domains.surface_snapshot()?;
    let existing_committed = renderer_evidence
        .pointer("/surface_existing_commit/status")
        .and_then(Value::as_str)
        == Some("committed");
    let provisioned_committed = renderer_evidence
        .pointer("/surface_provisioned_commit/status")
        .and_then(Value::as_str)
        == Some("committed");
    let source = document
        .surface(
            &longhorn_core::SurfaceId::new(surface::SOURCE_SURFACE_ID)
                .expect("proof Surface id is valid"),
        )
        .ok_or_else(|| "source Surface disappeared".to_string())?;
    let second = document
        .surface(
            &longhorn_core::SurfaceId::new(surface::SECOND_SURFACE_ID)
                .expect("proof Surface id is valid"),
        )
        .ok_or_else(|| "second Surface disappeared".to_string())?;
    let binding_retained =
        source.id().as_str() == SOURCE_CONTAINER_ID && second.id().as_str() == TARGET_CONTAINER_ID;
    let provisioned_window = app
        .get_webview_window(surface::PROVISIONED_WINDOW_ID)
        .ok_or_else(|| "provisioned Tauri window disappeared before closeout".to_string())?;
    let visible = provisioned_window
        .is_visible()
        .map_err(|error| error.to_string())?;
    let runtime = TauriTransferRuntime::new(state.host.window_host.clone(), UniformScaleMapper);
    let runtime_snapshot = runtime
        .snapshot(
            &HostWindowHandle::new(domain::SOURCE_WINDOW_ID)
                .expect("proof Tauri label uses the opaque-id grammar"),
        )
        .map_err(|error| error.to_string())?;
    let provisioned_geometry = runtime_snapshot
        .windows()
        .iter()
        .find(|window| window.window_id().as_str() == surface::PROVISIONED_WINDOW_ID)
        .ok_or_else(|| "provisioned window is absent from managed runtime readback".to_string())?;
    let desired = state.host.screen_policy.placement();
    let placement_matches = provisioned_geometry.outer_bounds().origin() == desired.outer_origin()
        && provisioned_geometry.content_bounds().size() == desired.inner_size();
    let passed = existing_committed
        && provisioned_committed
        && document.revision().get() == 9
        && binding_retained
        && visible
        && placement_matches;
    Ok((
        json!({
            "existing_commit_succeeded": existing_committed,
            "provisioned_commit_succeeded": provisioned_committed,
            "revision_advanced_twice": document.revision().get() == 9,
            "layout_bindings_retained": binding_retained,
            "provisioned_window": {
                "visible_after_commit": visible,
                "managed_window_count_after_provision": runtime_snapshot.windows().len(),
                "outer_bounds": provisioned_geometry.outer_bounds(),
                "content_bounds": provisioned_geometry.content_bounds(),
                "desired_placement": desired,
                "placement_matches": placement_matches,
            },
            "authoritative_document": document,
        }),
        passed,
    ))
}

fn proof_output(app: &tauri::App<Wry>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("LONGHORN_TRANSFER_PROOF_OUTPUT") {
        let path = PathBuf::from(path);
        fs::create_dir_all(&path)?;
        return Ok(path);
    }
    let run = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok(app
        .path()
        .app_data_dir()?
        .join("runs")
        .join(run.to_string()))
}

const fn proof_mode() -> &'static str {
    if cfg!(feature = "surface-mode") {
        "surface"
    } else {
        "direct"
    }
}

fn main() {
    let builder = tauri::Builder::default();
    #[cfg(not(feature = "surface-mode"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        longhorn_transfer_snapshot,
        longhorn_transfer_start_panel,
        longhorn_transfer_publish_lease,
        longhorn_transfer_commit_panel,
        longhorn_transfer_cancel,
        proof_bootstrap,
        proof_target_ready,
        proof_target_status,
        proof_run_matrix,
        proof_complete,
        proof_failed,
        proof_status,
    ]);
    #[cfg(feature = "surface-mode")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        longhorn_transfer_snapshot,
        longhorn_transfer_start_panel,
        longhorn_transfer_publish_lease,
        longhorn_transfer_commit_panel,
        longhorn_transfer_cancel,
        longhorn_transfer_start_surface,
        longhorn_transfer_commit_surface,
        proof_bootstrap,
        proof_target_ready,
        proof_target_status,
        proof_run_matrix,
        proof_complete,
        proof_failed,
        proof_status,
    ]);
    let app = builder
        .setup(setup)
        .build(tauri::generate_context!())
        .expect("could not build packaged Longhorn transfer proof");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. })
            && let Some(state) = app.try_state::<ProofState>()
        {
            let transfer = state.host.transfer.clone();
            drop(transfer);
            let _ = state.host.window_host.teardown();
        }
    });
}
