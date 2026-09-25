//! Tauri command that begins an agent-originated picker and waits for MCP
//! settlement. Human-originated calls never reach this command.

use longhorn_agent_control::{BeginSelectionRequest, SelectionRegistry, plugin_result};
use serde::Deserialize;
use tauri::{Runtime, State, Webview};

/// Renderer invoke name. The host-protocol proof matches this identifier
/// on both the Rust command and the TypeScript constant.
pub const BEGIN_SELECTION_COMMAND: &str = "longhorn_agent_control_begin_selection";

/// Invoke payload for [`longhorn_agent_control_begin_selection`].
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BeginSelectionArgs {
    kind: longhorn_agent_control::SelectionKind,
    #[serde(default)]
    directory: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    filters: Vec<longhorn_agent_control::SelectionFilter>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    default_path: Option<String>,
}

/// Starts a pending selection for the invoking webview and returns the
/// plugin-compatible picker result once an agent answers or rejects.
///
/// Expiry, cancellation, and shutdown fail typed. There is no native-dialog
/// fallback on this path.
#[tauri::command]
pub async fn longhorn_agent_control_begin_selection<R: Runtime>(
    webview: Webview<R>,
    registry: State<'_, SelectionRegistry>,
    options: BeginSelectionArgs,
) -> Result<serde_json::Value, String> {
    let webview_label = webview.label().to_owned();
    let window_label = webview.window().label().to_owned();
    let multiple = options.multiple;
    let wait = registry
        .begin(BeginSelectionRequest {
            kind: options.kind,
            directory: options.directory,
            multiple,
            filters: options.filters,
            title: options.title,
            default_path: options.default_path,
            window: Some(window_label),
            webview: Some(webview_label),
        })
        .map_err(|error| error.to_string())?;
    let id = wait.id().clone();
    let outcome = wait.await;
    plugin_result(&id, multiple, &outcome)
        .map_err(|error| serde_json::to_string(&error).unwrap_or_else(|_| error.to_string()))
}
