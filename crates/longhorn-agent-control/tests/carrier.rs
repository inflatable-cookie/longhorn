//! End-to-end stdio carrier fixtures (g02.044): the built
//! `longhorn-agent-control-client` binary is spawned exactly as a harness
//! would spawn it — piped stdio, newline-delimited JSON-RPC — in front of a
//! live contract 022 router, and every reply is asserted equal to the same
//! exchange made directly over HTTP. That equality is the semantics-parity
//! proof: one server, one catalogue, no carrier-owned meaning.
//!
//! The listen fixture proves the position on `listen` and resources: they
//! are adapted by pass-through, so a subscribed harness sees
//! `notifications/resources/updated` from the carrier and cancellation
//! still ends the stream.

#![cfg(feature = "client")]

use std::{
    collections::BTreeSet,
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::{Router, body::Body, http::Request};
use http_body_util::BodyExt as _;
use longhorn_agent_control::{
    ActionReceipt, CommandResult, ControlHandler, ElementRef, EvaluateResult, InstanceToken,
    ListWindowsResult, PageState, ScreenshotResult, SelectionRegistry, SemanticNode,
    SnapshotResult, ToolError, WaitForResult, carrier::select_instance, control_router,
    publish_discovery,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};
use tower::ServiceExt as _;

/// Canned host authority: fixed answers plus the page-event ring the
/// server's listen loop polls through `evaluate` of `readEvents`.
#[derive(Clone)]
struct StubHandler {
    events: Arc<Mutex<Vec<Value>>>,
    next_seq: Arc<AtomicU64>,
}

impl StubHandler {
    fn empty_ring() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            next_seq: Arc::new(AtomicU64::new(1)),
        }
    }

    fn seeded_ring() -> Self {
        let handler = Self::empty_ring();
        handler.push_console("hello from stub");
        handler
    }

    fn push_console(&self, text: &str) {
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        self.events.lock().unwrap().push(json!({
            "seq": seq,
            "kind": "console",
            "level": "log",
            "text": text,
        }));
    }
}

fn semantic_root() -> SemanticNode {
    SemanticNode {
        element_ref: ElementRef::new("root").unwrap(),
        role: "document".to_owned(),
        name: None,
        value: None,
        states: BTreeSet::new(),
        children: Vec::new(),
    }
}

impl ControlHandler for StubHandler {
    async fn snapshot(
        &self,
        _request: longhorn_agent_control::SnapshotRequest,
    ) -> Result<SnapshotResult, ToolError> {
        Ok(SnapshotResult {
            window: longhorn_core::WindowId::new("main").unwrap(),
            webview: None,
            page: PageState {
                url: "http://localhost/".to_owned(),
                title: "stub".to_owned(),
            },
            root: semantic_root(),
        })
    }

    async fn click(
        &self,
        _request: longhorn_agent_control::ClickRequest,
    ) -> Result<ActionReceipt, ToolError> {
        Ok(ActionReceipt {})
    }

    async fn r#type(
        &self,
        _request: longhorn_agent_control::TypeRequest,
    ) -> Result<ActionReceipt, ToolError> {
        Ok(ActionReceipt {})
    }

    async fn press(
        &self,
        _request: longhorn_agent_control::PressRequest,
    ) -> Result<ActionReceipt, ToolError> {
        Ok(ActionReceipt {})
    }

    async fn scroll(
        &self,
        _request: longhorn_agent_control::ScrollRequest,
    ) -> Result<ActionReceipt, ToolError> {
        Ok(ActionReceipt {})
    }

    async fn drag(
        &self,
        _request: longhorn_agent_control::DragRequest,
    ) -> Result<ActionReceipt, ToolError> {
        Ok(ActionReceipt {})
    }

    async fn set_file_input(
        &self,
        _request: longhorn_agent_control::SetFileInputRequest,
    ) -> Result<ActionReceipt, ToolError> {
        Ok(ActionReceipt {})
    }

    async fn evaluate(
        &self,
        request: longhorn_agent_control::EvaluateRequest,
    ) -> Result<EvaluateResult, ToolError> {
        if let Some(since) = read_events_since(&request.js) {
            let events = self.events.lock().unwrap();
            let filtered: Vec<Value> = events
                .iter()
                .filter(|event| event["seq"].as_u64().unwrap_or(0) > since)
                .cloned()
                .collect();
            let next_seq = self.next_seq.load(Ordering::SeqCst);
            return Ok(EvaluateResult {
                value: Value::String(
                    json!({
                        "events": filtered,
                        "nextSeq": next_seq,
                        "dropped": 0
                    })
                    .to_string(),
                ),
            });
        }
        Ok(EvaluateResult {
            value: Value::String(request.js),
        })
    }

    async fn wait_for(
        &self,
        _request: longhorn_agent_control::WaitForRequest,
    ) -> Result<WaitForResult, ToolError> {
        Ok(WaitForResult {})
    }

    async fn screenshot(
        &self,
        _request: longhorn_agent_control::ScreenshotRequest,
    ) -> Result<ScreenshotResult, ToolError> {
        Ok(ScreenshotResult {
            window: longhorn_core::WindowId::new("main").unwrap(),
            png: vec![1, 2, 3],
        })
    }

    async fn command(
        &self,
        _request: longhorn_agent_control::CommandRequest,
    ) -> Result<CommandResult, ToolError> {
        Ok(CommandResult { output: None })
    }

    async fn list_windows(
        &self,
        _request: longhorn_agent_control::ListWindowsRequest,
    ) -> Result<ListWindowsResult, ToolError> {
        Ok(ListWindowsResult {
            windows: Vec::new(),
        })
    }

    async fn resize_window(
        &self,
        _request: longhorn_agent_control::ResizeWindowRequest,
    ) -> Result<ActionReceipt, ToolError> {
        Ok(ActionReceipt {})
    }
}

fn read_events_since(js: &str) -> Option<u64> {
    let start = js.find("readEvents(")?;
    let rest = &js[start + "readEvents(".len()..];
    let end = rest.find(')')?;
    rest[..end].trim().parse().ok()
}

/// One live server plus its discovery file, and the spawned carrier.
struct CarrierSession {
    dir: TempDir,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    server: tokio::task::JoinHandle<()>,
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
}

impl CarrierSession {
    async fn spawn(app_id: &str, stub: StubHandler) -> Self {
        let dir = TempDir::new().unwrap();
        let discovery_dir = dir.path().join("agent-control");
        let token = InstanceToken::generate().unwrap();
        let router = control_router(stub, token.clone(), SelectionRegistry::new("test"));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });
        let _instance = publish_discovery(&discovery_dir, app_id, port, token).unwrap();
        // The publish handle is intentionally detached: it would remove the
        // file on drop, but the session asserts against a live server until
        // the child exits. `TempDir` cleanup removes the directory anyway.
        std::mem::forget(_instance);

        let mut child = Command::new(env!("CARGO_BIN_EXE_longhorn-agent-control-client"))
            .arg("--discovery-dir")
            .arg(&discovery_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        Self {
            dir,
            shutdown: Some(shutdown_tx),
            server,
            child,
            stdin,
            stdout_lines: BufReader::new(stdout).lines(),
        }
    }

    fn discovery_dir(&self) -> std::path::PathBuf {
        self.dir.path().join("agent-control")
    }

    async fn send(&mut self, message: &Value) {
        let mut text = serde_json::to_string(message).unwrap();
        text.push('\n');
        self.stdin.write_all(text.as_bytes()).await.unwrap();
        self.stdin.flush().await.unwrap();
    }

    /// Reads the next stdout line, failing the test on timeout: a missing
    /// reply is a lost exchange, never a slow one.
    async fn next(&mut self) -> Value {
        let line = timeout(Duration::from_secs(10), self.stdout_lines.next_line())
            .await
            .expect("carrier answered within ten seconds")
            .unwrap()
            .expect("carrier stdout stays open");
        serde_json::from_str(&line).unwrap()
    }

    /// Reads until the reply carrying `id` arrives, skipping interleaved
    /// server-to-client notifications.
    async fn next_for(&mut self, id: i64) -> Value {
        loop {
            let message = self.next().await;
            if message.get("id") == Some(&json!(id)) {
                return message;
            }
        }
    }

    async fn request(&mut self, id: i64, method: &str, params: Value) -> Value {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await;
        self.next_for(id).await
    }

    async fn close(mut self) {
        drop(self.stdin);
        timeout(Duration::from_secs(10), self.child.wait())
            .await
            .expect("carrier exits after stdin closes")
            .unwrap();
        let _ = self.shutdown.take().map(|tx| tx.send(()));
        timeout(Duration::from_secs(10), self.server)
            .await
            .expect("server shuts down")
            .unwrap();
    }
}

/// The `_meta` envelope revision 2026-07-28 requires, mirroring what
/// rmcp's own HTTP client injects on every post-`initialize` request.
fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "carrier-e2e", "version": "0.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

/// One direct HTTP exchange against the same router, returning the reply
/// message the way the server framed it (first SSE `data:` payload).
async fn direct(router: Router, token: &InstanceToken, method: &str, params: Value) -> Value {
    let mut params = params;
    if let Some(object) = params.as_object_mut() {
        object.entry("_meta".to_owned()).or_insert_with(meta);
    }
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    })
    .to_string();
    let mut request = Request::builder()
        .method("POST")
        .uri("http://localhost/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("authorization", format!("Bearer {}", token.as_str()))
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", method);
    // Same `Mcp-Name` sourcing the carrier uses: the strict path
    // requires it for `tools/call` and `resources/read`.
    let name = match method {
        "tools/call" => params.get("name").and_then(Value::as_str),
        "resources/read" => params.get("uri").and_then(Value::as_str),
        _ => None,
    };
    if let Some(name) = name {
        request = request.header("mcp-name", name);
    }
    let request = request.body(Body::from(body)).unwrap();
    // Errors (unknown tools, bad params) arrive as JSON-RPC error objects
    // on non-200 statuses; the carrier forwards those same objects, so the
    // parity comparison reads the body either way.
    let response = router.oneshot(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    // Success rides SSE `data:`; errors arrive as a plain JSON body.
    let mut reply: Value = match text.lines().find_map(|line| line.strip_prefix("data: ")) {
        Some(data) => serde_json::from_str(data).unwrap(),
        None => serde_json::from_str(&text)
            .unwrap_or_else(|_| panic!("direct {method} answered no JSON-RPC message: {text}")),
    };
    // The direct path uses id 1; normalize so replies compare by shape.
    if let Some(id) = reply.get_mut("id") {
        *id = Value::Null;
    }
    reply
}

fn normalize_id(mut reply: Value) -> Value {
    if let Some(id) = reply.get_mut("id") {
        *id = Value::Null;
    }
    reply
}

#[tokio::test]
async fn carrier_proxies_tools_with_http_parity() {
    let stub = StubHandler::seeded_ring();
    let mut session = CarrierSession::spawn("dev.example.carrier", stub).await;

    let init = session
        .request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": {"name": "carrier-e2e", "version": "0.0.0"},
            }),
        )
        .await;
    assert_eq!(
        init["result"]["serverInfo"]["name"],
        json!("longhorn-agent-control")
    );
    // No session id is ever minted or echoed, through the carrier or not.
    assert!(init["result"].get("sessionId").is_none());
    session
        .send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await;

    // The direct router for parity comparison.
    let token = select_instance(&session.discovery_dir(), Some("dev.example.carrier"))
        .unwrap()
        .file
        .token;
    let parity_router = || {
        control_router(
            StubHandler::seeded_ring(),
            token.clone(),
            SelectionRegistry::new("test"),
        )
    };

    let listed = session.request(2, "tools/list", json!({})).await;
    let direct_listed = direct(parity_router(), &token, "tools/list", json!({})).await;
    assert_eq!(normalize_id(listed.clone()), direct_listed);
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for expected in [
        "snapshot",
        "click",
        "type",
        "press",
        "scroll",
        "drag",
        "evaluate",
        "wait_for",
        "screenshot",
        "command",
        "list_windows",
        "resize_window",
        "answer_selection",
        "reject_selection",
    ] {
        assert!(names.contains(&expected), "carrier hides {expected}");
    }

    let snapshot = session
        .request(
            3,
            "tools/call",
            json!({"name": "snapshot", "arguments": {}}),
        )
        .await;
    let direct_snapshot = direct(
        parity_router(),
        &token,
        "tools/call",
        json!({"name": "snapshot", "arguments": {}}),
    )
    .await;
    assert_eq!(normalize_id(snapshot.clone()), direct_snapshot);
    let text = snapshot["result"]["content"][0]["text"].as_str().unwrap();
    let snapshot_value: Value = serde_json::from_str(text).unwrap();
    assert_eq!(snapshot_value["page"]["title"], json!("stub"));

    // Typed server errors pass through as results, not transport failures.
    let unknown = session
        .request(
            4,
            "tools/call",
            json!({"name": "no-such-tool", "arguments": {}}),
        )
        .await;
    let direct_unknown = direct(
        parity_router(),
        &token,
        "tools/call",
        json!({"name": "no-such-tool", "arguments": {}}),
    )
    .await;
    assert_eq!(normalize_id(unknown.clone()), direct_unknown);
    assert!(unknown.get("error").is_some());

    session.close().await;
}

#[tokio::test]
async fn carrier_adapts_listen_and_resources_by_pass_through() {
    let stub = StubHandler::empty_ring();
    let mut session = CarrierSession::spawn("dev.example.carrier", stub.clone()).await;

    let init = session
        .request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": {"name": "carrier-e2e", "version": "0.0.0"},
            }),
        )
        .await;
    assert!(init.get("result").is_some());
    session
        .send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await;

    let resources = session.request(2, "resources/list", json!({})).await;
    let uris: Vec<&str> = resources["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect();
    assert!(uris.contains(&"longhorn://agent-control/console"));

    // The listen request stays pending while its SSE stream is open; the
    // pushed console event must arrive as a resource notification.
    session
        .send(&json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "subscriptions/listen",
            "params": {
                "notifications": {
                    "resourceSubscriptions": ["longhorn://agent-control/console"],
                },
            },
        }))
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    stub.push_console("carrier-live");
    let notification = timeout(Duration::from_secs(10), async {
        loop {
            let message = session.next().await;
            if message.get("method") == Some(&json!("notifications/resources/updated")) {
                return message;
            }
        }
    })
    .await
    .expect("listen delivers the subscribed event");
    assert_eq!(
        notification["params"]["uri"],
        json!("longhorn://agent-control/console")
    );

    // The event body itself reads back through the carrier.
    let read = session
        .request(
            3,
            "resources/read",
            json!({"uri": "longhorn://agent-control/console"}),
        )
        .await;
    let text = read["result"]["contents"][0]["text"].as_str().unwrap();
    let body: Value = serde_json::from_str(text).unwrap();
    assert!(
        body["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["text"] == json!("carrier-live"))
    );

    // Cancelling the pending listen ends the stream; the carrier stays up.
    session
        .send(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {"requestId": 10},
        }))
        .await;
    let listed = session.request(11, "tools/list", json!({})).await;
    assert!(!listed["result"]["tools"].as_array().unwrap().is_empty());

    session.close().await;
}

#[tokio::test]
async fn carrier_exits_2_without_a_live_instance() {
    let dir = TempDir::new().unwrap();
    let discovery_dir = dir.path().join("agent-control");
    let output = Command::new(env!("CARGO_BIN_EXE_longhorn-agent-control-client"))
        .arg("--discovery-dir")
        .arg(&discovery_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no live agent-control instance"),
        "{stderr}"
    );
}

#[tokio::test]
async fn carrier_exits_2_on_ambiguous_instances() {
    let dir = TempDir::new().unwrap();
    let discovery_dir = dir.path().join("agent-control");
    let first = publish_discovery(
        &discovery_dir,
        "dev.example.one",
        49152,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();
    let second = publish_discovery(
        &discovery_dir,
        "dev.example.two",
        49153,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_longhorn-agent-control-client"))
        .arg("--discovery-dir")
        .arg(&discovery_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("more than one live"), "{stderr}");

    // `--app-id` disambiguates: selection resolves without spawning.
    let selected = select_instance(&discovery_dir, Some("dev.example.two")).unwrap();
    assert_eq!(selected.file.app_id, "dev.example.two");

    first.remove().unwrap();
    second.remove().unwrap();
}
