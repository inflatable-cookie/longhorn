//! Real native proof for Longhorn's bundled local child-view source
//! (contract 017).
//!
//! One predeclared host window carries one child webview whose initial
//! document is a bundled local asset. The proof drives the ordinary
//! coordinator plan lifecycle (attach, bounds, show, input, focus), then
//! asserts, against the real native webview:
//!
//! - the observed source identity is the application asset protocol;
//! - the scoped IPC command runs inside the child and echoes the
//!   initialization-script nonce, while an unpermitted command is denied;
//! - the document's popup and navigation escape probes are denied;
//! - every page-load event names the application asset protocol;
//! - detach, post-detach observation, and idempotent teardown receipts.
//!
//! The document's final `document.title` is the one evidence channel back to
//! this process; the child holds exactly one IPC permission, so nothing else
//! can report. Every failure lands in `failures`, the report line
//! (`ncl-proof-report:{json}`) carries the whole ledger, and the process exit
//! code is 0 only for a pass. A watchdog fails the proof at 120 seconds.

use std::{
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use longhorn_core::{ClientPoint, ClientRect, ClientSize, RoundingMode, ScaleFactor, WindowId};
use longhorn_native_content::{
    AttachGeneration, DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility, FocusIntent,
    InputRoutingMode, NativeContentCoordinator, NativeContentIslandId, NativeContentKindId,
    OperationOutcome,
};
use longhorn_tauri_native_content_child_view::{
    CHILD_VIEW_CAPABILITIES, ChildViewAdapter, ChildViewError, ChildViewLabel,
    ChildViewPolicyEvent, ChildViewPolicyHooks, ChildViewSource, ChildViewSpec,
    ChildViewTeardownOutcome, TauriChildViewRuntime,
};
use serde_json::{Value, json};
use tauri::{AppHandle, Manager, State, Url, Wry};

const BUNDLED_DOCUMENT: &str = "offline/document.html";
const DOCUMENT_PATH: &str = "/offline/document.html";
const ESCAPE_URL: &str = "https://escape.invalid/adapter-probe";
const ESCAPE_POPUP_URL: &str = "https://escape.invalid/popup";
const CSP: &str = "default-src 'self'; connect-src 'self' ipc: http://ipc.localhost";
const NAVIGATION_POLICY: &str =
    "admit only scheme `tauri` (the application asset protocol); deny every other candidate";
const CAPABILITY: &str = "offline-document: webviews [proof-child], permissions [allow-ipc-proof]";
const EXPECTED_TITLE_PREFIX: &str = "ncl-proof|popup:denied|ping:";
const EXPECTED_TITLE_TAIL: &str = "|forbidden:denied|csp:denied|navigation:denied";
const WATCHDOG: Duration = Duration::from_secs(120);
const ISLAND: &str = "island:local-proof";
const HOST_WINDOW: &str = "window:local-proof-host";

type Adapter = ChildViewAdapter<TauriChildViewRuntime<Wry>>;

struct Core {
    ledger: Arc<Mutex<Ledger>>,
    adapter: Arc<Adapter>,
    coordinator: Arc<Mutex<NativeContentCoordinator>>,
    generation: AttachGeneration,
    app: AppHandle<Wry>,
}

#[derive(Default)]
struct Ledger {
    steps: Vec<Value>,
    events: Vec<Value>,
    failures: Vec<String>,
    injected_nonce: String,
    load_finished_url: Option<String>,
    post_load_done: bool,
    ipc_nonce: Option<String>,
    ipc_labels: Vec<String>,
    title: Option<String>,
    popup_denied_url: Option<String>,
    download_denied: bool,
    teardown_started: bool,
    done: bool,
}

impl Ledger {
    fn record(&mut self, name: &str, detail: Value) {
        self.steps.push(json!({ "step": name, "detail": detail }));
    }

    fn fail(&mut self, message: impl Into<String>) {
        self.failures.push(message.into());
    }
}

fn nonce() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock runs after the epoch")
        .as_nanos();
    format!("ncl-{nanos:016x}")
}

fn host_window_id() -> WindowId {
    WindowId::new(HOST_WINDOW).expect("bounded window id")
}

fn island_id() -> NativeContentIslandId {
    NativeContentIslandId::new(ISLAND).expect("bounded island id")
}

fn viewport() -> ClientRect {
    ClientRect::new(
        ClientPoint::new(24.0, 24.0).expect("bounded CSS point"),
        ClientSize::new(440.0, 300.0).expect("bounded CSS size"),
    )
}

fn desired_update(
    generation: AttachGeneration,
    presence: DesiredPresence,
    focus: FocusIntent,
) -> DesiredUpdate {
    DesiredUpdate::new(
        generation,
        host_window_id(),
        viewport(),
        ScaleFactor::from_thousandths(1000).expect("1000 thousandths is valid"),
        RoundingMode::Nearest,
        presence,
        DesiredVisibility::Visible,
        focus,
        InputRoutingMode::NativeDirect,
    )
}

fn event_json(event: &ChildViewPolicyEvent) -> Value {
    match event {
        ChildViewPolicyEvent::PageLoadStarted { url } => {
            json!({"kind": "pageLoadStarted", "url": url.as_str()})
        }
        ChildViewPolicyEvent::PageLoadFinished { url } => {
            json!({"kind": "pageLoadFinished", "url": url.as_str()})
        }
        ChildViewPolicyEvent::PopupDenied { url } => {
            json!({"kind": "popupDenied", "url": url.as_str()})
        }
        ChildViewPolicyEvent::DownloadDenied { url } => {
            json!({"kind": "downloadDenied", "url": url.as_str()})
        }
        ChildViewPolicyEvent::DocumentTitleChanged { title } => {
            json!({"kind": "documentTitleChanged", "title": title})
        }
    }
}

/// One structured report line. The proof script parses this line.
fn report_line(ledger: &Ledger) -> (String, bool) {
    let pass = ledger.failures.is_empty();
    let report = json!({
        "schema": "longhorn.native-content-local-proof.v1",
        "outcome": if pass { "pass" } else { "fail" },
        "platform": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "family": std::env::consts::FAMILY,
        },
        "tauriVersion": tauri::VERSION,
        "source": {"kind": "bundled", "document": BUNDLED_DOCUMENT},
        "navigationPolicy": NAVIGATION_POLICY,
        "csp": CSP,
        "capability": CAPABILITY,
        "injectedNonce": ledger.injected_nonce,
        "pageLoadUrls": ledger
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event["kind"].as_str(),
                    Some("pageLoadStarted") | Some("pageLoadFinished")
                )
            })
            .filter_map(|event| event["url"].as_str().map(str::to_string))
            .collect::<Vec<_>>(),
        "ipc": {
            "nonce": ledger.ipc_nonce,
            "webviews": ledger.ipc_labels,
            "nonceMatchesInjection": ledger.ipc_nonce.as_ref() == Some(&ledger.injected_nonce),
        },
        "documentTitle": ledger.title,
        "steps": ledger.steps,
        "policyEvents": ledger.events,
        "failures": ledger.failures,
    });
    (format!("ncl-proof-report:{report}"), pass)
}

fn exit_with(ledger: &Arc<Mutex<Ledger>>, code: i32) -> ! {
    let guard = ledger.lock().expect("proof ledger locks");
    let (line, pass) = report_line(&guard);
    println!("{line}");
    drop(guard);
    std::process::exit(if pass && code == 0 { 0 } else { code.max(1) });
}

/// Advances the proof after any evidence arrival. Runs the one post-load
/// native step on the caller (the main thread) and spawns the one teardown
/// thread once every pre-teardown receipt exists.
fn advance(core: &Arc<Core>) {
    let run_post = {
        let mut ledger = core.ledger.lock().expect("proof ledger locks");
        if ledger.load_finished_url.is_some() && !ledger.post_load_done {
            ledger.post_load_done = true;
            true
        } else {
            false
        }
    };
    if run_post {
        run_post_load(core);
    }

    let spawn = {
        let mut ledger = core.ledger.lock().expect("proof ledger locks");
        if ledger.post_load_done
            && ledger.ipc_nonce.is_some()
            && ledger.title.is_some()
            && ledger.popup_denied_url.is_some()
            && ledger.download_denied
            && !ledger.teardown_started
        {
            ledger.teardown_started = true;
            true
        } else {
            false
        }
    };
    if spawn {
        let core = core.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            run_teardown(&core);
        });
    }
}

/// Post-load native evidence: source identity, the adapter-typed escape
/// denial, and the coordinator observation admission that later detach needs.
/// Native calls run without the ledger lock; results are recorded after.
fn run_post_load(core: &Arc<Core>) {
    let mut steps: Vec<(&str, Value)> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    match core.adapter.current_url(core.generation) {
        Ok(observed) => {
            let scheme_ok = observed.scheme() == "tauri";
            let path_ok = observed.path() == DOCUMENT_PATH;
            steps.push((
                "observed-source-identity",
                json!({
                    "url": observed.as_str(),
                    "schemeIsAssetProtocol": scheme_ok,
                    "pathIsBundledDocument": path_ok,
                }),
            ));
            if !scheme_ok {
                failures.push(format!(
                    "observed source is not the application asset protocol: {observed}"
                ));
            }
            if !path_ok {
                failures.push(format!(
                    "observed source path is not the bundled document: {observed}"
                ));
            }
        }
        Err(error) => failures.push(format!("current_url failed after load: {error:?}")),
    }

    let escape = Url::parse(ESCAPE_URL).expect("escape probe URL parses");
    match core.adapter.navigate(core.generation, escape) {
        Err(ChildViewError::NavigationDenied(denied)) if denied.as_str() == ESCAPE_URL => {
            steps.push((
                "adapter-typed-escape-denial",
                json!({"denied": ESCAPE_URL, "code": "child:navigation-denied"}),
            ));
        }
        other => failures.push(format!(
            "adapter did not deny the escape navigation with the typed error: {other:?}"
        )),
    }

    match core.adapter.observe(core.generation) {
        Ok(update) => {
            let mut coordinator = core.coordinator.lock().expect("proof coordinator locks");
            let revision = coordinator.observed().revision();
            match coordinator.admit_observation(revision, update) {
                Ok(_) => steps.push((
                    "observation-admitted",
                    json!({"generation": core.generation.get()}),
                )),
                Err(error) => failures.push(format!("observation admission failed: {error:?}")),
            }
        }
        Err(error) => failures.push(format!("observe failed after load: {error:?}")),
    }

    let clean = {
        let mut ledger = core.ledger.lock().expect("proof ledger locks");
        for (name, detail) in steps {
            ledger.record(name, detail);
        }
        for failure in failures {
            ledger.fail(failure);
        }
        ledger.failures.is_empty()
    };
    if clean {
        advance(core);
    }
}

/// Detach through the coordinator plan, then assert post-detach observation
/// and idempotent teardown, and finish.
fn run_teardown(core: &Arc<Core>) {
    let mut failures: Vec<String> = Vec::new();
    let mut steps: Vec<(&str, Value)> = Vec::new();
    steps.push((
        "teardown-started",
        json!({"generation": core.generation.get()}),
    ));

    {
        let mut coordinator = core.coordinator.lock().expect("proof coordinator locks");
        let revision = coordinator.desired().revision();
        if let Err(error) = coordinator.update_desired(
            revision,
            desired_update(
                core.generation,
                DesiredPresence::Absent,
                FocusIntent::Unchanged,
            ),
        ) {
            failures.push(format!("detach desired update failed: {error:?}"));
        } else {
            match coordinator.plan() {
                Ok(plan) => match core.adapter.apply(&coordinator, &plan) {
                    Ok(receipt) => {
                        let applied = receipt
                            .steps()
                            .iter()
                            .filter(|step| step.outcome() == &OperationOutcome::Applied)
                            .count();
                        steps.push((
                            "detach-applied",
                            json!({"appliedSteps": applied, "totalSteps": receipt.steps().len()}),
                        ));
                    }
                    Err(error) => failures.push(format!("detach apply failed: {error:?}")),
                },
                Err(error) => failures.push(format!("detach plan failed: {error:?}")),
            }
        }
    }

    match core.adapter.observe(core.generation) {
        Ok(update) => match serde_json::to_value(&update) {
            Ok(value) => steps.push(("post-detach-observation", value)),
            Err(error) => {
                failures.push(format!("post-detach observation encoding failed: {error}"))
            }
        },
        Err(error) => failures.push(format!("post-detach observe failed: {error:?}")),
    }
    match core.adapter.teardown() {
        Ok(receipt) => {
            let idempotent = receipt.outcome() == ChildViewTeardownOutcome::AlreadyDetached;
            steps.push((
                "teardown-receipt",
                json!({"outcome": receipt.outcome(), "idempotent": idempotent}),
            ));
            if !idempotent {
                failures.push("teardown after a coordinator detach was not idempotent".into());
            }
        }
        Err(error) => failures.push(format!("teardown failed: {error:?}")),
    }

    {
        let mut ledger = core.ledger.lock().expect("proof ledger locks");
        ledger.done = true;
        for (name, detail) in steps {
            ledger.record(name, detail);
        }
        for failure in failures {
            ledger.fail(failure);
        }
    }
    exit_with(&core.ledger, 0);
}

#[tauri::command]
fn ipc_proof_ping(
    nonce: String,
    webview: tauri::Webview<Wry>,
    core: State<'_, Arc<Core>>,
) -> String {
    {
        let mut ledger = core.ledger.lock().expect("proof ledger locks");
        ledger.ipc_nonce = Some(nonce.clone());
        ledger.ipc_labels.push(webview.label().to_string());
        ledger.events.push(json!({
            "kind": "ipcProofPing",
            "webview": webview.label(),
            "nonce": nonce,
        }));
    }
    advance(&core);
    nonce
}

/// Exists so the document can prove the capability boundary denies it. It is
/// granted to no webview.
#[tauri::command]
fn ipc_forbidden_probe() -> String {
    "the forbidden probe must never run".to_string()
}

fn main() {
    let ledger = Arc::new(Mutex::new(Ledger {
        injected_nonce: nonce(),
        ..Ledger::default()
    }));

    // Watchdog: a proof that never completes must fail loudly, not hang.
    {
        let ledger = ledger.clone();
        thread::spawn(move || {
            let deadline = Instant::now() + WATCHDOG;
            loop {
                thread::sleep(Duration::from_millis(500));
                let mut guard = ledger.lock().expect("proof ledger locks");
                if guard.done {
                    return;
                }
                if Instant::now() > deadline {
                    guard.fail("watchdog deadline exceeded before the proof completed");
                    let (line, _) = report_line(&guard);
                    println!("{line}");
                    std::process::exit(3);
                }
            }
        });
    }

    let core_slot: Arc<OnceLock<Arc<Core>>> = Arc::new(OnceLock::new());
    let setup_ledger = ledger.clone();

    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ipc_proof_ping, ipc_forbidden_probe])
        .setup(move |app| {
            let ledger = setup_ledger;
            let runtime = TauriChildViewRuntime::new(app.handle().clone());
            let injected_nonce =
                ledger.lock().expect("proof ledger locks").injected_nonce.clone();
            let policy_hooks = ChildViewPolicyHooks::new(
                Some(format!("window.__NCL_PROOF_NONCE = '{injected_nonce}';")),
                {
                    let ledger = ledger.clone();
                    let core_slot = core_slot.clone();
                    Arc::new(move |event| {
                        {
                            let mut guard = ledger.lock().expect("proof ledger locks");
                            guard.events.push(event_json(&event));
                            match event {
                                ChildViewPolicyEvent::PageLoadFinished { url } => {
                                    guard.load_finished_url = Some(url.to_string());
                                }
                                ChildViewPolicyEvent::PopupDenied { url } => {
                                    guard.popup_denied_url = Some(url.to_string());
                                    if url.as_str() != ESCAPE_POPUP_URL {
                                        guard.fail(format!("unexpected popup denial for {url}"));
                                    }
                                }
                                ChildViewPolicyEvent::DownloadDenied { .. } => {
                                    guard.download_denied = true;
                                }
                                ChildViewPolicyEvent::DocumentTitleChanged { title } => {
                                    // macOS reports the document's static
                                    // title first; only the proof-shaped
                                    // title is evidence. Everything else is
                                    // recorded and ignored.
                                    if title.starts_with("ncl-proof|") {
                                        guard.title = Some(title.clone());
                                        let shaped = title.starts_with(EXPECTED_TITLE_PREFIX)
                                            && title.ends_with(EXPECTED_TITLE_TAIL);
                                        guard.record(
                                            "document-title-evidence",
                                            json!({
                                                "title": title,
                                                "shapeMatchesExpectation": shaped,
                                            }),
                                        );
                                        if !shaped {
                                            guard.fail(format!(
                                                "document title evidence has the wrong shape: {title}"
                                            ));
                                        } else {
                                            let echo = format!("ping:{}", guard.injected_nonce);
                                            if !title.contains(&echo) {
                                                guard.fail(
                                                    "document title evidence does not echo the injected nonce",
                                                );
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Some(core) = core_slot.get() {
                            advance(core);
                        }
                    })
                },
            )
            .expect("proof policy hooks are bounded");

            let spec = ChildViewSpec::new(
                island_id(),
                host_window_id(),
                ChildViewLabel::new("main").expect("bounded label"),
                ChildViewLabel::new("proof-child").expect("bounded label"),
                ChildViewSource::bundled(BUNDLED_DOCUMENT).expect("bounded bundled document"),
                Some(*b"local-proof-16by"),
                Arc::new(|candidate: &Url| candidate.scheme() == "tauri"),
                policy_hooks,
            )
            .expect("bounded child-view spec");

            let adapter = Arc::new(ChildViewAdapter::new(runtime, spec, Arc::new(|_| {})));
            let generation = AttachGeneration::new(1).expect("generation 1 is valid");
            let coordinator = Arc::new(Mutex::new(
                NativeContentCoordinator::new(
                    DesiredState::new(
                        island_id(),
                        NativeContentKindId::new("proof:offline-document")
                            .expect("bounded kind id"),
                        CHILD_VIEW_CAPABILITIES,
                        desired_update(generation, DesiredPresence::Present, FocusIntent::Request),
                    )
                    .expect("bounded desired state"),
                ),
            ));

            let plan = match coordinator.lock().expect("proof coordinator locks").plan() {
                Ok(plan) => plan,
                Err(error) => {
                    let mut guard = ledger.lock().expect("proof ledger locks");
                    guard.fail(format!("initial plan failed: {error:?}"));
                    drop(guard);
                    exit_with(&ledger, 1);
                }
            };
            let apply = adapter.apply(&coordinator.lock().expect("proof coordinator locks"), &plan);
            match apply {
                Ok(receipt) => {
                    let applied = receipt
                        .steps()
                        .iter()
                        .filter(|step| step.outcome() == &OperationOutcome::Applied)
                        .count();
                    let mut guard = ledger.lock().expect("proof ledger locks");
                    guard.record(
                        "initial-apply",
                        json!({"appliedSteps": applied, "totalSteps": receipt.steps().len()}),
                    );
                    if applied != receipt.steps().len() {
                        guard.fail("the initial apply did not apply every step");
                    }
                }
                Err(error) => {
                    let mut guard = ledger.lock().expect("proof ledger locks");
                    guard.fail(format!("initial apply failed: {error:?}"));
                    drop(guard);
                    exit_with(&ledger, 1);
                }
            }

            let core_value = Arc::new(Core {
                ledger: ledger.clone(),
                adapter,
                coordinator,
                generation,
                app: app.handle().clone(),
            });
            app.manage(core_value.clone());
            let _ = core_slot.set(core_value);
            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(error) = result {
        let mut guard = ledger.lock().expect("proof ledger locks");
        guard.fail(format!("tauri run failed: {error}"));
        drop(guard);
        exit_with(&ledger, 2);
    }
}
