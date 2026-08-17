//! Packaged native proof for the public Longhorn Tauri window host.

use std::{
    collections::BTreeMap,
    convert::Infallible,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use longhorn_core::{DisplayId, ScreenPoint, ScreenSize, WindowId, WindowPlacement};
use longhorn_display::{
    DisplayIdAllocator, KnownDisplayRegistry, ObservedDisplay, reconcile_displays,
};
use longhorn_tauri_windowing::{
    ApplyConvergence, ApplyReadback, CapturedDisplayAssociation, CapturedWindowPlacement,
    DefaultDisplayMetadata, ManagedWebviewWindow, PredeclaredTauriWindow, ProcessMonotonicClock,
    ScheduledWindowLifecycleWake, TauriDesktopReadback, TauriWindowCaptureBackend, TauriWindowHost,
    TauriWindowLifecycleServices, TauriWindowMutationBackend, TauriWindowRevealBackend,
    UniformWindowGeometryMapper, WindowFlushRequest, WindowLifecycleClock, WindowLifecycleReport,
    WindowLifecycleScheduler, WindowPlacementFlushCompletion, WindowPlacementFlushTicket,
    WindowPlacementSink, WindowRevealReceipt, WindowShutdownReceipt, WindowUserCloseHandler,
    assemble_tauri_window_host, observe_tauri_desktop, scale_factor_from_tauri,
};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostWindowHandle, PlacementPolicy, ProtectedPrimaryPolicy,
    WindowDiffInput, WindowLifecycleDuration, WindowLifecycleEvent, WindowLifecyclePolicy,
    WindowPlacementConfig, WindowPlacementResolution, WindowRole, resolve_window_placement,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{
    AppHandle, Manager, Runtime, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry,
};

const MAIN_LABEL: &str = "main";
const WORKSPACE_LABEL: &str = "workspace";
const MISSING_DISPLAY_ID: &str = "proof-display:missing";
const TAURI_VERSION: &str = "2.10.3";

type ProofHost = TauriWindowHost<Wry>;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PersistedProofState {
    schema_version: u32,
    placements: BTreeMap<String, CapturedWindowPlacement>,
    restart_scenario: Option<String>,
}

impl Default for PersistedProofState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            placements: BTreeMap::new(),
            restart_scenario: None,
        }
    }
}

struct EvidenceLog {
    path: PathBuf,
    sequence: AtomicU64,
    write_lock: Mutex<()>,
}

impl EvidenceLog {
    fn new(path: PathBuf) -> Result<Self, String> {
        let parent = path
            .parent()
            .ok_or_else(|| "evidence path has no parent".to_string())?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        Ok(Self {
            path,
            sequence: AtomicU64::new(0),
            write_lock: Mutex::new(()),
        })
    }

    fn record(&self, event: &str, detail: Value) -> Result<(), String> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "evidence log lock is poisoned".to_string())?;
        let envelope = json!({
            "sequence": self.sequence.fetch_add(1, Ordering::Relaxed) + 1,
            "unix_millis": unix_millis(),
            "event": event,
            "detail": detail,
        });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut file, &envelope).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())
    }
}

struct ProofSink {
    path: PathBuf,
    state: Mutex<PersistedProofState>,
    log: Arc<EvidenceLog>,
}

impl ProofSink {
    fn load(path: PathBuf, log: Arc<EvidenceLog>) -> Result<Self, String> {
        let state = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                format!(
                    "could not decode persisted proof state at {}: {error}",
                    path.display()
                )
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                PersistedProofState::default()
            }
            Err(error) => return Err(error.to_string()),
        };
        Ok(Self {
            path,
            state: Mutex::new(state),
            log,
        })
    }

    fn snapshot(&self) -> Result<PersistedProofState, String> {
        self.state
            .lock()
            .map_err(|_| "proof sink lock is poisoned".to_string())
            .map(|state| state.clone())
    }

    fn persist(&self, reason: &str) -> Result<(), String> {
        let state = self.snapshot()?;
        write_json_atomically(&self.path, &state)?;
        self.log.record(
            "proof_state_persisted",
            json!({
                "reason": reason,
                "path": self.path,
                "window_count": state.placements.len(),
                "restart_scenario": state.restart_scenario,
            }),
        )
    }

    fn prepare_missing_display_restart(&self, window_id: WindowId) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "proof sink lock is poisoned".to_string())?;
        let maximized = state
            .placements
            .get(window_id.as_str())
            .is_some_and(CapturedWindowPlacement::is_maximized);
        state.placements.insert(
            window_id.as_str().to_string(),
            CapturedWindowPlacement::new(
                window_id,
                WindowPlacement::new(ScreenPoint::new(50_000, 50_000), ScreenSize::new(900, 650)),
                maximized,
                CapturedDisplayAssociation::Unresolved,
            ),
        );
        state.restart_scenario = Some("missing_saved_display".to_string());
        drop(state);
        self.persist("prepare_missing_saved_display")
    }

    fn consume_restart_scenario(&self) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "proof sink lock is poisoned".to_string())?;
        state.restart_scenario = None;
        drop(state);
        self.persist("consume_restart_scenario")
    }
}

impl WindowPlacementSink for ProofSink {
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String> {
        self.state
            .lock()
            .map_err(|_| "proof sink lock is poisoned".to_string())?
            .placements
            .insert(
                placement.window_id().as_str().to_string(),
                placement.clone(),
            );
        self.log.record(
            "placement_staged",
            serde_json::to_value(placement).map_err(|error| error.to_string())?,
        )
    }

    fn request_flush(
        &self,
        request: &WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        let (sender, receiver) = mpsc::channel();
        let result = self.persist("lifecycle_flush");
        let completion = match &result {
            Ok(()) => WindowPlacementFlushCompletion::Succeeded,
            Err(detail) => WindowPlacementFlushCompletion::Failed(detail.clone()),
        };
        let _ = self.log.record(
            "flush_receipt",
            json!({
                "request": request,
                "completion": format!("{completion:?}"),
            }),
        );
        sender.send(completion).map_err(|error| error.to_string())?;
        result?;
        Ok(WindowPlacementFlushTicket::new(receiver))
    }
}

struct ProofScheduler {
    app: AppHandle<Wry>,
    clock: Arc<ProcessMonotonicClock>,
    host: Mutex<Option<Weak<ProofHost>>>,
    log: Arc<EvidenceLog>,
}

impl ProofScheduler {
    fn new(app: AppHandle<Wry>, clock: Arc<ProcessMonotonicClock>, log: Arc<EvidenceLog>) -> Self {
        Self {
            app,
            clock,
            host: Mutex::new(None),
            log,
        }
    }

    fn attach(&self, host: &Arc<ProofHost>) -> Result<(), String> {
        *self
            .host
            .lock()
            .map_err(|_| "scheduler host lock is poisoned".to_string())? =
            Some(Arc::downgrade(host));
        Ok(())
    }
}

impl WindowLifecycleScheduler for ProofScheduler {
    fn schedule(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        let weak = self
            .host
            .lock()
            .map_err(|_| "scheduler host lock is poisoned".to_string())?
            .clone()
            .ok_or_else(|| "scheduler host is not attached".to_string())?;
        let delay = wake.due_at().get().saturating_sub(self.clock.now().get());
        let app = self.app.clone();
        let log = self.log.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(delay));
            let dispatch_log = log.clone();
            let dispatch = app.run_on_main_thread(move || {
                let result = weak
                    .upgrade()
                    .ok_or_else(|| "scheduled host is unavailable".to_string())
                    .and_then(|host| {
                        host.handle_scheduled_wake(wake)
                            .map_err(|error| format!("{error:?}"))
                    });
                let _ = dispatch_log.record(
                    "scheduled_wake_delivered",
                    json!({"result": format!("{result:?}")}),
                );
            });
            if let Err(error) = dispatch {
                let _ = log.record(
                    "scheduled_wake_dispatch_failed",
                    json!({"detail": error.to_string()}),
                );
            }
        });
        Ok(())
    }
}

struct ProofDisplayAllocator {
    next: u64,
}

impl DisplayIdAllocator for ProofDisplayAllocator {
    type Error = Infallible;

    fn allocate(&mut self, _observation: &ObservedDisplay) -> Result<DisplayId, Self::Error> {
        let id = DisplayId::new(format!("proof-display:{}", self.next))
            .expect("proof display ids use the opaque-id grammar");
        self.next += 1;
        Ok(id)
    }
}

struct ProofState {
    host: Arc<ProofHost>,
    sink: Arc<ProofSink>,
    log: Arc<EvidenceLog>,
    generation: AtomicU64,
    initial_restore_complete: AtomicBool,
    workspace_enabled: AtomicBool,
    startup_scenario: Option<String>,
}

impl ProofState {
    fn next_generation(&self) -> ApplyGeneration {
        ApplyGeneration::new(self.generation.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

fn window_id(label: &str) -> Result<WindowId, String> {
    WindowId::new(label).map_err(|error| error.to_string())
}

fn protected_main() -> Result<ProtectedPrimaryPolicy, String> {
    Ok(ProtectedPrimaryPolicy::Preserve {
        transport_handle: HostWindowHandle::new(MAIN_LABEL).map_err(|error| error.to_string())?,
    })
}

fn managed_windows<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<ManagedWebviewWindow<R>>, String> {
    [MAIN_LABEL, WORKSPACE_LABEL]
        .into_iter()
        .filter_map(|label| {
            app.get_webview_window(label)
                .filter(|window| window.is_visible().is_ok())
                .map(|window| (label, window))
        })
        .map(|(label, window)| Ok(ManagedWebviewWindow::new(Some(window_id(label)?), window)))
        .collect()
}

fn current_observation<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<longhorn_tauri_windowing::DesktopObservation, String> {
    observe_tauri_desktop(app, &managed_windows(app)?, &mut DefaultDisplayMetadata)
        .map_err(|error| format!("{error:?}"))
}

fn fallback_placement(
    observations: &[ObservedDisplay],
    ordinal: usize,
) -> Result<WindowPlacement, String> {
    let display = observations
        .iter()
        .find(|display| display.facts().is_main())
        .or_else(|| observations.first())
        .ok_or_else(|| "proof requires at least one available display".to_string())?;
    let work = display.facts().work_area();
    let inset = i32::try_from(80 + ordinal * 90).map_err(|error| error.to_string())?;
    Ok(WindowPlacement::new(
        ScreenPoint::new(
            work.origin().x().get().saturating_add(inset),
            work.origin().y().get().saturating_add(inset),
        ),
        if ordinal == 0 {
            ScreenSize::new(900, 650)
        } else {
            ScreenSize::new(680, 480)
        },
    ))
}

fn desired_windows(
    observation: &longhorn_tauri_windowing::DesktopObservation,
    sink: &ProofSink,
    include_main: bool,
    include_workspace: bool,
    main_maximized: Option<bool>,
    hidden_restore: bool,
    log: &EvidenceLog,
) -> Result<Vec<DesiredWindow>, String> {
    let persisted = sink.snapshot()?;
    let mut allocator = ProofDisplayAllocator { next: 0 };
    let reconciliation = reconcile_displays(
        &KnownDisplayRegistry::new(),
        observation.displays().iter().cloned(),
        &mut allocator,
    )
    .map_err(|error| error.to_string())?;
    let policy = PlacementPolicy::new(ScreenSize::new(320, 240), ScreenSize::new(160, 120));
    let scenario_active = persisted.restart_scenario.as_deref() == Some("missing_saved_display");
    let requested = [
        (MAIN_LABEL, include_main, 0_usize),
        (WORKSPACE_LABEL, include_workspace, 1_usize),
    ];
    let mut desired = Vec::new();

    for (label, included, ordinal) in requested {
        if !included {
            continue;
        }
        let id = window_id(label)?;
        let saved = persisted.placements.get(label);
        let normal = saved
            .map(CapturedWindowPlacement::normal_placement)
            .map_or_else(|| fallback_placement(observation.displays(), ordinal), Ok)?;
        let maximized = if label == MAIN_LABEL {
            main_maximized
                .unwrap_or_else(|| saved.is_some_and(CapturedWindowPlacement::is_maximized))
        } else {
            saved.is_some_and(CapturedWindowPlacement::is_maximized)
        };
        let home = scenario_active.then(|| {
            DisplayId::new(MISSING_DISPLAY_ID)
                .expect("constant missing display id uses the opaque-id grammar")
        });
        let config = WindowPlacementConfig::new(id, WindowRole::RequiredPrimary, normal)
            .with_home_display(home)
            .with_maximized(maximized);
        let resolution = resolve_window_placement(&config, reconciliation.inventory(), policy)
            .map_err(|error| error.to_string())?;
        log.record(
            "placement_resolution",
            json!({
                "window": label,
                "scenario": persisted.restart_scenario,
                "inventory": reconciliation.inventory(),
                "config": config,
                "resolution": resolution,
            }),
        )?;
        match resolution {
            WindowPlacementResolution::Resolved(resolved) => {
                let visible = if hidden_restore {
                    include_workspace && label == MAIN_LABEL
                } else {
                    true
                };
                desired.push(DesiredWindow::from_resolved(&resolved, visible));
            }
            other => return Err(format!("window {label} did not resolve: {other:?}")),
        }
    }
    Ok(desired)
}

fn dynamic_factory(
    app: &AppHandle<Wry>,
    id: &WindowId,
) -> Result<WebviewWindow<Wry>, longhorn_tauri_windowing::WindowFactoryError> {
    WebviewWindowBuilder::new(app, id.as_str(), WebviewUrl::App("index.html".into()))
        .title("Longhorn Dynamic Workspace")
        .inner_size(680.0, 480.0)
        .visible(false)
        .resizable(true)
        .background_color(tauri::webview::Color(16, 19, 24, 255))
        .build()
        .map_err(
            |error| longhorn_tauri_windowing::WindowFactoryError::Failed {
                detail: error.to_string(),
            },
        )
}

#[allow(clippy::too_many_arguments)]
fn apply_window_set(
    app: &AppHandle<Wry>,
    state: &ProofState,
    include_main: bool,
    include_workspace: bool,
    main_maximized: Option<bool>,
    hidden_restore: bool,
    evidence_name: &str,
) -> Result<Value, String> {
    let observation = current_observation(app)?;
    let desired = desired_windows(
        &observation,
        &state.sink,
        include_main,
        include_workspace,
        main_maximized,
        hidden_restore,
        &state.log,
    )?;
    let input = WindowDiffInput::new(
        desired,
        observation.windows().iter().cloned(),
        state.host.capabilities(true),
        state.next_generation(),
    )
    .with_protected_primary(protected_main()?);
    let receipt = state
        .host
        .apply(
            app,
            input,
            dynamic_factory,
            TauriWindowMutationBackend,
            TauriDesktopReadback::new(DefaultDisplayMetadata),
        )
        .map_err(|error| format!("{error:?}"))?;
    let summary = json!({
        "converged": receipt.apply().is_converged(),
        "plan": receipt.apply().plan(),
        "attempts": receipt.apply().attempts().iter().map(|attempt| {
            json!({
                "window_id": attempt.window_id(),
                "operation": format!("{:?}", attempt.operation()),
                "outcome": format!("{:?}", attempt.outcome()),
            })
        }).collect::<Vec<_>>(),
        "readback": match receipt.apply().readback() {
            ApplyReadback::Complete { observation, convergence } => json!({
                "observation": observation,
                "convergence": match convergence {
                    ApplyConvergence::Planned(plan) => json!({
                        "state": "planned",
                        "remaining": plan,
                    }),
                    ApplyConvergence::Invalid(error) => json!({
                        "state": "invalid",
                        "error": format!("{error:?}"),
                    }),
                },
            }),
            ApplyReadback::Failed(error) => json!({
                "state": "failed",
                "error": format!("{error:?}"),
            }),
        },
        "reveal": receipt.reveal(),
    });
    state.log.record(evidence_name, summary.clone())?;
    Ok(summary)
}

fn flush_window(
    state: &ProofState,
    label: &str,
) -> Result<longhorn_tauri_windowing::TauriWindowLifecycleReceipt, String> {
    state
        .host
        .handle_lifecycle_event(WindowLifecycleEvent::FlushRequested {
            window_id: window_id(label)?,
        })
        .map_err(|error| format!("{error:?}"))
}

fn flush_all(app: &AppHandle<Wry>, state: &ProofState) -> Result<Value, String> {
    let mut receipts = Vec::new();
    for label in [MAIN_LABEL, WORKSPACE_LABEL] {
        if app.get_webview_window(label).is_some() {
            receipts.push(flush_window(state, label)?);
        }
    }
    let value = serde_json::to_value(&receipts).map_err(|error| error.to_string())?;
    state.log.record("operator_flush", value.clone())?;
    Ok(value)
}

#[tauri::command]
fn page_ready(label: String, state: State<'_, ProofState>) -> Result<WindowRevealReceipt, String> {
    let receipt = state
        .host
        .mark_page_ready(&window_id(&label)?)
        .map_err(|error| format!("{error:?}"))?;
    state.log.record(
        "page_ready",
        serde_json::to_value(&receipt).map_err(|error| error.to_string())?,
    )?;
    Ok(receipt)
}

#[tauri::command]
fn proof_status(app: AppHandle<Wry>, state: State<'_, ProofState>) -> Result<Value, String> {
    let observation = current_observation(&app)?;
    Ok(json!({
        "artifact": {
            "application": "Longhorn Window Proof",
            "longhorn": env!("CARGO_PKG_VERSION"),
            "tauri": TAURI_VERSION,
            "rustc": env!("PROOF_RUSTC_VERSION"),
            "target": std::env::consts::ARCH,
            "os": platform_version(),
        },
        "host": {
            "active": state.host.is_active(),
            "installed_window_count": state.host.installed_window_count().map_err(|error| format!("{error:?}"))?,
            "initial_restore_complete": state.initial_restore_complete.load(Ordering::Acquire),
            "workspace_enabled": state.workspace_enabled.load(Ordering::Acquire),
        },
        "startup_scenario": state.startup_scenario,
        "paths": {
            "placement_state": state.sink.path,
            "transcript": state.log.path,
        },
        "persisted": state.sink.snapshot()?,
        "observation": observation,
    }))
}

#[tauri::command]
fn toggle_maximized(app: AppHandle<Wry>, state: State<'_, ProofState>) -> Result<Value, String> {
    let observation = current_observation(&app)?;
    let maximized = observation
        .windows()
        .iter()
        .find(|window| {
            window
                .window_id()
                .is_some_and(|id| id.as_str() == MAIN_LABEL)
        })
        .ok_or_else(|| "main window observation is missing".to_string())?
        .is_maximized();
    apply_window_set(
        &app,
        &state,
        true,
        state.workspace_enabled.load(Ordering::Acquire),
        Some(!maximized),
        false,
        "toggle_maximized_apply",
    )
}

#[tauri::command]
fn set_workspace(
    app: AppHandle<Wry>,
    state: State<'_, ProofState>,
    enabled: bool,
) -> Result<Value, String> {
    state.workspace_enabled.store(enabled, Ordering::Release);
    let summary = match apply_window_set(
        &app,
        &state,
        true,
        enabled,
        None,
        enabled,
        if enabled {
            "dynamic_workspace_create_apply"
        } else {
            "dynamic_workspace_close_apply"
        },
    ) {
        Ok(summary) => summary,
        Err(error) => {
            state.workspace_enabled.store(!enabled, Ordering::Release);
            return Err(error);
        }
    };
    if summary["converged"].as_bool() != Some(true) {
        schedule_workspace_reconcile(app, enabled, 1);
    }
    Ok(summary)
}

#[tauri::command]
fn prove_protected_primary(
    app: AppHandle<Wry>,
    state: State<'_, ProofState>,
) -> Result<Value, String> {
    let workspace = state.workspace_enabled.load(Ordering::Acquire);
    let omitted = apply_window_set(
        &app,
        &state,
        false,
        workspace,
        None,
        false,
        "protected_primary_omitted_apply",
    )?;
    let survived = app.get_webview_window(MAIN_LABEL).is_some();
    if !survived {
        return Err("protected main window was destroyed".to_string());
    }
    let restored = apply_window_set(
        &app,
        &state,
        true,
        workspace,
        None,
        false,
        "protected_primary_restored_apply",
    )?;
    let result = json!({
        "survived_desired_set_omission": survived,
        "omitted_apply": omitted,
        "restored_apply": restored,
    });
    state
        .log
        .record("protected_primary_proof", result.clone())?;
    Ok(result)
}

#[tauri::command]
fn prepare_missing_display_restart(
    app: AppHandle<Wry>,
    state: State<'_, ProofState>,
) -> Result<String, String> {
    let _ = flush_all(&app, &state)?;
    state
        .sink
        .prepare_missing_display_restart(window_id(MAIN_LABEL)?)?;
    state.log.record(
        "missing_display_restart_prepared",
        json!({
            "saved_home_display": MISSING_DISPLAY_ID,
            "saved_outer_origin": [50_000, 50_000],
        }),
    )?;
    Ok("Missing saved display prepared. Quit and relaunch the packaged app.".to_string())
}

#[tauri::command]
fn flush_proof(app: AppHandle<Wry>, state: State<'_, ProofState>) -> Result<Value, String> {
    flush_all(&app, &state)
}

#[tauri::command]
fn quit_proof(app: AppHandle<Wry>, state: State<'_, ProofState>) -> Result<(), String> {
    let receipt = state
        .host
        .teardown()
        .map_err(|error| format!("{error:?}"))?;
    state
        .log
        .record("operator_teardown", teardown_evidence(&receipt))?;
    app.exit(0);
    Ok(())
}

fn teardown_evidence(receipt: &longhorn_tauri_windowing::TauriWindowHostTeardownReceipt) -> Value {
    json!({
        "status": format!("{:?}", receipt.status()),
        "deactivated_listeners": receipt.deactivated_listeners(),
        "shutdown": receipt.shutdown().map(shutdown_evidence),
    })
}

fn shutdown_evidence(receipt: &WindowShutdownReceipt) -> Value {
    json!({
        "actions": receipt.actions(),
        "flush": receipt.flush(),
    })
}

fn schedule_initial_restore(app: AppHandle<Wry>, attempt: u8) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        let dispatch_app = app.clone();
        let _ = app.run_on_main_thread(move || run_initial_restore(&dispatch_app, attempt));
    });
}

fn schedule_workspace_reconcile(app: AppHandle<Wry>, enabled: bool, attempt: u8) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        let dispatch_app = app.clone();
        let _ = app.run_on_main_thread(move || {
            run_workspace_reconcile(&dispatch_app, enabled, attempt);
        });
    });
}

fn run_workspace_reconcile(app: &AppHandle<Wry>, enabled: bool, attempt: u8) {
    let Some(state) = app.try_state::<ProofState>() else {
        return;
    };
    if state.workspace_enabled.load(Ordering::Acquire) != enabled {
        return;
    }
    let result = apply_window_set(
        app,
        &state,
        true,
        enabled,
        None,
        enabled,
        if enabled {
            "dynamic_workspace_create_retry"
        } else {
            "dynamic_workspace_close_retry"
        },
    );
    match result {
        Ok(summary) if summary["converged"].as_bool() == Some(true) => {
            let _ = state.log.record(
                "dynamic_workspace_reconciled",
                json!({"enabled": enabled, "attempt": attempt, "result": summary}),
            );
        }
        Ok(summary) if attempt < 8 => {
            let _ = state.log.record(
                "dynamic_workspace_reconcile_retry",
                json!({"enabled": enabled, "attempt": attempt, "result": summary}),
            );
            schedule_workspace_reconcile(app.clone(), enabled, attempt + 1);
        }
        result => {
            let _ = state.log.record(
                "dynamic_workspace_reconcile_failed",
                json!({
                    "enabled": enabled,
                    "attempt": attempt,
                    "result": result.map_err(|error| error.to_string()),
                }),
            );
        }
    }
}

fn run_initial_restore(app: &AppHandle<Wry>, attempt: u8) {
    let Some(state) = app.try_state::<ProofState>() else {
        return;
    };
    let result = apply_window_set(
        app,
        &state,
        true,
        false,
        None,
        true,
        "initial_hidden_restore_apply",
    );
    match result {
        Ok(summary) if summary["converged"].as_bool() == Some(true) => {
            state
                .initial_restore_complete
                .store(true, Ordering::Release);
            if state.startup_scenario.as_deref() == Some("missing_saved_display") {
                if let Err(error) = state.sink.consume_restart_scenario() {
                    let _ = state.log.record(
                        "missing_display_restart_consume_failed",
                        json!({"detail": error}),
                    );
                    return;
                }
                let _ = state.log.record(
                    "missing_display_restart_resolved",
                    json!({"attempt": attempt, "result": summary}),
                );
            }
        }
        Ok(summary) if attempt < 8 => {
            let _ = state.log.record(
                "initial_hidden_restore_retry",
                json!({"attempt": attempt, "result": summary}),
            );
            schedule_initial_restore(app.clone(), attempt + 1);
        }
        result => {
            let _ = state.log.record(
                "initial_hidden_restore_failed",
                json!({
                    "attempt": attempt,
                    "result": result.map_err(|error| error.to_string()),
                }),
            );
        }
    }
}

fn setup(app: &mut tauri::App<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&data_dir)?;
    let log = Arc::new(EvidenceLog::new(
        data_dir.join("operator-transcript.jsonl"),
    )?);
    let sink = Arc::new(ProofSink::load(
        data_dir.join("placement-state.json"),
        log.clone(),
    )?);
    let persisted = sink.snapshot()?;
    let startup_scenario = persisted.restart_scenario.clone();
    let main = app
        .get_webview_window(MAIN_LABEL)
        .ok_or("predeclared main window is missing")?;
    let scale = scale_factor_from_tauri(main.scale_factor()?)?;
    let mapper = Arc::new(UniformWindowGeometryMapper::new(scale));
    let capture = Arc::new(TauriWindowCaptureBackend::new(mapper.clone()));
    let clock = Arc::new(ProcessMonotonicClock::new());
    let scheduler = Arc::new(ProofScheduler::new(
        app_handle.clone(),
        clock.clone(),
        log.clone(),
    ));
    let reporter_log = log.clone();
    let reporter = Arc::new(move |report: WindowLifecycleReport| {
        let detail = serde_json::to_value(report)
            .unwrap_or_else(|error| json!({"serialization_error": error.to_string()}));
        let _ = reporter_log.record("lifecycle_report", detail);
    });
    let close_log = log.clone();
    let close_handler: Arc<dyn WindowUserCloseHandler> = Arc::new(move |id: &WindowId| {
        close_log.record("user_close_reported", json!({"window_id": id}))
    });
    let services = TauriWindowLifecycleServices::new(
        clock,
        scheduler.clone(),
        mapper,
        capture,
        sink.clone(),
        close_handler,
        reporter,
        Arc::new(TauriWindowRevealBackend),
    );
    let initial_normal = persisted
        .placements
        .get(MAIN_LABEL)
        .map(CapturedWindowPlacement::normal_placement);
    let predeclared = match initial_normal {
        Some(normal) => {
            PredeclaredTauriWindow::new(window_id(MAIN_LABEL)?, main).with_initial_normal(normal)
        }
        None => PredeclaredTauriWindow::new(window_id(MAIN_LABEL)?, main),
    };
    let initialization = assemble_tauri_window_host(
        &app_handle,
        WindowLifecyclePolicy::new(
            WindowLifecycleDuration::from_millis(180),
            WindowLifecycleDuration::from_millis(250),
            WindowLifecycleDuration::from_millis(450),
            WindowLifecycleDuration::from_millis(300),
            WindowLifecycleDuration::from_millis(1_500),
        ),
        services,
        [predeclared],
        Some(HostWindowHandle::new(MAIN_LABEL)?),
    )
    .map_err(|error| format!("{error:?}"))?;
    let (host, initialization_receipt) = initialization.into_parts();
    scheduler.attach(&host)?;
    log.record(
        "host_initialized",
        json!({
            "status": format!("{:?}", initialization_receipt.status()),
            "registrations": initialization_receipt.registrations().iter().map(|registration| {
                json!({
                    "window_id": registration.window_id(),
                    "transport_handle": registration.transport_handle(),
                })
            }).collect::<Vec<_>>(),
            "startup_visibility": false,
            "startup_scenario": startup_scenario,
        }),
    )?;
    let state = ProofState {
        host,
        sink,
        log,
        generation: AtomicU64::new(0),
        initial_restore_complete: AtomicBool::new(false),
        workspace_enabled: AtomicBool::new(false),
        startup_scenario,
    };
    app.manage(state);
    schedule_initial_restore(app_handle, 1);
    Ok(())
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

fn write_json_atomically(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "placement path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn main() {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            page_ready,
            proof_status,
            toggle_maximized,
            set_workspace,
            prove_protected_primary,
            prepare_missing_display_restart,
            flush_proof,
            quit_proof,
        ])
        .setup(setup)
        .build(tauri::generate_context!())
        .expect("could not build packaged Longhorn window proof");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. })
            && let Some(state) = app.try_state::<ProofState>()
            && state.host.is_active()
        {
            let result = state.host.teardown();
            let detail = result
                .as_ref()
                .map(teardown_evidence)
                .unwrap_or_else(|error| json!({"error": format!("{error:?}")}));
            let _ = state.log.record("application_exit_teardown", detail);
        }
    });
}
