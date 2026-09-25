//! Tauri host wiring for the Longhorn agent app-control surface
//! (contract 022).
//!
//! This crate composes the host-agnostic `longhorn-agent-control` server
//! inside a running Tauri application: it mounts the stateless MCP server on
//! a background thread, implements `ControlHandler` against the app's
//! windows, and routes `command` invocations into the app's contract-006
//! registry through the host-supplied [`CommandBridge`].
//!
//! The entire surface sits behind the off-by-default `agent-control` cargo
//! feature. Gating is compile-time and total: a build without the feature
//! compiles to an empty library — no server, route, token, or discovery code
//! exists in the artifact, and no runtime toggle can enable it (contract 022;
//! `scripts/verify-agent-control-release-absence.ts` proves the three feature
//! states). A consumer may enable `agent-control` in a packaged build; the
//! application still has to call `mount_agent_control`. `evaluate` is a
//! second opt-in (`agent-control-evaluate`), off by default. The plugin adds
//! no authority: it reaches app behavior only through the existing command
//! and IPC boundaries (contracts 006, 010).

#[cfg(feature = "agent-control")]
mod bridge;
#[cfg(all(feature = "agent-control", target_os = "macos"))]
mod capture;
#[cfg(feature = "agent-control")]
mod commands;
#[cfg(feature = "agent-control")]
mod handler;
#[cfg(feature = "agent-control")]
mod mount;
#[cfg(feature = "agent-control")]
mod shim;

#[cfg(feature = "agent-control")]
pub use bridge::{CommandBridge, NoCommandBridge};
// Glob so this file never names `longhorn_agent_control_*`. rustc records
// cfg'd-out pub-use idents in the feature-off rlib, and the release-absence
// scan treats that substring as a core-crate leak.
#[cfg(feature = "agent-control")]
pub use commands::*;
#[cfg(feature = "agent-control")]
pub use handler::TauriControlHandler;
#[cfg(feature = "agent-control")]
pub use longhorn_agent_control::ToolError;
#[cfg(feature = "agent-control")]
pub use mount::{
    AgentControlConfig, AgentControlHandle, AgentControlMountError, AgentControlShutdownError,
    mount_agent_control,
};
