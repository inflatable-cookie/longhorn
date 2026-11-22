//! rmcp `ServerHandler` over the host's [`ControlHandler`].
//!
//! One tool method per vocabulary request; each deserializes directly into
//! the shared vocabulary type (except `resize_window`, whose flat wire shape
//! adapts through [`ResizeWindowArgs`]), awaits the host, and maps the
//! outcome onto a [`CallToolResult`]. Tool-level failures carry the typed
//! [`ToolError`] as JSON content so agents read the same error vocabulary
//! the host produced.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, ContentBlock, Implementation, ListResourcesResult,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerConfig, SubscriptionFilter,
    },
    serde::Serialize,
    service::SubscriptionContext,
    tool, tool_handler, tool_router,
};

use super::args::{ResizeWindowArgs, invalid_params};
use crate::{
    AnswerSelectionRequest, AnswerSelectionResult, ClickRequest, CommandRequest, ControlHandler,
    DragRequest, EvaluateRequest, EvaluateResult, PressRequest, RejectSelectionRequest,
    RejectSelectionResult, ScreenshotRequest, ScrollRequest, SelectionRegistry,
    SetFileInputRequest, SnapshotRequest, ToolError, TypeRequest, WaitForRequest,
};

use super::events;

/// Polling cadence for page-event subscriptions (`console`, `error`,
/// `navigation`).
///
/// Selection updates (`longhorn://agent-control/selection`) are pushed over
/// a `watch` channel with zero polling. Page events live in the in-page shim
/// ring because WKWebView has no host push channel for them; when at least
/// one page-event URI is subscribed, each listener polls `readEvents` on a
/// retained interval at this cadence (never reset by selection wakes).
const PAGE_EVENT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Per-poll evaluation bound for `readEvents` inside `listen`, separate from
/// [`PAGE_EVENT_POLL_INTERVAL`], so a wedged WebKit evaluation cannot hang a
/// listener indefinitely. In-flight evaluations are polled concurrently with
/// cancellation and selection pushes.
const PAGE_EVENT_EVAL_TIMEOUT: Duration = Duration::from_secs(2);

/// One batch returned by the in-page shim's `readEvents(sinceSeq, sinceEpoch)`.
#[derive(Debug, Eq, PartialEq)]
struct EventBatch {
    epoch: Option<String>,
    events: Vec<serde_json::Value>,
    next_seq: u64,
    dropped: u64,
}

/// Cursor tracking one listener's position across document reloads.
#[derive(Debug, Default, Eq, PartialEq)]
struct ListenCursor {
    epoch: Option<String>,
    seq: u64,
}

impl ListenCursor {
    /// Updates the cursor from `batch`.
    ///
    /// Returns `true` when the document epoch changed while `events` was empty
    /// and a non-zero prior cursor was passed (a 1-argument shim/stub that
    /// ignored `sinceEpoch` and filtered with the old `seq`), signaling the
    /// caller to immediately re-read from `seq = 0`.
    fn apply(&mut self, batch: &EventBatch) -> bool {
        let epoch_changed = match (&self.epoch, &batch.epoch) {
            (Some(prev), Some(curr)) => prev != curr,
            _ => false,
        };
        if batch.epoch.is_some() {
            self.epoch = batch.epoch.clone();
        }
        if epoch_changed {
            let had_cursor = self.seq > 0;
            self.seq = advance_listen_cursor(0, &batch.events);
            return had_cursor && batch.events.is_empty() && batch.next_seq > 1;
        }
        // Cursor is the last delivered event seq, not `nextSeq`. The ring
        // assigns `nextSeq` to the *next* push and filters `seq > sinceSeq`,
        // so using `nextSeq` as the cursor drops the first event after
        // subscribe (Card 237).
        self.seq = advance_listen_cursor(self.seq, &batch.events);
        false
    }
}

/// MCP service dispatching to the host's control authority.
#[derive(Clone)]
pub(super) struct AgentControlMcp<H> {
    handler: Arc<H>,
    registry: SelectionRegistry,
}

/// Maps a handler outcome onto a tool result: success as JSON content,
/// typed failure as an `isError` result carrying the [`ToolError`] JSON.
fn json_result<T: Serialize>(outcome: Result<T, ToolError>) -> Result<CallToolResult, ErrorData> {
    match outcome {
        Ok(value) => {
            ContentBlock::json(&value).map(|content| CallToolResult::success(vec![content]))
        }
        Err(error) => {
            ContentBlock::json(&error).map(|content| CallToolResult::error(vec![content]))
        }
    }
}

/// Server instructions. The evaluate-on wording is also the artifact-scan
/// marker for `agent-control-evaluate` (`full code execution in the app`).
fn instructions() -> &'static str {
    #[cfg(feature = "agent-control-evaluate")]
    {
        "Longhorn agent app control (contract 022): snapshot the semantic tree, act by element ref with untrusted synthetic events, evaluate JS as an escape hatch (full code execution in the app), wait on DOM-relative predicates, capture fresh window images, invoke registered commands for native-chrome behavior, and answer pending file/folder selection with answer_selection/reject_selection. Subscribe to longhorn://agent-control/{console,error,navigation,selection} over subscriptions/listen for page events and pending pickers. Stateless: every call is self-contained."
    }
    #[cfg(not(feature = "agent-control-evaluate"))]
    {
        "Longhorn agent app control (contract 022): snapshot the semantic tree, act by element ref with untrusted synthetic events, evaluate answers typed Unsupported unless agent-control-evaluate is enabled, wait on DOM-relative predicates, capture fresh window images, invoke registered commands for native-chrome behavior, and answer pending file/folder selection with answer_selection/reject_selection. Subscribe to longhorn://agent-control/{console,error,navigation,selection} over subscriptions/listen for page events and pending pickers. Stateless: every call is self-contained."
    }
}

#[tool_router]
impl<H> AgentControlMcp<H>
where
    H: ControlHandler,
{
    pub(super) fn new(handler: Arc<H>, registry: SelectionRegistry) -> Self {
        Self { handler, registry }
    }

    #[tool(
        description = "Semantic element tree of the targeted webview (roles, names, values, state) with element refs stamped into the live DOM. Omit webview for the window's UI webview; an explicit webview label addresses an opted-in child. Refs are scoped to the webview that stamped them and resolve against the live DOM on use."
    )]
    async fn snapshot(
        &self,
        Parameters(request): Parameters<SnapshotRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.snapshot(request).await)
    }

    #[tool(
        description = "Synthetic in-page click on an element ref. Dispatched as untrusted DOM events: never moves the OS pointer, never requires focus, and does not satisfy isTrusted checks."
    )]
    async fn click(
        &self,
        Parameters(request): Parameters<ClickRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.click(request).await)
    }

    #[tool(
        name = "type",
        description = "Synthetic in-page text entry into an element ref, as untrusted DOM events. Never moves the OS pointer and never requires focus."
    )]
    async fn r#type(
        &self,
        Parameters(request): Parameters<TypeRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.r#type(request).await)
    }

    #[tool(
        description = "Synthetic in-page key press with optional modifiers, as untrusted DOM events. Never requires OS focus."
    )]
    async fn press(
        &self,
        Parameters(request): Parameters<PressRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.press(request).await)
    }

    #[tool(
        description = "Synthetic in-page scroll of an element or the document, as untrusted DOM events."
    )]
    async fn scroll(
        &self,
        Parameters(request): Parameters<ScrollRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.scroll(request).await)
    }

    #[tool(
        description = "Synthetic in-page drag between two element refs. Untrusted DOM events only: native hover and OS drag-and-drop are out of scope, and there is no OS-level mode."
    )]
    async fn drag(
        &self,
        Parameters(request): Parameters<DragRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.drag(request).await)
    }

    #[tool(
        description = "Supply inline file content to an HTML file input by element ref. The agent is the byte source: there is no path argument and Longhorn never reads the filesystem. Dispatches untrusted input and change events and never opens an OS panel."
    )]
    async fn set_file_input(
        &self,
        Parameters(request): Parameters<SetFileInputRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let validated = request
            .into_validated()
            .map_err(|error| invalid_params(error.to_string()))?;
        json_result(self.handler.set_file_input_prevalidated(validated).await)
    }

    #[tool(
        description = "Run JavaScript in the page and return the JSON result. Escape hatch, not the primary path. Packaged agent-control builds omit execution and answer typed Unsupported unless agent-control-evaluate is enabled."
    )]
    async fn evaluate(
        &self,
        Parameters(request): Parameters<EvaluateRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        #[cfg(feature = "agent-control-evaluate")]
        {
            json_result::<EvaluateResult>(self.handler.evaluate(request).await)
        }
        #[cfg(not(feature = "agent-control-evaluate"))]
        {
            let _ = request;
            json_result::<EvaluateResult>(Err(ToolError::Unsupported {
                message:
                    "evaluate is omitted from this build; enable the agent-control-evaluate feature"
                        .to_owned(),
            }))
        }
    }

    #[tool(
        description = "Wait until a DOM-relative predicate holds (ref resolves, ref absent, page URL or title contains), bounded by timeoutMs. No time-only or animation-frame waits exist: WKWebView timer coalescing and rAF suspension make them meaningless while unfocused."
    )]
    async fn wait_for(
        &self,
        Parameters(request): Parameters<WaitForRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        request
            .validate()
            .map_err(|error| invalid_params(error.to_string()))?;
        json_result(self.handler.wait_for(request).await)
    }

    #[tool(
        description = "Fresh PNG snapshot of a window via webview capture. Works occluded, unfocused, and minimized; requires no screen-recording permission."
    )]
    async fn screenshot(
        &self,
        Parameters(request): Parameters<ScreenshotRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.handler.screenshot(request).await {
            Ok(result) => Ok(CallToolResult::success(vec![ContentBlock::image(
                BASE64.encode(result.png),
                "image/png",
            )])),
            Err(error) => {
                ContentBlock::json(&error).map(|content| CallToolResult::error(vec![content]))
            }
        }
    }

    #[tool(
        description = "Invoke a registered contract-006 command by id. The route to behavior behind native menus and dialogs; agents do not click native chrome."
    )]
    async fn command(
        &self,
        Parameters(request): Parameters<CommandRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.command(request).await)
    }

    #[tool(description = "List every window the app exposes to the control surface.")]
    async fn list_windows(&self) -> Result<CallToolResult, ErrorData> {
        json_result(
            self.handler
                .list_windows(crate::ListWindowsRequest {})
                .await,
        )
    }

    #[tool(description = "Resize one window's content area to a logical-pixel size.")]
    async fn resize_window(
        &self,
        Parameters(args): Parameters<ResizeWindowArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.resize_window(args.into_request()?).await)
    }

    #[tool(
        description = "Answer a pending agent-originated file or folder picker with paths. Single open and save take one path; multiple open takes one or more. Open paths must exist as the requested kind (file or directory). Save chooses a target; Longhorn never writes it. Unknown, expired, settled, or malformed answers fail typed."
    )]
    async fn answer_selection(
        &self,
        Parameters(request): Parameters<AnswerSelectionRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result::<AnswerSelectionResult>(
            self.registry
                .answer(&request.id, &request.paths)
                .map(|()| AnswerSelectionResult {}),
        )
    }

    #[tool(
        description = "Reject a pending agent-originated picker. The app call resolves as null, the plugin's cancellation result. Unknown, expired, or already-settled ids fail typed."
    )]
    async fn reject_selection(
        &self,
        Parameters(request): Parameters<RejectSelectionRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result::<RejectSelectionResult>(
            self.registry
                .reject(&request.id)
                .map(|()| RejectSelectionResult {}),
        )
    }

    async fn read_events(
        &self,
        since_seq: u64,
        since_epoch: Option<&str>,
    ) -> Result<EventBatch, ToolError> {
        let args = match since_epoch {
            Some(epoch) => format!(
                "{since_seq}, {}",
                serde_json::Value::String(epoch.to_owned())
            ),
            None => format!("{since_seq}"),
        };
        let value = self
            .handler
            .evaluate(EvaluateRequest {
                window: None,
                webview: None,
                js: format!(
                    "JSON.stringify(globalThis.__longhornAgentControl ? globalThis.__longhornAgentControl.readEvents({args}) : {{events:[], nextSeq:{since_seq}, dropped:0}})"
                ),
            })
            .await?
            .value;
        let parsed = match value {
            serde_json::Value::String(text) => {
                serde_json::from_str(&text).map_err(|error| ToolError::EvaluationFailed {
                    message: format!("readEvents JSON did not parse: {error}"),
                })?
            }
            other => other,
        };
        let epoch = parsed
            .get("epoch")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let events = parsed
            .get("events")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let next_seq = parsed
            .get("nextSeq")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(since_seq);
        let dropped = parsed
            .get("dropped")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        Ok(EventBatch {
            epoch,
            events,
            next_seq,
            dropped,
        })
    }

    async fn read_event_resource(&self, uri: &str) -> Result<String, ErrorData> {
        let kind = events::kind_for_uri(uri).ok_or_else(|| {
            ErrorData::resource_not_found(format!("unknown resource {uri}"), None)
        })?;
        let batch = self
            .read_events(0, None)
            .await
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        let filtered: Vec<_> = batch
            .events
            .into_iter()
            .filter(|event| event.get("kind").and_then(serde_json::Value::as_str) == Some(kind))
            .collect();
        let mut body = serde_json::json!({
            "events": filtered,
            "nextSeq": batch.next_seq,
            "dropped": batch.dropped,
        });
        if let Some(epoch) = batch.epoch {
            body["epoch"] = serde_json::Value::String(epoch);
        }
        Ok(body.to_string())
    }
}

#[tool_handler]
impl<H> ServerHandler for AgentControlMcp<H>
where
    H: ControlHandler,
{
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_subscribe()
                .build(),
        )
        .with_server_info(Implementation::new(
            "longhorn-agent-control",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(instructions())
    }

    fn accepted_subscription_filter(
        &self,
        requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        Some(
            requested.intersection(
                &SubscriptionFilter::builder()
                    .resource_subscription(events::CONSOLE_URI)
                    .resource_subscription(events::ERROR_URI)
                    .resource_subscription(events::NAVIGATION_URI)
                    .resource_subscription(events::SELECTION_URI)
                    .build(),
            ),
        )
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(events::all_resources()))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ReadResourceResponse, ErrorData> {
        if request.uri == events::SELECTION_URI {
            let body = serde_json::to_string(&self.registry.resource_body()).map_err(|error| {
                ErrorData::internal_error(format!("selection resource JSON: {error}"), None)
            })?;
            return Ok(
                ReadResourceResult::new(vec![ResourceContents::text(body, request.uri)]).into(),
            );
        }
        if !events::known_uri(&request.uri) {
            return Err(ErrorData::resource_not_found(
                format!("unknown resource {}", request.uri),
                None,
            ));
        }
        let body = self.read_event_resource(&request.uri).await?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(body, request.uri)]).into())
    }

    async fn listen(&self, context: SubscriptionContext) -> Result<(), ErrorData> {
        let sink = context.sink().clone();
        let accepted = context.accepted().clone();
        let mut cursor = ListenCursor::default();
        let selection_subscribed = accepted
            .resource_subscriptions
            .as_ref()
            .is_some_and(|uris| uris.iter().any(|item| item == events::SELECTION_URI));
        let page_events_subscribed = accepted
            .resource_subscriptions
            .as_ref()
            .is_some_and(|uris| {
                uris.iter().any(|item| {
                    item == events::CONSOLE_URI
                        || item == events::ERROR_URI
                        || item == events::NAVIGATION_URI
                })
            });
        let mut selection_rx = self.registry.subscribe();
        let mut page_interval = tokio::time::interval_at(
            tokio::time::Instant::now() + PAGE_EVENT_POLL_INTERVAL,
            PAGE_EVENT_POLL_INTERVAL,
        );
        page_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut pending_eval: Option<
            Pin<Box<dyn Future<Output = Option<EventBatch>> + Send + '_>>,
        > = None;

        loop {
            tokio::select! {
                () = context.cancelled() => break,
                changed = selection_rx.changed(), if selection_subscribed => {
                    if changed.is_err() {
                        break;
                    }
                    let _ = sink.notify_resource_updated(events::SELECTION_URI).await;
                }
                batch = async { pending_eval.as_mut().expect("checked is_some").await }, if pending_eval.is_some() => {
                    pending_eval = None;
                    let Some(batch) = batch else {
                        // No page yet, evaluate failed, or timed out: keep the
                        // stream alive; the next tick retries.
                        continue;
                    };
                    if cursor.apply(&batch) {
                        // 1-arg shim/stub ignored sinceEpoch and filtered with
                        // the old seq; immediately schedule a fresh seq=0 read.
                        let epoch = cursor.epoch.clone();
                        pending_eval = Some(Box::pin(async move {
                            tokio::time::timeout(
                                PAGE_EVENT_EVAL_TIMEOUT,
                                self.read_events(0, epoch.as_deref()),
                            )
                            .await
                            .ok()
                            .and_then(Result::ok)
                        }));
                        continue;
                    }
                    for event in batch.events {
                        let Some(uri) = event
                            .get("kind")
                            .and_then(serde_json::Value::as_str)
                            .and_then(events::uri_for_kind)
                        else {
                            continue;
                        };
                        let subscribed = accepted
                            .resource_subscriptions
                            .as_ref()
                            .is_some_and(|uris| uris.iter().any(|item| item == uri));
                        if subscribed {
                            let _ = sink.notify_resource_updated(uri).await;
                        }
                    }
                }
                _ = page_interval.tick(), if page_events_subscribed && pending_eval.is_none() => {
                    let seq = cursor.seq;
                    let epoch = cursor.epoch.clone();
                    pending_eval = Some(Box::pin(async move {
                        tokio::time::timeout(
                            PAGE_EVENT_EVAL_TIMEOUT,
                            self.read_events(seq, epoch.as_deref()),
                        )
                        .await
                        .ok()
                        .and_then(Result::ok)
                    }));
                }
            }
        }
        Ok(())
    }
}

/// Advance the listen cursor past events already delivered.
///
/// `nextSeq` from the ring is the next seq to assign, not a since-cursor.
/// An empty batch leaves `previous` unchanged so a later first event is
/// still `seq > previous`.
fn advance_listen_cursor(previous: u64, events: &[serde_json::Value]) -> u64 {
    events
        .iter()
        .filter_map(|event| event.get("seq").and_then(serde_json::Value::as_u64))
        .max()
        .unwrap_or(previous)
}

#[cfg(test)]
mod cursor_tests {
    use super::{EventBatch, ListenCursor, advance_listen_cursor};
    use serde_json::json;

    #[test]
    fn empty_batch_keeps_the_previous_cursor() {
        assert_eq!(advance_listen_cursor(0, &[]), 0);
        assert_eq!(advance_listen_cursor(3, &[]), 3);
    }

    #[test]
    fn cursor_is_the_last_delivered_seq_not_next_seq() {
        let events = vec![
            json!({"seq": 1, "kind": "navigation"}),
            json!({"seq": 2, "kind": "navigation"}),
        ];
        // nextSeq would be 3 here; using it would drop the next event.
        assert_eq!(advance_listen_cursor(0, &events), 2);
    }

    #[test]
    fn epoch_change_resets_cursor_for_reloaded_document() {
        let mut cursor = ListenCursor::default();
        let initial = EventBatch {
            epoch: Some("doc-1".to_owned()),
            events: vec![
                json!({"seq": 1, "kind": "console"}),
                json!({"seq": 5, "kind": "console"}),
            ],
            next_seq: 6,
            dropped: 0,
        };
        assert!(!cursor.apply(&initial));
        assert_eq!(cursor.epoch.as_deref(), Some("doc-1"));
        assert_eq!(cursor.seq, 5);

        // Reload with an event at seq 1 already returned via sinceEpoch mismatch.
        let reloaded = EventBatch {
            epoch: Some("doc-2".to_owned()),
            events: vec![json!({"seq": 1, "kind": "navigation"})],
            next_seq: 2,
            dropped: 0,
        };
        assert!(!cursor.apply(&reloaded));
        assert_eq!(cursor.epoch.as_deref(), Some("doc-2"));
        assert_eq!(cursor.seq, 1);

        // Reload with no events yet resets seq to 0 so the next seq 1 event is not dropped.
        let empty_reload = EventBatch {
            epoch: Some("doc-3".to_owned()),
            events: vec![],
            next_seq: 1,
            dropped: 0,
        };
        assert!(!cursor.apply(&empty_reload));
        assert_eq!(cursor.epoch.as_deref(), Some("doc-3"));
        assert_eq!(cursor.seq, 0);

        // Reload against a 1-arg stub that ignored sinceEpoch and filtered out seq 1:
        // signals immediate re-read from seq 0.
        cursor.seq = 4;
        let filtered_reload = EventBatch {
            epoch: Some("doc-4".to_owned()),
            events: vec![],
            next_seq: 2,
            dropped: 0,
        };
        assert!(cursor.apply(&filtered_reload));
        assert_eq!(cursor.epoch.as_deref(), Some("doc-4"));
        assert_eq!(cursor.seq, 0);
    }
}
