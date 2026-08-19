//! The host-supplied bridge from the `command` tool into the app's
//! contract-006 command registry.
//!
//! The plugin deliberately holds no command authority of its own: the app
//! implements [`CommandBridge`] over its own sealed registry and admission
//! engine, so a `command` tool call travels exactly the path a menu or
//! palette invocation would (contract 006), and the control surface adds no
//! new route to behavior (contract 022).

use longhorn_agent_control::ToolError;
use longhorn_core::CommandId;
use serde_json::Value;

/// Routes one `command` tool invocation into the app's contract-006
/// registry and returns its output payload.
///
/// Implementations run inside the control server thread; they must be
/// non-blocking with respect to the control surface and must map every
/// admission or execution failure to a typed [`ToolError`] — never panic.
pub trait CommandBridge: Send + Sync + 'static {
    /// Invokes `command` with `argument` through the app's registry and
    /// returns the command's output payload, when it produces one.
    ///
    /// Unknown commands, stale registries, invalid arguments, and execution
    /// failures surface as [`ToolError::CommandFailed`] (or a more specific
    /// variant the host prefers); the core vocabulary carries them to the
    /// caller untransformed.
    fn invoke_command(
        &self,
        command: &CommandId,
        argument: Option<Value>,
    ) -> Result<Option<Value>, ToolError>;
}
