//! The bridge from a window hosting a child webview: the native-content island
//! shape, where Tauri no longer treats the window as a `WebviewWindow`.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use longhorn_bridge::{
    AuthorityEpoch, BridgeCancellationRequest, BridgeCommandEnvelope, BridgeEventEnvelope,
    BridgeStreamCursor, BridgeStreamSequence, WriteAuthority,
};
use longhorn_core::{BridgeJobId, BridgeRequestId, BridgeSessionId};
use longhorn_tauri_bridge::{
    BRIDGE_DOMAIN_EVENT, BridgeEventSink, BridgeHandlerAssembly, BridgeHostError,
    BridgeHostErrorCode, TauriBridgeEventSink, bridge_plugin,
};
use serde_json::{Value, json};
use tauri::{
    LogicalPosition, LogicalSize, Manager, Webview, WebviewUrl, WebviewWindowBuilder,
    test::MockRuntime,
    webview::{InvokeRequest, WebviewBuilder},
};

mod support;

use support::*;

const SESSION: &str = "session:current";

#[test]
fn island_window_ui_webview_drives_every_command_and_receives_events() {
    let dispatches = Arc::new(AtomicUsize::new(0));
    let (app, assembly, ui, _island) = island_app(Arc::clone(&dispatches));

    for (command, body) in invocations() {
        invoke(&ui, command, body)
            .unwrap_or_else(|error| panic!("{command} refused the UI webview: {error}"));
    }
    assert_eq!(
        dispatches.load(Ordering::SeqCst),
        3,
        "query, command and cancellation dispatched"
    );

    let session = BridgeSessionId::new(SESSION).unwrap();
    let event = BridgeEventEnvelope::new(
        BridgeStreamCursor::new(
            session.clone(),
            domain(),
            AuthorityEpoch::new(1).unwrap(),
            BridgeStreamSequence::new(2),
        ),
        json!({ "changed": true }),
    );
    assembly.publish_domain_event(&event).unwrap();
    assembly
        .publish_progress(&session, &domain(), &progress_event())
        .unwrap();
    assembly
        .publish_terminal(&session, &domain(), &terminal_event())
        .unwrap();

    // A child webview's label is not a delivery target.
    let sink = TauriBridgeEventSink::new(app.handle().clone());
    assert_eq!(
        sink.emit("preview-island", BRIDGE_DOMAIN_EVENT, json!(null))
            .unwrap_err()
            .code,
        BridgeHostErrorCode::EventPublication
    );
}

#[test]
fn island_child_webview_cannot_act_as_its_window() {
    let dispatches = Arc::new(AtomicUsize::new(0));
    let (_app, _assembly, ui, island) = island_app(Arc::clone(&dispatches));
    invoke(&ui, "longhorn_bridge_hello", json!({ "request": hello() })).unwrap();

    // Even with the window's live session id, every command refuses the child.
    for (command, body) in invocations() {
        let error: BridgeHostError =
            serde_json::from_value(invoke(&island, command, body).unwrap_err()).unwrap();
        assert_eq!(error.code, BridgeHostErrorCode::InvalidSession, "{command}");
        assert!(
            error.message.contains("preview-island"),
            "{command}: {}",
            error.message
        );
    }
    assert_eq!(dispatches.load(Ordering::SeqCst), 0);

    // The refused child hello did not replace or end the window's session.
    invoke(
        &ui,
        "longhorn_bridge_authority",
        json!({ "sessionId": SESSION }),
    )
    .unwrap();
}

type IslandApp = (
    tauri::App<MockRuntime>,
    Arc<BridgeHandlerAssembly<FixedAuthority>>,
    Webview<MockRuntime>,
    Webview<MockRuntime>,
);

/// One window "main" hosting its UI webview and a differently labelled child.
fn island_app(dispatches: Arc<AtomicUsize>) -> IslandApp {
    let app = tauri::test::mock_builder()
        .invoke_handler(tauri::generate_handler![
            longhorn_tauri_bridge::longhorn_bridge_hello,
            longhorn_tauri_bridge::longhorn_bridge_authority,
            longhorn_tauri_bridge::longhorn_bridge_query,
            longhorn_tauri_bridge::longhorn_bridge_command,
            longhorn_tauri_bridge::longhorn_bridge_cancel,
            longhorn_tauri_bridge::longhorn_bridge_resync,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let assembly = Arc::new(BridgeHandlerAssembly::with_event_sink(
        FixedAuthority {
            receipt: authority_receipt(capabilities(), WriteAuthority::Authoritative),
        },
        registered_domains(dispatches),
        Arc::new(TauriBridgeEventSink::new(app.handle().clone())),
    ));
    app.handle()
        .plugin(bridge_plugin(assembly.clone()))
        .unwrap();

    let window = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();
    let ui: Webview<MockRuntime> = window.as_ref().clone();
    let island = ui
        .window()
        .add_child(
            WebviewBuilder::new("preview-island", WebviewUrl::default()),
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(100.0, 100.0),
        )
        .unwrap();
    assert!(
        app.get_webview_window("main").is_none(),
        "the fixture must be a multiwebview window"
    );
    (app, assembly, ui, island)
}

/// One valid invocation of each bridge command, `hello` first.
fn invocations() -> [(&'static str, Value); 6] {
    [
        ("longhorn_bridge_hello", json!({ "request": hello() })),
        ("longhorn_bridge_authority", json!({ "sessionId": SESSION })),
        (
            "longhorn_bridge_query",
            json!({
                "route": "workspace.read",
                "request": query_request("request:query", SESSION, DOMAIN),
            }),
        ),
        (
            "longhorn_bridge_command",
            json!({
                "route": "workspace.write",
                "request": BridgeCommandEnvelope::new(
                    context("request:command", SESSION, DOMAIN),
                    AuthorityEpoch::new(1).unwrap(),
                    None,
                    None,
                    json!({}),
                ),
            }),
        ),
        (
            "longhorn_bridge_cancel",
            json!({
                "route": "workspace.cancel",
                "request": BridgeCancellationRequest::new(
                    context("request:cancel", SESSION, DOMAIN),
                    BridgeRequestId::new("request:job").unwrap(),
                    BridgeJobId::new("job:scan").unwrap(),
                ),
            }),
        ),
        (
            "longhorn_bridge_resync",
            json!({ "sessionId": SESSION, "domainId": DOMAIN }),
        ),
    ]
}

/// `get_ipc_response` takes `AsRef<Webview>`, which a bare child `Webview`
/// does not implement.
struct Invoking<'a>(&'a Webview<MockRuntime>);

impl AsRef<Webview<MockRuntime>> for Invoking<'_> {
    fn as_ref(&self) -> &Webview<MockRuntime> {
        self.0
    }
}

fn invoke(webview: &Webview<MockRuntime>, command: &str, body: Value) -> Result<(), Value> {
    tauri::test::get_ipc_response(
        &Invoking(webview),
        InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map(|_| ())
}
