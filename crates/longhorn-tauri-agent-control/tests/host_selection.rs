//! Fixtures for the Rust-side picker entry (g02.047).
//!
//! Agent origin publishes a pending request bound to the invoking webview
//! and settles through the same result shape as the JS route. Human,
//! missing, and malformed origin return UsePlugin without touching the
//! registry. There is no native-dialog fallback on the agent path.

#![cfg(feature = "agent-control")]

use std::{
    fs::File,
    time::{Duration, Instant},
};

use longhorn_tauri_agent_control::{
    BeginSelectionArgs, HostSelection, PendingSelection, SelectionKind, SelectionOrigin,
    SelectionRegistry, ToolError, begin_host_selection,
};
use serde_json::{Value, json};
use tauri::{
    Manager, Runtime, State, Webview, WebviewUrl, WebviewWindowBuilder, test::MockRuntime,
};
use tempfile::TempDir;

struct Harness {
    _app: tauri::App<MockRuntime>,
    webview: Webview<MockRuntime>,
    registry: SelectionRegistry,
}

fn harness() -> Harness {
    harness_with(SelectionRegistry::new("app:1"))
}

fn harness_with(registry: SelectionRegistry) -> Harness {
    let app = tauri::test::mock_app();
    WebviewWindowBuilder::new(app.handle(), "main", Default::default())
        .build()
        .unwrap();
    let webview = app.get_webview("main").expect("main webview");
    Harness {
        _app: app,
        webview,
        registry,
    }
}

fn save_args() -> BeginSelectionArgs {
    BeginSelectionArgs {
        kind: SelectionKind::Save,
        directory: false,
        multiple: false,
        filters: Vec::new(),
        title: Some("Export backup".to_owned()),
        default_path: None,
    }
}

fn path_string(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

async fn wait_pending(registry: &SelectionRegistry) -> PendingSelection {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(pending) = registry.pending().into_iter().next() {
            return pending;
        }
        assert!(
            Instant::now() < deadline,
            "pending selection never appeared"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

fn spawn_begin(
    origin: SelectionOrigin,
    webview: Webview<MockRuntime>,
    registry: SelectionRegistry,
    options: BeginSelectionArgs,
) -> tokio::task::JoinHandle<HostSelection> {
    tokio::spawn(async move { begin_host_selection(origin, &webview, &registry, options).await })
}

#[tokio::test]
async fn agent_origin_answers_with_window_and_webview_and_no_plugin_arm() {
    let harness = harness();
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("backup.zip");
    let expected = path_string(&target);
    let generation = harness.registry.generation();

    let begin = spawn_begin(
        SelectionOrigin::Agent,
        harness.webview.clone(),
        harness.registry.clone(),
        save_args(),
    );
    let pending = wait_pending(&harness.registry).await;
    assert_eq!(pending.window.as_deref(), Some("main"));
    assert_eq!(pending.webview.as_deref(), Some("main"));
    assert_eq!(pending.kind, SelectionKind::Save);
    assert!(harness.registry.generation() > generation);
    harness
        .registry
        .answer(&pending.id, std::slice::from_ref(&expected))
        .unwrap();
    assert!(!target.exists(), "save must not write the path");

    match begin.await.unwrap() {
        HostSelection::UsePlugin => panic!("agent origin must not fall back to the plugin"),
        HostSelection::Agent(result) => {
            assert_eq!(result.unwrap(), Value::String(expected));
        }
    }
    assert!(harness.registry.pending().is_empty());
}

#[tokio::test]
async fn human_absent_and_malformed_origin_use_the_plugin_and_publish_nothing() {
    let harness = harness();
    let generation = harness.registry.generation();
    let origins = [
        SelectionOrigin::Human,
        SelectionOrigin::from_optional_json(None),
        SelectionOrigin::from_json(&json!("human")),
        SelectionOrigin::from_json(&json!(null)),
        SelectionOrigin::from_json(&json!(42)),
        SelectionOrigin::from_json(&json!({"origin": "agent"})),
        serde_json::from_value(json!("AGENT")).unwrap(),
    ];
    for origin in origins {
        let outcome =
            begin_host_selection(origin, &harness.webview, &harness.registry, save_args()).await;
        assert_eq!(outcome, HostSelection::UsePlugin, "{origin:?}");
        assert!(harness.registry.pending().is_empty());
        assert_eq!(harness.registry.generation(), generation);
    }
}

#[tokio::test]
async fn reject_yields_plugin_null() {
    let harness = harness();
    let begin = spawn_begin(
        SelectionOrigin::Agent,
        harness.webview.clone(),
        harness.registry.clone(),
        save_args(),
    );
    let pending = wait_pending(&harness.registry).await;
    harness.registry.reject(&pending.id).unwrap();
    match begin.await.unwrap() {
        HostSelection::Agent(Ok(Value::Null)) => {}
        other => panic!("expected agent null, got {other:?}"),
    }
}

#[tokio::test]
async fn expiry_fails_typed_and_settles_once() {
    let harness = harness_with(SelectionRegistry::with_ttl(
        "app:1",
        Duration::from_millis(200),
    ));
    let begin = spawn_begin(
        SelectionOrigin::Agent,
        harness.webview.clone(),
        harness.registry.clone(),
        save_args(),
    );
    let pending = wait_pending(&harness.registry).await;
    match begin.await.unwrap() {
        HostSelection::Agent(Err(error)) => {
            let body: Value = serde_json::from_str(&error).unwrap();
            assert_eq!(body["error"], "expiredSelection");
        }
        other => panic!("expected typed expiry, got {other:?}"),
    }
    assert!(matches!(
        harness
            .registry
            .answer(&pending.id, &["/nope".to_owned()])
            .unwrap_err(),
        ToolError::ExpiredSelection { .. }
    ));
}

#[tokio::test]
async fn shutdown_fails_typed() {
    let harness = harness();
    let begin = spawn_begin(
        SelectionOrigin::Agent,
        harness.webview.clone(),
        harness.registry.clone(),
        save_args(),
    );
    let _pending = wait_pending(&harness.registry).await;
    harness.registry.shutdown();
    match begin.await.unwrap() {
        HostSelection::Agent(Err(error)) => {
            let body: Value = serde_json::from_str(&error).unwrap();
            assert_eq!(body["error"], "unsupported");
        }
        other => panic!("expected typed shutdown, got {other:?}"),
    }
    assert!(harness.registry.pending().is_empty());
}

#[tokio::test]
async fn duplicate_answer_fails_typed_after_settlement() {
    let harness = harness();
    let dir = TempDir::new().unwrap();
    let target = path_string(&dir.path().join("out.txt"));
    let begin = spawn_begin(
        SelectionOrigin::Agent,
        harness.webview.clone(),
        harness.registry.clone(),
        save_args(),
    );
    let pending = wait_pending(&harness.registry).await;
    harness
        .registry
        .answer(&pending.id, std::slice::from_ref(&target))
        .unwrap();
    let again = harness
        .registry
        .answer(&pending.id, std::slice::from_ref(&target));
    assert!(matches!(
        again.unwrap_err(),
        ToolError::SettledSelection { .. }
    ));
    match begin.await.unwrap() {
        HostSelection::Agent(Ok(Value::String(path))) => assert_eq!(path, target),
        other => panic!("expected answered path, got {other:?}"),
    }
}

#[tokio::test]
async fn wrong_instance_answer_is_typed_and_leaves_the_request() {
    let harness = harness();
    let other = SelectionRegistry::new("app:2");
    let begin = spawn_begin(
        SelectionOrigin::Agent,
        harness.webview.clone(),
        harness.registry.clone(),
        save_args(),
    );
    let pending = wait_pending(&harness.registry).await;
    assert!(matches!(
        other
            .answer(&pending.id, &["/nope".to_owned()])
            .unwrap_err(),
        ToolError::UnknownSelection { .. }
    ));
    assert_eq!(harness.registry.pending().len(), 1);
    harness.registry.reject(&pending.id).unwrap();
    match begin.await.unwrap() {
        HostSelection::Agent(Ok(Value::Null)) => {}
        other => panic!("expected reject null, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_save_answer_fails_typed_then_settles_once() {
    let harness = harness();
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("keep.txt")).unwrap();
    let target = path_string(dir.path());
    let begin = spawn_begin(
        SelectionOrigin::Agent,
        harness.webview.clone(),
        harness.registry.clone(),
        save_args(),
    );
    let pending = wait_pending(&harness.registry).await;
    let error = harness
        .registry
        .answer(&pending.id, std::slice::from_ref(&target))
        .unwrap_err();
    assert!(matches!(error, ToolError::MalformedSelection { .. }));
    assert_eq!(harness.registry.pending().len(), 1);
    let file_target = path_string(&dir.path().join("out.txt"));
    harness
        .registry
        .answer(&pending.id, std::slice::from_ref(&file_target))
        .unwrap();
    match begin.await.unwrap() {
        HostSelection::Agent(Ok(Value::String(path))) => assert_eq!(path, file_target),
        other => panic!("expected answered path, got {other:?}"),
    }
}

/// Documented consumer command: missing `origin` is human.
#[tauri::command]
async fn export_backup<R: Runtime>(
    webview: Webview<R>,
    registry: State<'_, SelectionRegistry>,
    origin: Option<SelectionOrigin>,
) -> Result<String, String> {
    match begin_host_selection(origin.unwrap_or_default(), &webview, &registry, save_args()).await {
        HostSelection::UsePlugin => Ok("use-plugin".to_owned()),
        HostSelection::Agent(_) => Ok("agent".to_owned()),
    }
}

#[tokio::test]
async fn missing_origin_key_on_a_real_command_is_human() {
    let registry = SelectionRegistry::new("app:1");
    let generation = registry.generation();
    let app = tauri::test::mock_builder()
        .manage(registry.clone())
        .invoke_handler(tauri::generate_handler![export_backup])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();
    let response = tauri::test::get_ipc_response(
        &webview,
        tauri::webview::InvokeRequest {
            cmd: "export_backup".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({})),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .unwrap_or_else(|error| panic!("missing origin key must not fail invoke: {error:?}"));
    let outcome: String = response.deserialize().unwrap();
    assert_eq!(outcome, "use-plugin");
    assert!(registry.pending().is_empty());
    assert_eq!(registry.generation(), generation);
}
