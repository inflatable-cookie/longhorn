//! Host-agnostic core of the Longhorn agent app-control surface
//! (contract 022).
//!
//! The contract's vocabulary as types: tool requests, results, and errors;
//! the discovery-file lifecycle; the per-instance bearer token; the
//! native-surface provider seam; and the stateless MCP streamable-HTTP
//! server assembly (Card 229) over rmcp. The `evaluate` JS escape hatch is
//! behind the off-by-default `agent-control-evaluate` feature; without it the
//! tool answers typed `Unsupported`. No host dependency — a Tauri host
//! (g02.031) or a GPUI host mounts the router and implements
//! [`ControlHandler`].

#[cfg(feature = "client")]
pub mod carrier;
mod discovery;
mod handler;
mod provider;
mod selection;
mod server;
mod token;
mod tools;

pub use discovery::{
    DISCOVERY_SCHEMA_VERSION, DiscoveryError, DiscoveryFile, DiscoveryInstance, DiscoveryRecord,
    DiscoveryScan, enumerate_discovery, enumerate_discovery_with, process_alive, publish_discovery,
    remove_discovery_file, resolve_discovery_dir, resolve_discovery_dir_with_state_override,
    sweep_stale_discovery, sweep_stale_discovery_with,
};
pub use handler::ControlHandler;
pub use provider::{NativeSurfaceAction, NativeSurfaceProvider};
pub use selection::{
    BeginSelectionError, BeginSelectionRequest, DEFAULT_SELECTION_TTL, PendingSelection,
    SelectionFilter, SelectionKind, SelectionOutcome, SelectionRegistry, SelectionResource,
    SelectionWait, path_string, plugin_result,
};
pub use server::{
    ControlServerConfig, ServeError, ServeReceipt, control_router, serve_control_surface,
};
pub use token::{InstanceToken, TokenError};
pub use tools::{
    ActionReceipt, AnswerSelectionRequest, AnswerSelectionResult, CONTROL_TOOL_NAMES, ClickRequest,
    CommandRequest, CommandResult, DragRequest, ElementRef, EvaluateRequest, EvaluateResult,
    KeyModifier, ListWindowsRequest, ListWindowsResult, PageState, PressRequest,
    RejectSelectionRequest, RejectSelectionResult, ResizeWindowRequest, ScreenshotRequest,
    ScreenshotResult, ScrollRequest, SelectionId, SemanticNode, SnapshotRequest, SnapshotResult,
    ToolError, TypeRequest, WaitForRequest, WaitForResult, WaitPredicate, WebviewLabel,
    WebviewTarget, WindowInfo, WindowTarget,
};
