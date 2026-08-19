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
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo},
    serde::Serialize,
    tool, tool_handler, tool_router,
};

use super::args::{
    ClickArgs, CommandArgs, DragArgs, EvaluateArgs, PressArgs, ResizeWindowArgs, ScreenshotArgs,
    ScrollArgs, SnapshotArgs, TypeArgs, WaitForArgs,
};
use crate::{ControlHandler, ToolError};

/// MCP service dispatching to the host's control authority.
#[derive(Clone)]
pub(super) struct AgentControlMcp<H> {
    handler: Arc<H>,
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

#[tool_router]
impl<H> AgentControlMcp<H>
where
    H: ControlHandler,
{
    pub(super) fn new(handler: Arc<H>) -> Self {
        Self { handler }
    }

    #[tool(
        description = "Semantic element tree of the webview (roles, names, values, state) with element refs stamped into the live DOM. Refs resolve against the live DOM on use; a ref from any prior snapshot either resolves or fails explicitly."
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
        description = "Run JavaScript in the page and return the JSON result. Escape hatch, not the primary path; full code execution in the app."
    )]
    async fn evaluate(
        &self,
        Parameters(args): Parameters<EvaluateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json_result(self.handler.evaluate(args.into_request()?).await)
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
}

#[tool_handler]
impl<H> ServerHandler for AgentControlMcp<H>
where
    H: ControlHandler,
{
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "longhorn-agent-control",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Longhorn agent app control (contract 022): snapshot the semantic tree, act by element ref with untrusted synthetic events, evaluate JS as an escape hatch, wait on DOM-relative predicates, capture fresh window images, and invoke registered commands for native-chrome behavior. Stateless: every call is self-contained.",
            )
    }
}
