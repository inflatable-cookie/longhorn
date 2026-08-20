//! Host-agnostic core of the Longhorn agent app-control surface
//! (contract 022).
//!
//! The contract's vocabulary as types: tool requests, results, and errors;
//! the discovery-file lifecycle; the per-instance bearer token; the
//! native-surface provider seam; and the stateless MCP streamable-HTTP
//! server assembly (Card 229) over rmcp. No host dependency — a Tauri host
//! (g02.031) or a GPUI host mounts the router and implements
//! [`ControlHandler`].

mod discovery;
mod handler;
mod provider;
mod server;
mod token;
mod tools;

pub use discovery::{
    DISCOVERY_SCHEMA_VERSION, DiscoveryError, DiscoveryFile, DiscoveryInstance, DiscoveryRecord,
    DiscoveryScan, enumerate_discovery, enumerate_discovery_with, process_alive, publish_discovery,
    remove_discovery_file, resolve_discovery_dir, resolve_discovery_dir_with_state_override,
};
pub use handler::ControlHandler;
pub use provider::{NativeSurfaceAction, NativeSurfaceProvider};
pub use server::{
    ControlServerConfig, ServeError, ServeReceipt, control_router, serve_control_surface,
};
pub use token::{InstanceToken, TokenError};
pub use tools::{
    ActionReceipt, CONTROL_TOOL_NAMES, ClickRequest, CommandRequest, CommandResult, DragRequest,
    ElementRef, EvaluateRequest, EvaluateResult, KeyModifier, ListWindowsRequest,
    ListWindowsResult, PageState, PressRequest, ResizeWindowRequest, ScreenshotRequest,
    ScreenshotResult, ScrollRequest, SemanticNode, SnapshotRequest, SnapshotResult, ToolError,
    TypeRequest, WaitForRequest, WaitForResult, WaitPredicate, WebviewLabel, WebviewTarget,
    WindowInfo, WindowTarget,
};
