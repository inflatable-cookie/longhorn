//! rmcp `ServerHandler` over the host's [`ControlHandler`].
//!
//! One tool method per vocabulary request; each validates its wire
//! arguments, awaits the host, and maps the outcome onto a
//! [`CallToolResult`]. Tool-level failures carry the typed [`ToolError`] as
//! JSON content so agents read the same error vocabulary the host produced.

use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, ContentBlock, Implementation, ListResourcesResult,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerInfo, SubscriptionFilter,
    },
    serde::Serialize,
    service::SubscriptionContext,
    tool, tool_handler, tool_router,
};

use super::args::{
    AnswerSelectionArgs, ClickArgs, CommandArgs, DragArgs, EvaluateArgs, PressArgs,
    RejectSelectionArgs, ResizeWindowArgs, ScreenshotArgs, ScrollArgs, SnapshotArgs, TypeArgs,
    WaitForArgs,
};
use crate::{
    AnswerSelectionResult, ControlHandler, EvaluateRequest, EvaluateResult, RejectSelectionResult,
    SelectionRegistry, ToolError,
};

use super::events;

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
        Parameters(args): Parameters<SnapshotArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.snapshot(args.into_request()?).await)
    }

    #[tool(
        description = "Synthetic in-page click on an element ref. Dispatched as untrusted DOM events: never moves the OS pointer, never requires focus, and does not satisfy isTrusted checks."
    )]
    async fn click(
        &self,
        Parameters(args): Parameters<ClickArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.click(args.into_request()?).await)
    }

    #[tool(
        name = "type",
        description = "Synthetic in-page text entry into an element ref, as untrusted DOM events. Never moves the OS pointer and never requires focus."
    )]
    async fn r#type(
        &self,
        Parameters(args): Parameters<TypeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.r#type(args.into_request()?).await)
    }

    #[tool(
        description = "Synthetic in-page key press with optional modifiers, as untrusted DOM events. Never requires OS focus."
    )]
    async fn press(
        &self,
        Parameters(args): Parameters<PressArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.press(args.into_request()?).await)
    }

    #[tool(
        description = "Synthetic in-page scroll of an element or the document, as untrusted DOM events."
    )]
    async fn scroll(
        &self,
        Parameters(args): Parameters<ScrollArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.scroll(args.into_request()?).await)
    }

    #[tool(
        description = "Synthetic in-page drag between two element refs. Untrusted DOM events only: native hover and OS drag-and-drop are out of scope, and there is no OS-level mode."
    )]
    async fn drag(
        &self,
        Parameters(args): Parameters<DragArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.drag(args.into_request()?).await)
    }

    #[tool(
        description = "Run JavaScript in the page and return the JSON result. Escape hatch, not the primary path. Packaged agent-control builds omit execution and answer typed Unsupported unless agent-control-evaluate is enabled."
    )]
    async fn evaluate(
        &self,
        Parameters(args): Parameters<EvaluateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let request = args.into_request()?;
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
        Parameters(args): Parameters<WaitForArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.wait_for(args.into_request()?).await)
    }

    #[tool(
        description = "Fresh PNG snapshot of a window via webview capture. Works occluded, unfocused, and minimized; requires no screen-recording permission."
    )]
    async fn screenshot(
        &self,
        Parameters(args): Parameters<ScreenshotArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.handler.screenshot(args.into_request()?).await {
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
        Parameters(args): Parameters<CommandArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.command(args.into_request()?).await)
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
        Parameters(args): Parameters<AnswerSelectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let request = args.into_request()?;
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
        Parameters(args): Parameters<RejectSelectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let request = args.into_request()?;
        json_result::<RejectSelectionResult>(
            self.registry
                .reject(&request.id)
                .map(|()| RejectSelectionResult {}),
        )
    }

    async fn read_events(
        &self,
        since_seq: u64,
    ) -> Result<(Vec<serde_json::Value>, u64, u64), ToolError> {
        let value = self
            .handler
            .evaluate(EvaluateRequest {
                window: None,
                webview: None,
                js: format!(
                    "JSON.stringify(globalThis.__longhornAgentControl ? globalThis.__longhornAgentControl.readEvents({since_seq}) : {{events:[], nextSeq:{since_seq}, dropped:0}})"
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
        Ok((events, next_seq, dropped))
    }

    async fn read_event_resource(&self, uri: &str) -> Result<String, ErrorData> {
        let kind = events::kind_for_uri(uri).ok_or_else(|| {
            ErrorData::resource_not_found(format!("unknown resource {uri}"), None)
        })?;
        let (events, next_seq, dropped) = self
            .read_events(0)
            .await
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        let filtered: Vec<_> = events
            .into_iter()
            .filter(|event| event.get("kind").and_then(serde_json::Value::as_str) == Some(kind))
            .collect();
        Ok(serde_json::json!({
            "events": filtered,
            "nextSeq": next_seq,
            "dropped": dropped,
        })
        .to_string())
    }
}

#[tool_handler]
impl<H> ServerHandler for AgentControlMcp<H>
where
    H: ControlHandler,
{
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
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
        let mut seq = 0_u64;
        let mut selection_generation = self.registry.generation();
        let selection_subscribed = accepted
            .resource_subscriptions
            .as_ref()
            .is_some_and(|uris| uris.iter().any(|item| item == events::SELECTION_URI));
        loop {
            tokio::select! {
                () = context.cancelled() => break,
                () = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                    if selection_subscribed {
                        let current = self.registry.generation();
                        if current != selection_generation {
                            selection_generation = current;
                            let _ = sink.notify_resource_updated(events::SELECTION_URI).await;
                        }
                    }
                    match self.read_events(seq).await {
                        Ok((events, _next_seq, _dropped)) => {
                            // Cursor is the last delivered event seq, not
                            // `nextSeq`. The ring assigns `nextSeq` to the
                            // *next* push and filters `seq > sinceSeq`, so
                            // using `nextSeq` as the cursor drops the first
                            // event after subscribe (Card 237).
                            seq = advance_listen_cursor(seq, &events);
                            for event in events {
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
                        Err(_) => {
                            // No page yet, or evaluate failed: keep the stream
                            // alive; the next tick retries.
                        }
                    }
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
    use super::advance_listen_cursor;
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
}
