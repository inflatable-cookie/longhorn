//! Stateless MCP streamable-HTTP server mounted inside the app process.
//!
//! A background thread owns a tokio runtime and an axum server bound to
//! `127.0.0.1:0`; rmcp's `StreamableHttpService` is nested at `/mcp` with
//! `legacy_session_mode: false`, so no session ids are minted. The bind
//! address is printed to stdout as the discovery line for clients.

use std::{sync::Arc, thread};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo},
    schemars, serde, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use tauri::WebviewWindow;

use crate::control;

/// Spawns the MCP server thread for `window`.
pub fn spawn(window: WebviewWindow) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime must build");
        runtime.block_on(serve(window));
    });
}

async fn serve(window: WebviewWindow) {
    let service = StreamableHttpService::new(
        move || Ok(AgentControl::new(window.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_legacy_session_mode(false),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("MCP listener must bind");
    let address = listener.local_addr().expect("listener address");
    println!("agent-control: listening on http://{address}/mcp");
    axum::serve(listener, router)
        .await
        .expect("MCP server failed");
}

/// MCP service exposing `evaluate` and `screenshot` against the webview.
#[derive(Clone)]
pub struct AgentControl {
    window: WebviewWindow,
}

/// Arguments for the `evaluate` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EvaluateArgs {
    /// JavaScript source evaluated in the webview's main world.
    pub js: String,
}

#[tool_router]
impl AgentControl {
    fn new(window: WebviewWindow) -> Self {
        Self { window }
    }

    #[tool(
        description = "Evaluate JavaScript in the app's WKWebView and return the result as text (strings and numbers as plain text, other values via their description, undefined as \"undefined\")"
    )]
    async fn evaluate(
        &self,
        Parameters(EvaluateArgs { js }): Parameters<EvaluateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        match control::evaluate(&self.window, js).await {
            Ok(text) => Ok(CallToolResult::success(vec![ContentBlock::text(text)])),
            Err(message) => Ok(CallToolResult::error(vec![ContentBlock::text(message)])),
        }
    }

    #[tool(
        description = "Take a fresh WKWebView snapshot of the app window and return it as a PNG image"
    )]
    async fn screenshot(&self) -> Result<CallToolResult, ErrorData> {
        let png = control::screenshot(&self.window)
            .await
            .map_err(|message| ErrorData::internal_error(message, None))?;
        Ok(CallToolResult::success(vec![ContentBlock::image(
            BASE64.encode(png),
            "image/png",
        )]))
    }
}

#[tool_handler]
impl ServerHandler for AgentControl {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("longhorn-agent-control-proof", "0.0.0"))
            .with_instructions(
                "Controls the Longhorn agent-control proof app: evaluate runs JS in the WKWebView, screenshot returns a fresh PNG snapshot of it.",
            )
    }
}
