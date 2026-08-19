//! Tauri host wiring for the Longhorn agent app-control surface
//! (contract 022).
//!
//! This crate composes the host-agnostic `longhorn-agent-control` server
//! inside a running Tauri application: it mounts the stateless MCP server on
//! a background thread, implements [`ControlHandler`] against the app's
//! windows, and routes `command` invocations into the app's contract-006
//! registry through the host-supplied [`CommandBridge`].
//!
//! The entire surface sits behind the off-by-default `dev` cargo feature.
//! Gating is compile-time and total: a build without the feature compiles to
//! an empty library — no server, route, token, or discovery code exists in
//! the artifact, and no runtime toggle can enable it (contract 022;
//! `scripts/verify-agent-control-release-absence.ts` proves it). The plugin
//! adds no authority: it reaches app behavior only through the existing
//! command and IPC boundaries (contracts 006, 010).

#[cfg(feature = "dev")]
mod bridge;
#[cfg(all(feature = "dev", target_os = "macos"))]
mod capture;
#[cfg(feature = "dev")]
mod handler;
#[cfg(feature = "dev")]
mod mount;

#[cfg(feature = "dev")]
pub use bridge::CommandBridge;
#[cfg(feature = "dev")]
pub use handler::TauriControlHandler;
#[cfg(feature = "dev")]
pub use longhorn_agent_control::ToolError;
#[cfg(feature = "dev")]
pub use mount::{
    AgentControlConfig, AgentControlHandle, AgentControlMountError, AgentControlShutdownError,
    mount_agent_control,
};
