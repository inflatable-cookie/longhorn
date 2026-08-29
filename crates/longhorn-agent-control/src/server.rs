//! Stateless MCP streamable-HTTP server assembly (Card 229).
//!
//! rmcp's `StreamableHttpService` with `legacy_session_mode: false` — the
//! Card 227 configuration: POST-only, no minted session ids — nested at
//! `/mcp` on an axum router a host later mounts or serves directly. The
//! bearer-token and `Origin` guard runs as an outer layer, so rejection
//! happens before the MCP service, and therefore before any tool dispatch.
//!
//! The server binds 127.0.0.1 only; port 0 asks the OS for an ephemeral
//! port. [`serve_control_surface`] ties the discovery file's lifetime to
//! the server's: publish after bind (with the real port), remove on clean
//! shutdown.

mod args;
mod events;
mod guard;
mod mcp;

use std::{
    error::Error,
    fmt,
    future::Future,
    io,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::net::TcpListener;

use crate::{ControlHandler, DiscoveryError, InstanceToken, TokenError, publish_discovery};

/// Configuration for one control-surface server instance.
#[derive(Clone, Debug)]
pub struct ControlServerConfig {
    /// Canonical application id written to the discovery file.
    pub app_id: String,
    /// Resolved discovery directory (see `resolve_discovery_dir`).
    pub discovery_dir: PathBuf,
    /// Loopback port to bind; 0 asks the OS for an ephemeral port.
    pub port: u16,
}

/// Server startup, serving, or discovery-lifecycle failure.
#[derive(Debug)]
pub enum ServeError {
    /// Token generation failed.
    Token(TokenError),
    /// Discovery publication or removal failed.
    Discovery(DiscoveryError),
    /// Listener bind or serving failed.
    Io(io::Error),
}

impl fmt::Display for ServeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Token(source) => write!(formatter, "token generation failed: {source}"),
            Self::Discovery(source) => write!(formatter, "discovery lifecycle failed: {source}"),
            Self::Io(source) => write!(formatter, "control server I/O failed: {source}"),
        }
    }
}

impl Error for ServeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Token(source) => Some(source),
            Self::Discovery(source) => Some(source),
            Self::Io(source) => Some(source),
        }
    }
}

/// What one completed server run bound and served.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServeReceipt {
    /// Address the listener was bound to (127.0.0.1, real port).
    pub bound: SocketAddr,
}

/// Builds the control-surface router: the stateless MCP service nested at
/// `/mcp` behind the bearer-token and `Origin` guard. Hosts mounting the
/// surface into a larger app compose this router; the guard is inside it,
/// so mounting cannot forget the trust boundary.
pub fn control_router<H>(handler: H, token: InstanceToken) -> Router
where
    H: ControlHandler,
{
    let handler = Arc::new(handler);
    let service = StreamableHttpService::new(
        move || Ok(mcp::AgentControlMcp::new(Arc::clone(&handler))),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_legacy_session_mode(false),
    );
    Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(
            guard::GuardState::new(token),
            guard::auth_origin_guard,
        ))
}

/// Serves the control surface on 127.0.0.1 until `shutdown` resolves.
///
/// Generates the instance token, binds, publishes the discovery file with
/// the bound port (after sweeping dead-pid leftovers), serves, and removes
/// the file on the way out.
pub async fn serve_control_surface<H>(
    config: ControlServerConfig,
    handler: H,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<ServeReceipt, ServeError>
where
    H: ControlHandler,
{
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.port))
        .await
        .map_err(ServeError::Io)?;
    let bound = listener.local_addr().map_err(ServeError::Io)?;

    let token = InstanceToken::generate().map_err(ServeError::Token)?;
    let instance = publish_discovery(
        &config.discovery_dir,
        &config.app_id,
        bound.port(),
        token.clone(),
    )
    .map_err(ServeError::Discovery)?;

    let serve_result = axum::serve(listener, control_router(handler, token))
        .with_graceful_shutdown(shutdown)
        .await;

    // Clean exit removes the file; removal failure surfaces even when
    // serving itself succeeded, because a leftover live-pid file is a
    // credential-bearing lie.
    let removal = instance.remove();
    serve_result.map_err(ServeError::Io)?;
    removal.map_err(ServeError::Discovery)?;

    Ok(ServeReceipt { bound })
}
