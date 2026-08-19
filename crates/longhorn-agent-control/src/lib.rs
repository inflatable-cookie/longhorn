//! Host-agnostic core of the Longhorn agent app-control surface
//! (contract 022).
//!
//! The contract's vocabulary as types: tool requests, results, and errors;
//! the discovery-file lifecycle; the per-instance bearer token; and the
//! native-surface provider seam. No server and no host dependency — the
//! stateless MCP streamable-HTTP assembly (Card 229) builds on this
//! vocabulary, and hosts (Tauri in g02.031, a GPUI host with its own
//! provider or nothing) mount it.

mod discovery;
mod provider;
mod token;
mod tools;

pub use discovery::{
    DISCOVERY_SCHEMA_VERSION, DiscoveryError, DiscoveryFile, DiscoveryInstance, DiscoveryRecord,
    DiscoveryScan, enumerate_discovery, enumerate_discovery_with, process_alive, publish_discovery,
    remove_discovery_file, resolve_discovery_dir, resolve_discovery_dir_with_state_override,
};
pub use provider::{NativeSurfaceAction, NativeSurfaceProvider};
pub use token::{InstanceToken, TokenError};
pub use tools::{
    ActionReceipt, ClickRequest, CommandRequest, CommandResult, DragRequest, ElementRef,
    EvaluateRequest, EvaluateResult, KeyModifier, ListWindowsRequest, ListWindowsResult, PageState,
    PressRequest, ResizeWindowRequest, ScreenshotRequest, ScreenshotResult, ScrollRequest,
    SemanticNode, SnapshotRequest, SnapshotResult, ToolError, TypeRequest, WaitForRequest,
    WaitForResult, WaitPredicate, WindowInfo, WindowTarget,
};
