//! Opt-in stdio carrier for the contract 022 server (g02.044).
//!
//! A harness that cannot speak streamable HTTP spawns
//! `longhorn-agent-control-client`; the carrier fronts the one live
//! instance it finds through discovery and proxies newline-delimited
//! JSON-RPC on stdio to one POST per message on the instance's `/mcp`
//! endpoint. It holds no catalogue, registers nothing, mints nothing,
//! and binds no listener: every byte of MCP meaning comes from the
//! server's own responses.
//!
//! Position on `listen` and resources: adapted by pass-through, not
//! reimplemented. `resources/list`, `resources/read`, `resources/subscribe`,
//! and `subscriptions/listen` travel the same POST path as every other
//! method; a listen request's long-lived SSE stream stays mapped onto its
//! pending stdio request, and server-to-client notifications arriving as
//! SSE data are written to stdout like any other message. Cancelling the
//! stdio request (or closing stdin) aborts the HTTP stream, which is the
//! contract 022 cancellation.

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use longhorn_config::{PlatformDirectoryFact, PlatformDirectoryFacts, TargetPlatform};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    task::JoinHandle,
};

use crate::{DiscoveryError, DiscoveryFile, enumerate_discovery, resolve_discovery_dir};

/// Protocol version from which the server requires the SEP-2243 header set
/// (`Mcp-Method`, `Mcp-Name`, `Mcp-Param-*` plus `MCP-Protocol-Version`).
/// Matches rmcp's `ProtocolVersion::STANDARD_HEADERS`.
const STANDARD_HEADERS_VERSION: &str = "2026-07-28";

/// Sentinel wrapping a Base64-encoded header value. Matches rmcp's
/// `BASE64_HEADER_PREFIX`/`BASE64_HEADER_SUFFIX`.
const BASE64_PREFIX: &str = "=?base64?";
const BASE64_SUFFIX: &str = "?=";

/// Upper bound on one buffered HTTP response body or SSE event batch.
/// Screenshots ride as Base64 PNGs; the bound is generous but finite so a
/// misbehaving server cannot grow the carrier without limit.
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// Carrier startup or runtime failure.
#[derive(Debug)]
pub enum CarrierError {
    /// Discovery enumeration, resolution, or fact construction failed.
    Discovery(String),
    /// No live instance matches the selection.
    NoLiveInstance {
        /// Directory that was enumerated.
        dir: PathBuf,
        /// App-id filter that was applied, if any.
        app_id: Option<String>,
    },
    /// More than one live instance matches and the carrier refuses to guess.
    AmbiguousInstances {
        /// Directory that was enumerated.
        dir: PathBuf,
        /// `app-id (pid)` labels of the matching instances.
        candidates: Vec<String>,
    },
    /// The selected record's port or token cannot form an endpoint.
    InvalidRecord(String),
    /// An environment fact the directory resolution needs is missing.
    MissingEnvironment {
        /// Variable or fact that was missing.
        what: String,
    },
    /// A proxied exchange failed and the message could not be answered.
    Exchange(String),
    /// Stdout is broken; the harness is gone.
    OutputClosed(String),
}

impl fmt::Display for CarrierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(reason) => write!(formatter, "discovery failed: {reason}"),
            Self::NoLiveInstance { dir, app_id } => match app_id {
                Some(app_id) => write!(
                    formatter,
                    "no live agent-control instance for app {app_id:?} in {}",
                    dir.display()
                ),
                None => write!(
                    formatter,
                    "no live agent-control instance in {}",
                    dir.display()
                ),
            },
            Self::AmbiguousInstances { dir, candidates } => {
                write!(
                    formatter,
                    "more than one live agent-control instance in {} ({}); pass --app-id",
                    dir.display(),
                    candidates.join(", ")
                )
            }
            Self::InvalidRecord(reason) => write!(formatter, "invalid discovery record: {reason}"),
            Self::MissingEnvironment { what } => {
                write!(
                    formatter,
                    "cannot resolve platform directories: {what} is missing"
                )
            }
            Self::Exchange(reason) => write!(formatter, "exchange failed: {reason}"),
            Self::OutputClosed(reason) => write!(formatter, "stdout closed: {reason}"),
        }
    }
}

impl std::error::Error for CarrierError {}

impl From<DiscoveryError> for CarrierError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error.to_string())
    }
}

/// Builds platform directory facts from the process environment, using the
/// same per-platform base-directory rules the Tauri host injects
/// (`longhorn-tauri-config::platform_directory_facts`): XDG bases with
/// home-relative defaults on Linux, `Library` bases under `HOME` on macOS,
/// `LOCALAPPDATA`/`APPDATA` on Windows. Only the state root feeds
/// discovery resolution; the rest keep the resolver's required-fact set
/// complete.
pub fn facts_from_env() -> Result<PlatformDirectoryFacts, CarrierError> {
    #[cfg(target_os = "macos")]
    {
        let home = env_dir("HOME")?;
        let local_data = home.join("Library").join("Application Support");
        Ok(PlatformDirectoryFacts::complete(
            TargetPlatform::MacOs,
            local_data.clone(),
            local_data.clone(),
            local_data.clone(),
            home.join("Library").join("Caches"),
            home.join("Library").join("Logs"),
            std::env::temp_dir(),
        )
        .with(PlatformDirectoryFact::SharedData, local_data))
    }
    #[cfg(target_os = "windows")]
    {
        let local_data = env_dir("LOCALAPPDATA")?;
        let roaming_data =
            std::env::var_os("APPDATA").map_or_else(|| local_data.clone(), PathBuf::from);
        let temp = std::env::var_os("TEMP").map_or_else(std::env::temp_dir, PathBuf::from);
        Ok(PlatformDirectoryFacts::complete(
            TargetPlatform::Windows,
            local_data.clone(),
            local_data.clone(),
            local_data.clone(),
            local_data.clone(),
            local_data.clone(),
            temp,
        )
        .with(PlatformDirectoryFact::SharedData, roaming_data))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let home = env_dir("HOME")?;
        let config =
            std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| home.join(".config"), PathBuf::from);
        let data = std::env::var_os("XDG_DATA_HOME")
            .map_or_else(|| home.join(".local").join("share"), PathBuf::from);
        // Same rule the Tauri host honors: `XDG_STATE_HOME`, then the
        // spec default below home. The carrier must land in the same
        // directory the app published to.
        let state = std::env::var_os("XDG_STATE_HOME")
            .map_or_else(|| home.join(".local").join("state"), PathBuf::from);
        let cache =
            std::env::var_os("XDG_CACHE_HOME").map_or_else(|| home.join(".cache"), PathBuf::from);
        let runtime =
            std::env::var_os("XDG_RUNTIME_DIR").map_or_else(|| cache.clone(), PathBuf::from);
        Ok(PlatformDirectoryFacts::complete(
            TargetPlatform::Linux,
            config,
            data.clone(),
            state.clone(),
            cache,
            state,
            runtime,
        )
        .with(PlatformDirectoryFact::SharedData, data))
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        Err(CarrierError::MissingEnvironment {
            what: "supported desktop platform".to_owned(),
        })
    }
}

fn env_dir(name: &str) -> Result<PathBuf, CarrierError> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| CarrierError::MissingEnvironment {
            what: name.to_owned(),
        })
}

/// Resolves the discovery directory the carrier enumerates: explicit
/// `--discovery-dir` wins, then `--state-root` as an explicit
/// contract-004 override, then the platform-native directory.
pub fn resolve_carrier_dir(
    discovery_dir: Option<PathBuf>,
    state_root: Option<PathBuf>,
) -> Result<PathBuf, CarrierError> {
    if let Some(dir) = discovery_dir {
        return Ok(dir);
    }
    let facts = facts_from_env()?;
    if let Some(state_root) = state_root {
        return crate::resolve_discovery_dir_with_state_override(&facts, &state_root)
            .map_err(CarrierError::from);
    }
    resolve_discovery_dir(&facts).map_err(CarrierError::from)
}

/// The one discovered instance the carrier fronts.
#[derive(Clone, Debug)]
pub struct CarrierSelection {
    /// The discovery record: app id, pid, port, and bearer.
    pub file: DiscoveryFile,
    /// `http://127.0.0.1:<port>/mcp`.
    pub endpoint: String,
}

/// Selects the single live instance in `dir`, optionally filtered by app
/// id. Zero matches and multiple matches are both errors: the carrier
/// never invents an instance and never guesses between two.
pub fn select_instance(dir: &Path, app_id: Option<&str>) -> Result<CarrierSelection, CarrierError> {
    let scan = enumerate_discovery(dir)?;
    let mut live: Vec<&DiscoveryFile> = scan
        .instances
        .iter()
        .filter(|record| record.is_live())
        .map(crate::DiscoveryRecord::file)
        .collect();
    if let Some(app_id) = app_id {
        live.retain(|file| file.app_id == app_id);
    }
    // Deterministic order so the ambiguity error is stable.
    live.sort_by(|left, right| (&left.app_id, left.pid).cmp(&(&right.app_id, right.pid)));
    match live.len() {
        1 => {
            let file = live[0].clone();
            if file.port == 0 {
                return Err(CarrierError::InvalidRecord(format!(
                    "instance {} ({}) published port 0",
                    file.app_id, file.pid
                )));
            }
            Ok(CarrierSelection {
                endpoint: format!("http://127.0.0.1:{}/mcp", file.port),
                file,
            })
        }
        0 => Err(CarrierError::NoLiveInstance {
            dir: dir.to_path_buf(),
            app_id: app_id.map(str::to_owned),
        }),
        _ => Err(CarrierError::AmbiguousInstances {
            dir: dir.to_path_buf(),
            candidates: live
                .iter()
                .map(|file| format!("{} ({})", file.app_id, file.pid))
                .collect(),
        }),
    }
}

/// Session state negotiated with the server: the protocol version from the
/// `initialize` result and the tool input schemas cached from `tools/list`
/// results. Both feed the SEP-2243 header synthesis, mirroring rmcp's own
/// streamable-HTTP client.
#[derive(Clone, Debug, Default)]
struct SessionState {
    negotiated: Option<String>,
    tool_schemas: HashMap<String, serde_json::Map<String, Value>>,
}

impl SessionState {
    fn standard_headers(&self) -> bool {
        self.negotiated
            .as_deref()
            .is_some_and(|version| version >= STANDARD_HEADERS_VERSION)
    }
}

/// `_meta` keys the server requires on the strict (2026-07-28+) path.
const META_PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const META_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";

/// Prepares one client-to-server message for POST: assures the strict-path
/// `_meta` envelope, then synthesizes the SEP-2243 header set from the
/// body, exactly as rmcp's streamable-HTTP client does.
///
/// `_meta` assurance is purely additive and per-request: a harness that
/// already declares its own `_meta` keeps it (its version also selects
/// the `MCP-Protocol-Version` header, which must match the body), while a
/// plain stdio harness — which knows nothing of the HTTP envelope — gets
/// the negotiated version and empty client capabilities merged into its
/// params. Without this, every post-`initialize` request from a plain
/// harness would fail the server's `_meta` requirement; with it, the
/// carrier takes the same strict code path an rmcp HTTP client takes.
/// Below the negotiated standard version (or before negotiation) the body
/// and headers stay legacy-bare and the server skips both validations, so
/// older harnesses pass through untouched.
fn prepare_outgoing(message: &mut Value, session: &SessionState) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    let Some(method) = message
        .get("method")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return headers;
    };
    let effective = assure_meta(message, session);
    let params = message.get("params");
    headers.push(("Mcp-Method".to_owned(), method.clone()));
    if let Some(name) = extract_name(&method, params) {
        headers.push(("Mcp-Name".to_owned(), encode_header_value(&name)));
    }
    if let Some(effective) = effective {
        headers.push(("MCP-Protocol-Version".to_owned(), effective));
        if method == "tools/call"
            && session.standard_headers()
            && let Some(params) = message.get("params")
        {
            promote_param_headers(params, session, &mut headers);
        }
    }
    headers
}

/// Merges the strict-path `_meta` envelope into a request body's params
/// without touching anything the harness already declared. Returns the
/// effective protocol version for the header: the harness's own `_meta`
/// version when present, else the negotiated one, else none.
fn assure_meta(message: &mut Value, session: &SessionState) -> Option<String> {
    let params = message.get_mut("params")?;
    let params = params.as_object_mut()?;
    let declared = params
        .get("_meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get(META_PROTOCOL_VERSION))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let effective = declared.or_else(|| session.negotiated.clone())?;
    if effective.as_str() >= STANDARD_HEADERS_VERSION {
        let meta = params
            .entry("_meta".to_owned())
            .or_insert_with(|| Value::Object(Default::default()));
        if let Some(meta) = meta.as_object_mut() {
            meta.entry(META_PROTOCOL_VERSION.to_owned())
                .or_insert_with(|| Value::String(effective.clone()));
            meta.entry(META_CLIENT_CAPABILITIES.to_owned())
                .or_insert_with(|| Value::Object(Default::default()));
        }
    }
    Some(effective)
}

/// Returns the `Mcp-Name` value for methods that carry one.
fn extract_name(method: &str, params: Option<&Value>) -> Option<String> {
    let params = params?;
    let key = if matches!(method, "tools/call" | "prompts/get") {
        "name"
    } else if matches!(
        method,
        "resources/read" | "resources/subscribe" | "resources/unsubscribe"
    ) {
        "uri"
    } else if matches!(method, "tasks/get" | "tasks/update" | "tasks/cancel") {
        "taskId"
    } else {
        return None;
    };
    params.get(key)?.as_str().map(str::to_owned)
}

/// Wraps a value as `=?base64?<b64>?=` when it cannot travel as a bare
/// header value. Matches rmcp's emission rule.
fn encode_header_value(value: &str) -> String {
    if requires_base64(value) {
        format!("{BASE64_PREFIX}{}{BASE64_SUFFIX}", BASE64.encode(value))
    } else {
        value.to_owned()
    }
}

fn requires_base64(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let bytes = value.as_bytes();
    if matches!(bytes.first(), Some(b' ' | b'\t')) || matches!(bytes.last(), Some(b' ' | b'\t')) {
        return true;
    }
    if value
        .chars()
        .any(|c| (c as u32) < 0x20 || (c as u32) > 0x7E)
    {
        return true;
    }
    value.starts_with(BASE64_PREFIX) && value.ends_with(BASE64_SUFFIX)
}

/// Promotes annotated primitive `tools/call` arguments to `Mcp-Param-*`
/// headers using the schema cached from the `tools/list` result.
fn promote_param_headers(
    params: &Value,
    session: &SessionState,
    headers: &mut Vec<(String, String)>,
) {
    let Some(tool) = params
        .get("name")
        .and_then(Value::as_str)
        .and_then(|name| session.tool_schemas.get(name))
    else {
        return;
    };
    let Some(arguments) = params.get("arguments") else {
        return;
    };
    let Some(properties) = tool.get("properties").and_then(Value::as_object) else {
        return;
    };
    // Sorted for a stable header order.
    let mut annotated: BTreeMap<&str, &str> = BTreeMap::new();
    for (property, schema) in properties {
        if let Some(header) = schema.get("x-mcp-header").and_then(Value::as_str) {
            annotated.insert(property.as_str(), header);
        }
    }
    for (property, header) in annotated {
        let primitive = match arguments.get(property) {
            Some(Value::String(value)) => Some(value.clone()),
            Some(Value::Bool(value)) => Some(value.to_string()),
            Some(Value::Number(value)) => Some(value.to_string()),
            _ => None,
        };
        if let Some(value) = primitive {
            headers.push((format!("Mcp-Param-{header}"), encode_header_value(&value)));
        }
    }
}

/// Records an `initialize` result's negotiated version and caches `tools/list`
/// result schemas for `Mcp-Param-*` promotion.
fn observe_server_message(
    message: &Value,
    request_method: Option<&str>,
    session: &mut SessionState,
) {
    let result = match message.get("result") {
        Some(result) => result,
        None => return,
    };
    match request_method {
        Some("initialize") => {
            if let Some(version) = result.get("protocolVersion").and_then(Value::as_str) {
                session.negotiated = Some(version.to_owned());
            }
        }
        Some("tools/list") => {
            if !session.standard_headers() {
                return;
            }
            let tools = result
                .get("tools")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for tool in tools {
                if let (Some(name), Some(schema)) = (
                    tool.get("name").and_then(Value::as_str),
                    tool.get("inputSchema").and_then(Value::as_object),
                ) {
                    session.tool_schemas.insert(name.to_owned(), schema.clone());
                }
            }
        }
        _ => {}
    }
}

/// Incremental SSE parser: feeds response chunks, emits complete `data:`
/// payloads. Matches the `text/event-stream` framing rmcp's server emits.
#[derive(Debug, Default)]
struct SseParser {
    buffer: Vec<u8>,
    pending: Vec<u8>,
}

impl SseParser {
    /// Pushes one chunk; returns complete event payloads and whether the
    /// stream preview already exceeds the body bound.
    fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, CarrierError> {
        self.buffer.extend_from_slice(chunk);
        if self.buffer.len() + self.pending.len() > MAX_BODY_BYTES {
            return Err(CarrierError::Exchange(format!(
                "response exceeds {MAX_BODY_BYTES} bytes"
            )));
        }
        let mut payloads = Vec::new();
        while let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.is_empty() {
                if !self.pending.is_empty() {
                    // Trailing newline of a multi-line data block is framing.
                    self.pending.pop();
                    payloads.push(std::mem::take(&mut self.pending));
                }
                continue;
            }
            if line.first() == Some(&b':') {
                continue;
            }
            if let Some(data) = line
                .strip_prefix(b"data:")
                .map(|rest| rest.strip_prefix(b" ").unwrap_or(rest))
            {
                self.pending.extend_from_slice(data);
                self.pending.push(b'\n');
            }
        }
        Ok(payloads)
    }

    /// Flushes a final under-terminated payload, if any.
    fn finish(&mut self) -> Option<Vec<u8>> {
        if self.pending.is_empty() {
            return None;
        }
        self.pending.pop();
        Some(std::mem::take(&mut self.pending))
    }
}

/// How the carrier's run loop ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CarrierExit {
    /// Stdin reached EOF: the harness is done. In-flight exchanges abort.
    HarnessClosed,
}

/// Runs the stdio↔HTTP proxy until stdin closes or stdout breaks.
/// `stdin`/`stdout` are injected so the end-to-end test can drive the
/// carrier over pipes exactly as a harness would.
pub async fn run_carrier(
    selection: CarrierSelection,
    stdin: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    stdout: impl tokio::io::AsyncWrite + Unpin + Send + 'static,
) -> Result<CarrierExit, CarrierError> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|error| CarrierError::Exchange(format!("HTTP client failed: {error}")))?;
    let shared = Arc::new(CarrierShared {
        client,
        endpoint: selection.endpoint.clone(),
        token: selection.file.token.as_str().to_owned(),
        session: Mutex::new(SessionState::default()),
        output: tokio::sync::Mutex::new(stdout),
        inflight: tokio::sync::Mutex::new(HashMap::new()),
        output_broken: std::sync::atomic::AtomicBool::new(false),
    });

    let mut lines = BufReader::new(stdin).lines();
    loop {
        let line = lines
            .next_line()
            .await
            .map_err(|error| CarrierError::Exchange(format!("stdin failed: {error}")))?;
        let Some(line) = line else {
            shared.abort_all().await;
            return Ok(CarrierExit::HarnessClosed);
        };
        let line = line.strip_suffix('\r').unwrap_or(&line);
        if line.is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            // Unparsable input has no id to correlate a reply to; ignore it
            // like the rmcp stdio codec does.
            continue;
        };
        shared.dispatch(line.to_owned(), message).await;
        if shared
            .output_broken
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            shared.abort_all().await;
            return Err(CarrierError::OutputClosed(
                "harness stdout is gone".to_owned(),
            ));
        }
    }
}

/// One live exchange plus its dedup generation: a duplicate JSON-RPC id
/// replaces the registered entry, and the finishing task removes itself
/// only while its generation is still current.
type Inflight = HashMap<String, (JoinHandle<()>, Arc<()>)>;

struct CarrierShared<W> {
    client: reqwest::Client,
    endpoint: String,
    token: String,
    session: Mutex<SessionState>,
    output: tokio::sync::Mutex<W>,
    inflight: tokio::sync::Mutex<Inflight>,
    output_broken: std::sync::atomic::AtomicBool,
}

impl<W> CarrierShared<W>
where
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    async fn abort_all(self: &Arc<Self>) {
        let mut inflight = self.inflight.lock().await;
        for (_, (handle, _)) in inflight.drain() {
            handle.abort();
        }
    }

    /// Removes one in-flight exchange, but only when `generation` is still
    /// the registered one: a duplicate id may have replaced it while the
    /// old task was finishing.
    async fn remove_inflight(&self, key: &str, generation: &Arc<()>) {
        let mut inflight = self.inflight.lock().await;
        let stale = inflight
            .get(key)
            .is_some_and(|(_, current)| Arc::ptr_eq(current, generation));
        if stale {
            inflight.remove(key);
        }
    }

    /// Routes one parsed stdio message: cancellation aborts the in-flight
    /// exchange it names, everything else becomes one HTTP POST task.
    async fn dispatch(self: &Arc<Self>, raw: String, mut message: Value) {
        if is_cancellation(&message) {
            if let Some(target) = message
                .get("params")
                .and_then(|params| params.get("requestId"))
            {
                let key = canonical_id(target);
                if let Some((handle, _)) = self.inflight.lock().await.remove(&key) {
                    handle.abort();
                }
            }
            // Stateless HTTP holds no server-side subscription for the
            // cancelled request, so there is nothing to forward: aborting
            // the POST stream is the cancellation.
            return;
        }
        let id_key = message
            .get("id")
            .filter(|id| !id.is_null())
            .map(canonical_id);
        // The POST body is the stdio bytes plus the `_meta` envelope when
        // the strict path needs it; the harness's own bytes are never
        // rewritten, only additively completed.
        let headers = prepare_outgoing(&mut message, &self.session.lock().unwrap());
        let raw = serde_json::to_string(&message).unwrap_or(raw);
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let this = Arc::clone(self);
        let generation = Arc::new(());
        let finishing = Arc::clone(&generation);
        let completion_key = id_key.clone();
        let task = tokio::spawn(async move {
            this.exchange(raw, completion_key.clone(), headers, method)
                .await;
            if let Some(completion_key) = completion_key {
                this.remove_inflight(&completion_key, &finishing).await;
            }
        });
        if let Some(key) = id_key {
            // A duplicate id replaces the earlier exchange; JSON-RPC ids
            // correlate replies, so only the latest task may answer.
            if let Some((previous, _)) = self.inflight.lock().await.insert(key, (task, generation))
            {
                previous.abort();
            }
        } else {
            // Notifications are fire-and-forget; the task owns itself.
            drop(task);
        }
    }

    /// One POST exchange: forwards the stdio bytes verbatim, streams the
    /// reply back to stdout. Semantics stay the server's: the carrier never
    /// interprets result or error payloads.
    async fn exchange(
        &self,
        raw: String,
        id: Option<String>,
        headers: Vec<(String, String)>,
        method: Option<String>,
    ) {
        let is_notification = id.is_none();
        let mut request = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("Authorization", format!("Bearer {}", self.token))
            .body(raw);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                // The server may start after the harness spawned the
                // carrier; answer the pending request with a typed error
                // and stay alive rather than exiting.
                self.answer_request(&id, &transport_error(&error)).await;
                return;
            }
        };
        let status = response.status();
        if status.as_u16() == 202 {
            // Notification accepted; nothing to write back. A 202 for a
            // request would leave the harness hanging with no reply, so
            // answer it as a transport error instead.
            if !is_notification {
                self.answer_request(
                    &id,
                    &serde_json::json!({
                        "code": -32000,
                        "message": "carrier: control server accepted the request without a reply",
                    }),
                )
                .await;
            }
            return;
        }
        if !status.is_success() {
            let body = response.bytes().await.unwrap_or_default();
            match message_from_body(&body) {
                Some(messages) => {
                    for message in messages {
                        self.observe(&message, method.as_deref());
                        self.write_line(&message).await;
                    }
                }
                None => {
                    self.answer_request(&id, &http_status_error(status.as_u16(), &body))
                        .await;
                }
            }
            return;
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        if content_type.contains("text/event-stream") {
            self.forward_stream(response, method.as_deref(), is_notification)
                .await;
        } else {
            let body = response.bytes().await.unwrap_or_default();
            match message_from_body(&body) {
                Some(messages) => {
                    for message in messages {
                        self.observe(&message, method.as_deref());
                        if !is_notification {
                            self.write_line(&message).await;
                        }
                    }
                }
                None => {
                    self.answer_request(&id, &http_status_error(status.as_u16(), &body))
                        .await;
                }
            }
        }
    }

    /// Streams an SSE reply: each `data:` payload is one JSON-RPC message
    /// written to stdout in arrival order. For `subscriptions/listen` this
    /// keeps server-to-client notifications flowing on the pending stdio
    /// request until the stream closes.
    async fn forward_stream(
        &self,
        mut response: reqwest::Response,
        method: Option<&str>,
        is_notification: bool,
    ) {
        let mut parser = SseParser::default();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => match parser.push(&chunk) {
                    Ok(payloads) => {
                        for payload in payloads {
                            self.forward_payload(&payload, method, is_notification)
                                .await;
                        }
                    }
                    Err(error) => {
                        eprintln!("longhorn-agent-control-client: {error}");
                        return;
                    }
                },
                Ok(None) => break,
                Err(error) => {
                    eprintln!("longhorn-agent-control-client: stream failed: {error}");
                    return;
                }
            }
        }
        if let Some(payload) = parser.finish() {
            self.forward_payload(&payload, method, is_notification)
                .await;
        }
    }

    async fn forward_payload(&self, payload: &[u8], method: Option<&str>, is_notification: bool) {
        if payload.is_empty() {
            return;
        }
        let Ok(message) = serde_json::from_slice::<Value>(payload) else {
            return;
        };
        if !message.is_object() {
            return;
        }
        self.observe(&message, method);
        if !is_notification {
            self.write_line(&message).await;
        }
    }

    fn observe(&self, message: &Value, method: Option<&str>) {
        observe_server_message(message, method, &mut self.session.lock().unwrap());
    }

    /// Answers a pending request: nothing for notifications, one line for
    /// requests. Transport failures become JSON-RPC errors carrying the
    /// original id so the harness can correlate them.
    async fn answer_request(&self, id: &Option<String>, error: &Value) {
        let Some(id) = id else { return };
        let Ok(id) = serde_json::from_str::<Value>(id) else {
            return;
        };
        self.write_line(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": error,
        }))
        .await;
    }

    async fn write_line(&self, message: &Value) {
        let mut text = serde_json::to_string(message).unwrap_or_else(|_| "{}".to_owned());
        text.push('\n');
        let mut output = self.output.lock().await;
        if output.write_all(text.as_bytes()).await.is_err() || output.flush().await.is_err() {
            // Flagged for the run loop, which aborts all exchanges and
            // exits: there is no harness left to answer.
            self.output_broken
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

fn is_cancellation(message: &Value) -> bool {
    message.get("method").and_then(Value::as_str) == Some("notifications/cancelled")
        && message.get("id").is_none_or(Value::is_null)
}

/// Stable map key for a JSON-RPC id of any shape.
fn canonical_id(id: &Value) -> String {
    serde_json::to_string(id).unwrap_or_default()
}

fn transport_error(error: &reqwest::Error) -> Value {
    serde_json::json!({
        "code": -32000,
        "message": format!("carrier: control server unreachable: {error}"),
    })
}

fn http_status_error(status: u16, body: &[u8]) -> Value {
    let detail = String::from_utf8_lossy(body);
    let detail = detail.trim();
    let message = if detail.is_empty() {
        format!("carrier: control server answered HTTP {status}")
    } else {
        format!("carrier: control server answered HTTP {status}: {detail}")
    };
    let code = match status {
        401 | 403 => -32001,
        404 => -32002,
        _ => -32000,
    };
    serde_json::json!({ "code": code, "message": message })
}

/// Splits a JSON body into reply messages: one object, or one line per
/// element of a batch array. Anything else is not a JSON-RPC reply and
/// the caller answers with a synthesized transport error instead of
/// writing garbage to the harness's stdout.
fn message_from_body(body: &[u8]) -> Option<Vec<Value>> {
    let message: Value = serde_json::from_slice(body).ok()?;
    match message {
        Value::Array(messages) if messages.iter().all(Value::is_object) => Some(messages),
        Value::Array(_) => None,
        message if message.is_object() => Some(vec![message]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn session_at(version: &str) -> SessionState {
        SessionState {
            negotiated: Some(version.to_owned()),
            tool_schemas: HashMap::new(),
        }
    }

    fn prepare(message: Value, session: &SessionState) -> (Value, Vec<(String, String)>) {
        let mut message = message;
        let headers = prepare_outgoing(&mut message, session);
        (message, headers)
    }

    #[test]
    fn unnegotiated_messages_carry_method_but_no_version() {
        let (_, headers) = prepare(
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
            &SessionState::default(),
        );
        assert!(headers.contains(&("Mcp-Method".to_owned(), "tools/list".to_owned())));
        assert!(
            !headers
                .iter()
                .any(|(name, _)| name == "MCP-Protocol-Version")
        );
    }

    #[test]
    fn negotiated_modern_version_adds_version_and_name() {
        let (_, headers) = prepare(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {"name": "snapshot", "arguments": {}}
            }),
            &session_at("2026-07-28"),
        );
        assert!(headers.contains(&("Mcp-Method".to_owned(), "tools/call".to_owned())));
        assert!(headers.contains(&("Mcp-Name".to_owned(), "snapshot".to_owned())));
        assert!(headers.contains(&("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned())));
    }

    #[test]
    fn older_negotiated_version_stays_on_the_legacy_path() {
        let (message, headers) = prepare(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {"uri": "longhorn://agent-control/console"}
            }),
            &session_at("2025-06-18"),
        );
        assert!(headers.contains(&("Mcp-Method".to_owned(), "resources/read".to_owned())));
        assert!(headers.contains(&(
            "Mcp-Name".to_owned(),
            "longhorn://agent-control/console".to_owned()
        )));
        // The negotiated version still rides the header (as rmcp's own
        // client sends it), but below the standard line the server skips
        // header validation — and the body gains no strict envelope.
        assert!(headers.contains(&("MCP-Protocol-Version".to_owned(), "2025-06-18".to_owned())));
        assert!(message["params"].get("_meta").is_none());
    }

    #[test]
    fn resource_uris_source_the_name_header() {
        let (_, headers) = prepare(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {"uri": "longhorn://agent-control/console"}
            }),
            &session_at("2026-07-28"),
        );
        assert!(headers.contains(&(
            "Mcp-Name".to_owned(),
            "longhorn://agent-control/console".to_owned()
        )));
    }

    #[test]
    fn plain_harness_params_gain_the_meta_envelope() {
        let (message, _) = prepare(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            }),
            &session_at("2026-07-28"),
        );
        let meta = &message["params"]["_meta"];
        assert_eq!(
            meta["io.modelcontextprotocol/protocolVersion"],
            json!("2026-07-28")
        );
        assert_eq!(
            meta["io.modelcontextprotocol/clientCapabilities"],
            json!({})
        );
    }

    #[test]
    fn harness_declared_meta_is_never_rewritten() {
        let (message, headers) = prepare(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {"_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {"roots": {}},
                    "io.modelcontextprotocol/clientInfo": {"name": "harness", "version": "1"},
                }}
            }),
            &session_at("2026-07-28"),
        );
        let meta = &message["params"]["_meta"];
        assert_eq!(
            meta["io.modelcontextprotocol/clientCapabilities"],
            json!({"roots": {}})
        );
        assert_eq!(
            meta["io.modelcontextprotocol/clientInfo"]["name"],
            json!("harness")
        );
        assert!(headers.contains(&("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned())));
    }

    #[test]
    fn harness_declared_older_version_selects_the_legacy_path() {
        let (message, headers) = prepare(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {"_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2025-06-18",
                }}
            }),
            &session_at("2026-07-28"),
        );
        // The harness's version wins the header (it must match the body)
        // and no strict envelope is merged over a legacy declaration.
        assert!(headers.contains(&("MCP-Protocol-Version".to_owned(), "2025-06-18".to_owned())));
        assert!(
            message["params"]["_meta"]
                .get("io.modelcontextprotocol/clientCapabilities")
                .is_none()
        );
    }

    #[test]
    fn non_ascii_names_are_base64_wrapped() {
        assert_eq!(
            encode_header_value("snap–shot"),
            format!(
                "{BASE64_PREFIX}{}{BASE64_SUFFIX}",
                BASE64.encode("snap–shot")
            )
        );
        assert_eq!(encode_header_value("snapshot"), "snapshot");
    }

    #[test]
    fn annotated_tool_arguments_promote_to_param_headers() {
        let mut session = session_at("2026-07-28");
        session.tool_schemas.insert(
            "press".to_owned(),
            json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string", "x-mcp-header": "key"},
                    "nested": {"type": "object", "x-mcp-header": "nested"},
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let (_, headers) = prepare(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {"name": "press", "arguments": {"key": "Enter", "nested": {"a": 1}}}
            }),
            &session,
        );
        assert!(headers.contains(&("Mcp-Param-key".to_owned(), "Enter".to_owned())));
        assert!(!headers.iter().any(|(name, _)| name == "Mcp-Param-nested"));
    }

    #[test]
    fn initialize_result_negotiates_the_version() {
        let mut session = SessionState::default();
        observe_server_message(
            &json!({"jsonrpc": "2.0", "id": 1, "result": {"protocolVersion": "2026-07-28"}}),
            Some("initialize"),
            &mut session,
        );
        assert!(session.standard_headers());
    }

    #[test]
    fn tools_list_result_caches_input_schemas() {
        let mut session = session_at("2026-07-28");
        observe_server_message(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "result": {"tools": [
                    {"name": "press", "inputSchema": {"type": "object", "properties": {}}}
                ]}
            }),
            Some("tools/list"),
            &mut session,
        );
        assert!(session.tool_schemas.contains_key("press"));
    }

    #[test]
    fn sse_parser_emits_data_payloads_in_order() {
        let mut parser = SseParser::default();
        let payloads = parser
            .push(b"event: message\ndata: {\"a\":1}\n\ndata: {\"b\":2}\n\n")
            .unwrap();
        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0], b"{\"a\":1}");
        assert_eq!(payloads[1], b"{\"b\":2}");
    }

    #[test]
    fn sse_parser_handles_split_chunks_and_crlf() {
        let mut parser = SseParser::default();
        assert!(parser.push(b"data: {\"a\"").unwrap().is_empty());
        let payloads = parser.push(b":1}\r\n\r\n").unwrap();
        assert_eq!(payloads, vec![b"{\"a\":1}".to_vec()]);
    }

    #[test]
    fn sse_parser_ignores_comments_and_empty_dispatches() {
        let mut parser = SseParser::default();
        let payloads = parser.push(b": keep-alive\n\n").unwrap();
        assert!(payloads.is_empty());
        assert_eq!(parser.finish(), None);
    }

    #[test]
    fn instance_selection_requires_exactly_one_live_match() {
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("agent-control");
        let error = select_instance(&missing, None).unwrap_err();
        assert!(matches!(error, CarrierError::NoLiveInstance { .. }));
    }
}
