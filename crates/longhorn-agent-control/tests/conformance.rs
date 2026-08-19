//! Stateless-server conformance fixtures (Card 229), all in-process:
//! router-level requests through `tower::oneshot` plus one real-loopback
//! serve proving the discovery lifecycle.
//!
//! Request shapes copy the Card 227 wire capture: revision 2026-07-28 with
//! the `Mcp-Method`/`Mcp-Name` headers and the `_meta.io.modelcontextprotocol`
//! envelope that revision requires.

use std::{
    collections::BTreeSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use http_body_util::BodyExt as _;
use longhorn_agent_control::{
    ActionReceipt, CommandResult, ControlHandler, ControlServerConfig, ElementRef, EvaluateResult,
    InstanceToken, ListWindowsResult, PageState, ScreenshotResult, SemanticNode, SnapshotResult,
    ToolError, WaitForResult, control_router, enumerate_discovery, serve_control_surface,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt as _;

const SESSION_HEADER: &str = "mcp-session-id";

/// Host stub: canned answers, an invocation counter that proves rejection
/// happened before dispatch, and an echo journal for the interleave fixture.
#[derive(Clone, Default)]
struct StubHandler {
    invocations: Arc<AtomicUsize>,
    evaluated: Arc<Mutex<Vec<String>>>,
}

impl StubHandler {
    fn invocation_count(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
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
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(SnapshotResult {
            window: longhorn_core::WindowId::new("main").unwrap(),
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
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {})
    }

    async fn r#type(
        &self,
        _request: longhorn_agent_control::TypeRequest,
    ) -> Result<ActionReceipt, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {})
    }

    async fn press(
        &self,
        _request: longhorn_agent_control::PressRequest,
    ) -> Result<ActionReceipt, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {})
    }

    async fn scroll(
        &self,
        _request: longhorn_agent_control::ScrollRequest,
    ) -> Result<ActionReceipt, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {})
    }

    async fn drag(
        &self,
        _request: longhorn_agent_control::DragRequest,
    ) -> Result<ActionReceipt, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {})
    }

    async fn evaluate(
        &self,
        request: longhorn_agent_control::EvaluateRequest,
    ) -> Result<EvaluateResult, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        self.evaluated.lock().unwrap().push(request.js.clone());
        Ok(EvaluateResult {
            value: Value::String(request.js),
        })
    }

    async fn wait_for(
        &self,
        _request: longhorn_agent_control::WaitForRequest,
    ) -> Result<WaitForResult, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(WaitForResult {})
    }

    async fn screenshot(
        &self,
        _request: longhorn_agent_control::ScreenshotRequest,
    ) -> Result<ScreenshotResult, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ScreenshotResult {
            window: longhorn_core::WindowId::new("main").unwrap(),
            png: vec![1, 2, 3],
        })
    }

    async fn command(
        &self,
        _request: longhorn_agent_control::CommandRequest,
    ) -> Result<CommandResult, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(CommandResult { output: None })
    }

    async fn list_windows(
        &self,
        _request: longhorn_agent_control::ListWindowsRequest,
    ) -> Result<ListWindowsResult, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ListWindowsResult {
            windows: Vec::new(),
        })
    }

    async fn resize_window(
        &self,
        _request: longhorn_agent_control::ResizeWindowRequest,
    ) -> Result<ActionReceipt, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {})
    }
}

/// Router plus the token and stub the fixtures assert against.
fn app() -> (Router, InstanceToken, StubHandler) {
    let token = InstanceToken::generate().unwrap();
    let stub = StubHandler::default();
    (control_router(stub.clone(), token.clone()), token, stub)
}

/// The `_meta` envelope revision 2026-07-28 requires (Card 227 capture).
fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "conformance", "version": "0.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

/// Builds one MCP POST with the revision-2026-07-28 header set.
struct McpRequest {
    token: Option<String>,
    origin: Option<String>,
    session_id: Option<String>,
}

impl McpRequest {
    fn authed(token: &InstanceToken) -> Self {
        Self {
            token: Some(token.as_str().to_owned()),
            origin: None,
            session_id: None,
        }
    }

    fn unauthenticated() -> Self {
        Self {
            token: None,
            origin: None,
            session_id: None,
        }
    }

    fn origin(mut self, origin: &str) -> Self {
        self.origin = Some(origin.to_owned());
        self
    }

    fn session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_owned());
        self
    }

    fn post(self, method: &str, name: Option<&str>, params: Value) -> Request<Body> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        })
        .to_string();
        let mut builder = Request::builder()
            .method("POST")
            .uri("http://localhost/mcp")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header("mcp-protocol-version", "2026-07-28")
            .header("mcp-method", method);
        if let Some(name) = name {
            builder = builder.header("mcp-name", name);
        }
        if let Some(token) = &self.token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(origin) = &self.origin {
            builder = builder.header(header::ORIGIN, origin);
        }
        if let Some(session_id) = &self.session_id {
            builder = builder.header(SESSION_HEADER, session_id);
        }
        builder.body(Body::from(body)).unwrap()
    }

    fn tools_list(self) -> Request<Body> {
        self.post("tools/list", None, json!({ "_meta": meta() }))
    }

    fn evaluate(self, js: &str) -> Request<Body> {
        self.post(
            "tools/call",
            Some("evaluate"),
            json!({ "name": "evaluate", "arguments": { "js": js }, "_meta": meta() }),
        )
    }
}

struct McpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl McpResponse {
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

async fn exchange(app: Router, request: Request<Body>) -> McpResponse {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    McpResponse {
        status,
        headers,
        body: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

#[tokio::test]
async fn no_session_id_is_ever_minted_or_echoed() {
    let (app, token, _stub) = app();

    for request in [
        McpRequest::authed(&token).tools_list(),
        McpRequest::authed(&token).evaluate("1 + 1"),
        // A client-supplied session id must not be echoed either.
        McpRequest::authed(&token)
            .session_id("bogus-session")
            .tools_list(),
    ] {
        let response = exchange(app.clone(), request).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        assert!(
            !response.headers.contains_key(SESSION_HEADER),
            "session id header leaked: {:?}",
            response.headers
        );
        assert!(!response.body.contains("bogus-session"));
    }
}

#[tokio::test]
async fn get_and_delete_answer_405() {
    let (app, token, _stub) = app();

    for method in ["GET", "DELETE"] {
        let request = Request::builder()
            .method(method)
            .uri("http://localhost/mcp")
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header(header::AUTHORIZATION, format!("Bearer {}", token.as_str()))
            .body(Body::empty())
            .unwrap();
        let response = exchange(app.clone(), request).await;
        assert_eq!(response.status, StatusCode::METHOD_NOT_ALLOWED, "{method}");
        assert_eq!(
            response.headers.get(header::ALLOW).unwrap(),
            "POST",
            "{method}"
        );
    }
}

#[tokio::test]
async fn missing_or_wrong_token_is_rejected_before_dispatch() {
    let (app, token, stub) = app();

    for request in [
        McpRequest::unauthenticated().tools_list(),
        McpRequest::unauthenticated().evaluate("1 + 1"),
        McpRequest {
            token: Some("wrong-token".to_owned()),
            origin: None,
            session_id: None,
        }
        .tools_list(),
    ] {
        let response = exchange(app.clone(), request).await;
        assert_eq!(
            response.status,
            StatusCode::UNAUTHORIZED,
            "{}",
            response.body
        );
        assert_eq!(
            response.headers.get(header::WWW_AUTHENTICATE).unwrap(),
            "Bearer realm=\"longhorn-agent-control\""
        );
    }
    assert_eq!(
        stub.invocation_count(),
        0,
        "a rejected request reached tool dispatch"
    );

    // Positive control: the valid token passes the same guard.
    let response = exchange(app.clone(), McpRequest::authed(&token).tools_list()).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.body);
}

#[tokio::test]
async fn browser_origins_are_rejected_before_dispatch() {
    let (app, token, stub) = app();

    for origin in [
        "https://evil.example",
        "null",
        "http://127.0.0.1.evil.example",
    ] {
        let response = exchange(
            app.clone(),
            McpRequest::authed(&token).origin(origin).tools_list(),
        )
        .await;
        assert_eq!(response.status, StatusCode::FORBIDDEN, "origin {origin}");
    }
    assert_eq!(
        stub.invocation_count(),
        0,
        "a rejected origin reached tool dispatch"
    );

    // Loopback origins and absent origins pass the guard.
    for origin in [
        None,
        Some("http://localhost:5173"),
        Some("http://127.0.0.1:8080"),
    ] {
        let request = match origin {
            Some(origin) => McpRequest::authed(&token).origin(origin).tools_list(),
            None => McpRequest::authed(&token).tools_list(),
        };
        let response = exchange(app.clone(), request).await;
        assert_eq!(response.status, StatusCode::OK, "origin {origin:?}");
    }
    assert_eq!(stub.invocation_count(), 0, "tools/list runs no tool");
}

#[tokio::test]
async fn two_clients_interleave_without_cross_talk() {
    let (app, token, stub) = app();

    let mut set = tokio::task::JoinSet::new();
    for client in ["a", "b"] {
        for index in 0..8 {
            let app = app.clone();
            let token = token.clone();
            let js = format!("{client}-{index}");
            set.spawn(async move {
                let response = exchange(app, McpRequest::authed(&token).evaluate(&js)).await;
                assert_eq!(response.status, StatusCode::OK, "{}", response.body);
                (js, response.json())
            });
        }
    }

    let mut answered = BTreeSet::new();
    while let Some(outcome) = set.join_next().await {
        let (sent, body) = outcome.unwrap();
        let result = &body["result"];
        assert_eq!(result["isError"], json!(false), "{body}");
        let content: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        // The echo that came back is exactly the request this client sent.
        assert_eq!(content["value"], json!(sent));
        answered.insert(sent);
    }
    assert_eq!(answered.len(), 16);

    let mut journal = stub.evaluated.lock().unwrap().clone();
    journal.sort();
    assert_eq!(journal, answered.into_iter().collect::<Vec<_>>());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_lifetime_tracks_the_server() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    let config = ControlServerConfig {
        app_id: "dev.example.conformance".to_owned(),
        discovery_dir: dir.clone(),
        port: 0,
    };

    let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_control_surface(
        config,
        StubHandler::default(),
        async {
            let _ = shutdown_rx.await;
        },
    ));

    // The file appears on serve, carrying the real bound port and the token
    // an agent would read.
    let record = {
        let mut found = None;
        for _ in 0..200 {
            let scan = enumerate_discovery(&dir).unwrap();
            if let Some(record) = scan.instances.into_iter().next() {
                found = Some(record);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        found.expect("discovery file appears while serving")
    };
    assert!(record.is_live());
    assert_eq!(record.file().pid, std::process::id());
    let port = record.file().port;
    assert_ne!(port, 0, "the ephemeral port must be resolved");
    let token = record.file().token.clone();

    // One real-loopback request: the wired stack answers, still stateless.
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": { "_meta": meta() },
    })
    .to_string();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nAuthorization: Bearer {}\r\nMcp-Protocol-Version: 2026-07-28\r\nMcp-Method: tools/list\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        token.as_str(),
        body.len(),
    );
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();
    let mut raw = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut raw)
        .await
        .unwrap();
    let raw = String::from_utf8(raw).unwrap();
    assert!(raw.starts_with("HTTP/1.1 200"), "{raw}");
    assert!(raw.contains("\"evaluate\""), "{raw}");
    assert!(!raw.to_ascii_lowercase().contains(SESSION_HEADER), "{raw}");

    // Shutdown removes the file.
    shutdown.send(()).unwrap();
    let receipt = server.await.unwrap().unwrap();
    assert_eq!(receipt.bound.port(), port);
    assert!(enumerate_discovery(&dir).unwrap().instances.is_empty());
}
