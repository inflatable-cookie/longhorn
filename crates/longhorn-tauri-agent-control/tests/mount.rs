//! End-to-end mount fixtures (Card 230): a mock-runtime app composes the
//! plugin, and the fixtures drive the mounted server over real loopback
//! HTTP — discovery lifecycle, the full tool listing, typed `Unsupported`
//! for unwired tools, `command` through the host bridge, window scope, and
//! the guard's rejection paths.
//!
//! The HTTP client is a minimal blocking one over `std::net::TcpStream` so
//! the fixture adds no client dependency: one POST per connection,
//! `Connection: close`, chunked bodies de-framed by hand.

#![cfg(feature = "dev")]

use std::{
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use longhorn_core::CommandId;
use longhorn_tauri_agent_control::{
    AgentControlConfig, CommandBridge, NoCommandBridge, mount_agent_control,
};
use serde_json::{Value, json};
use tauri::WebviewWindowBuilder;
use tempfile::TempDir;

type Invocation = (String, Option<Value>);

/// Records every invocation and answers with a fixed output payload.
#[derive(Clone, Default)]
struct RecordingBridge {
    invocations: Arc<Mutex<Vec<Invocation>>>,
}

impl CommandBridge for RecordingBridge {
    fn invoke_command(
        &self,
        command: &CommandId,
        argument: Option<Value>,
    ) -> Result<Option<Value>, longhorn_tauri_agent_control::ToolError> {
        self.invocations
            .lock()
            .unwrap()
            .push((command.as_str().to_owned(), argument.clone()));
        Ok(Some(
            json!({ "invoked": command.as_str(), "argument": argument }),
        ))
    }
}

struct Mounted {
    state_root: TempDir,
    port: u16,
    token: String,
    discovery_path: PathBuf,
    handle: longhorn_tauri_agent_control::AgentControlHandle,
    bridge: RecordingBridge,
}

/// One blocking MCP POST against the mounted server.
struct McpPost {
    port: u16,
    token: Option<String>,
    origin: Option<String>,
}

struct McpReply {
    status: u16,
    body: String,
}

impl McpReply {
    /// First SSE `data:` payload as JSON.
    fn json(&self) -> Value {
        let data = self
            .body
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap_or_else(|| panic!("no SSE data line in: {}", self.body));
        serde_json::from_str(data).unwrap()
    }
}

impl McpPost {
    fn authed(port: u16, token: &str) -> Self {
        Self {
            port,
            token: Some(token.to_owned()),
            origin: None,
        }
    }

    fn unauthenticated(port: u16) -> Self {
        Self {
            port,
            token: None,
            origin: None,
        }
    }

    fn origin(mut self, origin: &str) -> Self {
        self.origin = Some(origin.to_owned());
        self
    }

    fn post(&self, method: &str, name: Option<&str>, params: Value) -> McpReply {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        })
        .to_string();
        let mut request = format!(
            "POST /mcp HTTP/1.1\r\nhost: 127.0.0.1:{}\r\ncontent-type: application/json\r\naccept: application/json, text/event-stream\r\nmcp-protocol-version: 2026-07-28\r\nmcp-method: {method}\r\nconnection: close\r\ncontent-length: {}\r\n",
            self.port,
            body.len(),
        );
        if let Some(name) = name {
            request.push_str(&format!("mcp-name: {name}\r\n"));
        }
        if let Some(token) = &self.token {
            request.push_str(&format!("authorization: Bearer {token}\r\n"));
        }
        if let Some(origin) = &self.origin {
            request.push_str(&format!("origin: {origin}\r\n"));
        }
        request.push_str("\r\n");
        request.push_str(&body);

        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let raw = String::from_utf8(raw).unwrap();
        let (head, body) = raw.split_once("\r\n\r\n").expect("HTTP response framing");
        let status: u16 = head
            .split_whitespace()
            .nth(1)
            .expect("HTTP status line")
            .parse()
            .unwrap();
        let body = if head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
        {
            dechunk(body)
        } else {
            body.to_owned()
        };
        McpReply { status, body }
    }

    fn tools_list(&self) -> McpReply {
        self.post("tools/list", None, json!({ "_meta": meta() }))
    }

    fn call(&self, name: &str, arguments: Value) -> McpReply {
        self.post(
            "tools/call",
            Some(name),
            json!({ "name": name, "arguments": arguments, "_meta": meta() }),
        )
    }

    fn resources_list(&self) -> McpReply {
        self.post("resources/list", None, json!({ "_meta": meta() }))
    }

    /// Opens `subscriptions/listen` and reads until the socket times out —
    /// the stream is long-lived, so the acknowledgment is the proof it
    /// accepted the filter.
    fn listen(&self, notifications: Value) -> McpReply {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "subscriptions/listen",
            "params": { "notifications": notifications, "_meta": meta() },
        })
        .to_string();
        let mut request = format!(
            "POST /mcp HTTP/1.1\r\nhost: 127.0.0.1:{}\r\ncontent-type: application/json\r\naccept: application/json, text/event-stream\r\nmcp-protocol-version: 2026-07-28\r\nmcp-method: subscriptions/listen\r\nconnection: close\r\ncontent-length: {}\r\n",
            self.port,
            body.len(),
        );
        if let Some(token) = &self.token {
            request.push_str(&format!("authorization: Bearer {token}\r\n"));
        }
        request.push_str("\r\n");
        request.push_str(&body);
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(400)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut raw = Vec::new();
        let _ = stream.read_to_end(&mut raw);
        let raw = String::from_utf8(raw).unwrap_or_default();
        let status = raw
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        McpReply { status, body: raw }
    }
}

/// The `_meta` envelope revision 2026-07-28 requires (Card 227 capture).
fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "mount-fixture", "version": "0.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

/// De-frames one HTTP/1.1 chunked body.
fn dechunk(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some((size_line, tail)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim(), 16).expect("chunk size");
        if size == 0 {
            break;
        }
        out.push_str(&tail[..size]);
        rest = &tail[size + 2..];
    }
    out
}

/// The tool result's first content block as JSON, and its `isError` flag.
fn tool_content(reply: &McpReply) -> (bool, Value) {
    let payload = reply.json();
    let result = &payload["result"];
    let is_error = result["isError"].as_bool().unwrap_or(false);
    let text = result["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text content in: {payload}"));
    (is_error, serde_json::from_str(text).unwrap())
}

/// Mounts the plugin on a mock app with one `main` window and waits for the
/// discovery file to appear.
fn mount() -> (tauri::AppHandle<tauri::test::MockRuntime>, Mounted) {
    let app = tauri::test::mock_app();
    WebviewWindowBuilder::new(app.handle(), "main", Default::default())
        .build()
        .unwrap();

    let state_root = tempfile::tempdir().unwrap();
    let bridge = RecordingBridge::default();
    let handle = mount_agent_control(
        app.handle(),
        AgentControlConfig::new("longhorn-mount-fixture")
            .with_state_root(state_root.path().to_owned()),
        Arc::new(bridge.clone()),
    )
    .unwrap();

    let discovery_dir = state_root.path().join("agent-control");
    let discovery_path = wait_for_discovery(&discovery_dir);
    let discovery: Value =
        serde_json::from_str(&std::fs::read_to_string(&discovery_path).unwrap()).unwrap();
    assert_eq!(discovery["appId"], "longhorn-mount-fixture");
    assert_eq!(discovery["pid"], std::process::id());
    assert_eq!(discovery["schemaVersion"], 1);
    let port = discovery["port"].as_u64().unwrap() as u16;
    assert!(port > 0);
    let token = discovery["token"].as_str().unwrap().to_owned();
    assert!(!token.is_empty());

    (
        app.handle().clone(),
        Mounted {
            state_root,
            port,
            token,
            discovery_path,
            handle,
            bridge,
        },
    )
}

fn wait_for_discovery(discovery_dir: &Path) -> PathBuf {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(mut entries) = discovery_dir
            .read_dir()
            .map(|entries| entries.flatten().collect::<Vec<_>>())
            && let Some(entry) = entries.pop()
        {
            return entry.path();
        }
        assert!(Instant::now() < deadline, "discovery file never appeared");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn mounted_server_serves_the_vocabulary_and_window_scope() {
    let (_app, mounted) = mount();
    let client = McpPost::authed(mounted.port, &mounted.token);

    let list = client.tools_list();
    assert_eq!(list.status, 200, "{}", list.body);
    let list_payload = list.json();
    let tools: Vec<&str> = list_payload["result"]["tools"]
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
    ] {
        assert!(tools.contains(&expected), "missing tool {expected}");
    }

    // Semantic tools are wired through the shim. The mock runtime has no
    // WKWebView, so evaluate fails typed — not the old g02.032 Unsupported.
    let reply = client.call("snapshot", json!({}));
    assert_eq!(reply.status, 200, "{}", reply.body);
    let (is_error, content) = tool_content(&reply);
    assert!(
        is_error,
        "snapshot must fail typed on the mock runtime: {content}"
    );
    assert_ne!(
        content["error"], "unsupported",
        "snapshot is wired; mock evaluate fails as evaluationFailed: {content}"
    );
    assert_eq!(content["error"], "evaluationFailed");

    // `command` reaches the host bridge and returns its output.
    let reply = client.call(
        "command",
        json!({ "command": "proof.ping", "argument": { "note": "hello" } }),
    );
    let (is_error, content) = tool_content(&reply);
    assert!(!is_error, "command must succeed: {content}");
    assert_eq!(content["output"]["invoked"], "proof.ping");
    assert_eq!(content["output"]["argument"]["note"], "hello");
    let invocations = mounted.bridge.invocations.lock().unwrap();
    assert_eq!(
        invocations.as_slice(),
        &[("proof.ping".to_owned(), Some(json!({ "note": "hello" })))]
    );
    drop(invocations);

    // Window scope: the mock `main` window lists and resizes.
    let reply = client.call("list_windows", json!({}));
    let (is_error, content) = tool_content(&reply);
    assert!(!is_error, "list_windows must succeed: {content}");
    let windows = content["windows"].as_array().unwrap();
    assert_eq!(windows.len(), 1, "{content}");
    assert_eq!(windows[0]["window"], "main");

    let reply = client.call(
        "resize_window",
        json!({ "window": "main", "width": 640.0, "height": 480.0 }),
    );
    let (is_error, content) = tool_content(&reply);
    assert!(!is_error, "resize_window must succeed: {content}");

    let reply = client.resources_list();
    assert_eq!(reply.status, 200, "{}", reply.body);
    let resources = reply.json()["result"]["resources"]
        .as_array()
        .cloned()
        .unwrap();
    let uris: Vec<&str> = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect();
    assert!(uris.contains(&"longhorn://agent-control/console"));
    assert!(uris.contains(&"longhorn://agent-control/error"));
    assert!(uris.contains(&"longhorn://agent-control/navigation"));

    let listen = client.listen(json!({
        "resourceSubscriptions": ["longhorn://agent-control/console"]
    }));
    assert_eq!(listen.status, 200, "{}", listen.body);
    assert!(
        listen
            .body
            .contains("notifications/subscriptions/acknowledged"),
        "listen stream must acknowledge: {}",
        listen.body
    );

    // An unknown window fails typed.
    let reply = client.call(
        "resize_window",
        json!({ "window": "missing", "width": 640.0, "height": 480.0 }),
    );
    let (is_error, content) = tool_content(&reply);
    assert!(is_error, "unknown window must fail typed: {content}");
    assert_eq!(content["error"], "unknownWindow");
    assert_eq!(content["window"], "missing");

    // Clean shutdown removes the discovery file.
    let receipt = mounted.handle.shutdown().unwrap();
    assert_eq!(receipt.bound.ip().to_string(), "127.0.0.1");
    assert_eq!(receipt.bound.port(), mounted.port);
    assert!(!mounted.discovery_path.exists());
    drop(mounted.state_root);
}

#[test]
fn guard_rejects_unauthenticated_and_foreign_origin() {
    let (_app, mounted) = mount();

    let reply = McpPost::unauthenticated(mounted.port).tools_list();
    assert_eq!(reply.status, 401, "{}", reply.body);

    let reply = McpPost::authed(mounted.port, "wrong-token").tools_list();
    assert_eq!(reply.status, 401, "{}", reply.body);

    let reply = McpPost::authed(mounted.port, &mounted.token)
        .origin("https://attacker.example")
        .tools_list();
    assert_eq!(reply.status, 403, "{}", reply.body);

    let reply = McpPost::authed(mounted.port, &mounted.token)
        .origin("http://localhost:3000")
        .tools_list();
    assert_eq!(reply.status, 200, "{}", reply.body);

    mounted.handle.shutdown().unwrap();
    drop(mounted.state_root);
}

/// The legitimate no-command composition: an app without a contract-006
/// registry mounts with `NoCommandBridge`, and `command` answers a typed
/// `Unsupported` naming the absence — not a panic, not a guessed failure.
#[test]
fn no_command_bridge_answers_typed_unsupported() {
    let app = tauri::test::mock_app();
    WebviewWindowBuilder::new(app.handle(), "main", Default::default())
        .build()
        .unwrap();

    let state_root = tempfile::tempdir().unwrap();
    let handle = mount_agent_control(
        app.handle(),
        AgentControlConfig::new("longhorn-no-command-fixture")
            .with_state_root(state_root.path().to_owned()),
        Arc::new(NoCommandBridge),
    )
    .unwrap();

    let discovery_dir = state_root.path().join("agent-control");
    let discovery_path = wait_for_discovery(&discovery_dir);
    let discovery: Value =
        serde_json::from_str(&std::fs::read_to_string(&discovery_path).unwrap()).unwrap();
    let port = discovery["port"].as_u64().unwrap() as u16;
    let token = discovery["token"].as_str().unwrap().to_owned();
    let client = McpPost::authed(port, &token);

    // The tool stays in the vocabulary; only its answer names the absence.
    let tools = client.tools_list();
    assert!(tools.body.contains("\"command\""), "{}", tools.body);

    let reply = client.call("command", json!({ "command": "anything.at.all" }));
    assert_eq!(reply.status, 200, "{}", reply.body);
    let (is_error, content) = tool_content(&reply);
    assert!(is_error, "command must fail typed: {content}");
    assert_eq!(content["error"], "unsupported", "{content}");
    assert!(
        content["message"]
            .as_str()
            .unwrap()
            .contains("no command registry"),
        "{content}"
    );

    handle.shutdown().unwrap();
    drop(app);
}

/// A window gaining a child webview (native-content island) must stay
/// enumerable and targetable: `webview_windows()` loses it, the handler's
/// `Window` + same-label-webview walk must not (Figmatic, 2026-08-20).
#[test]
fn multiwebview_window_stays_enumerable() {
    let app = tauri::test::mock_app();
    let window = WebviewWindowBuilder::new(app.handle(), "main", Default::default())
        .build()
        .unwrap();

    let state_root = tempfile::tempdir().unwrap();
    let handle = mount_agent_control(
        app.handle(),
        AgentControlConfig::new("longhorn-multiwebview-fixture")
            .with_state_root(state_root.path().to_owned()),
        Arc::new(NoCommandBridge),
    )
    .unwrap();

    let discovery_path = wait_for_discovery(&state_root.path().join("agent-control"));
    let discovery: Value =
        serde_json::from_str(&std::fs::read_to_string(&discovery_path).unwrap()).unwrap();
    let port = discovery["port"].as_u64().unwrap() as u16;
    let token = discovery["token"].as_str().unwrap().to_owned();
    let client = McpPost::authed(port, &token);

    // Attach a child webview with a different label — the Figmatic shape.
    let child = tauri::webview::WebviewBuilder::new(
        "preview-island",
        tauri::WebviewUrl::App("index.html".into()),
    );
    window
        .as_ref()
        .window()
        .add_child(
            child,
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(100.0, 100.0),
        )
        .unwrap();

    // The window must still enumerate and target.
    let reply = client.call("list_windows", json!({}));
    assert_eq!(reply.status, 200, "{}", reply.body);
    let (is_error, content) = tool_content(&reply);
    assert!(!is_error, "list_windows must succeed: {content}");
    let labels: Vec<_> = content["windows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|info| info["window"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        labels.contains(&"main".to_owned()),
        "main lost after child webview attach: {labels:?}"
    );

    let resize = client.call(
        "resize_window",
        json!({ "window": "main", "width": 400.0, "height": 300.0 }),
    );
    let (resize_error, resize_content) = tool_content(&resize);
    assert!(!resize_error, "resize must target main: {resize_content}");

    handle.shutdown().unwrap();
    drop(app);
}
