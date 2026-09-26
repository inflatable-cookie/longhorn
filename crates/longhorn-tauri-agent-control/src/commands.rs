//! JS begin-selection command and the Rust-side host picker entry.
//!
//! Agent-originated JS `open`/`save` never reach a native panel: they invoke
//! [`longhorn_agent_control_begin_selection`]. Consumer Rust commands that
//! currently call the dialog plugin use [`begin_host_selection`] with a
//! page-carried origin token. Human, missing, or malformed origin returns
//! [`HostSelection::UsePlugin`] and publishes nothing. There is no native
//! fallback on the agent path.

use longhorn_agent_control::{BeginSelectionRequest, SelectionRegistry, plugin_result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use tauri::{Runtime, State, Webview};

/// Renderer invoke name. The host-protocol proof matches this identifier
/// on both the Rust command and the TypeScript constant.
pub const BEGIN_SELECTION_COMMAND: &str = "longhorn_agent_control_begin_selection";

/// Scan token for the Rust-side host picker. The release-absence scan
/// requires this exact string in the feature-on plugin rlib and forbids it
/// when the feature is off.
pub const HOST_SELECTION_ENTRY: &str = "begin_host_selection";

/// Wire value of [`SelectionOrigin::Agent`]. Matches the TypeScript export.
pub const SELECTION_ORIGIN_AGENT: &str = "agent";

/// Wire value of [`SelectionOrigin::Human`]. Matches the TypeScript export.
pub const SELECTION_ORIGIN_HUMAN: &str = "human";

/// Page origin the consumer carries into its own command arguments.
///
/// Only the exact JSON string `"agent"` is agent. `"human"`, missing,
/// `null`, and every other JSON value are human (contract 022).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SelectionOrigin {
    /// Trusted input, no shim, missing token, or malformed token.
    #[default]
    Human,
    /// The page shim reported agent origin at invoke time.
    Agent,
}

impl SelectionOrigin {
    /// `true` only for [`SelectionOrigin::Agent`].
    #[must_use]
    pub const fn is_agent(self) -> bool {
        matches!(self, Self::Agent)
    }

    /// Decode a consumer-carried JSON value.
    #[must_use]
    pub fn from_json(value: &Value) -> Self {
        if value.as_str() == Some(SELECTION_ORIGIN_AGENT) {
            Self::Agent
        } else {
            Self::Human
        }
    }

    /// Missing JSON is human.
    #[must_use]
    pub fn from_optional_json(value: Option<&Value>) -> Self {
        value.map(Self::from_json).unwrap_or(Self::Human)
    }
}

impl Serialize for SelectionOrigin {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Agent => SELECTION_ORIGIN_AGENT,
            Self::Human => SELECTION_ORIGIN_HUMAN,
        })
    }
}

impl<'de> Deserialize<'de> for SelectionOrigin {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_json(&Value::deserialize(deserializer)?))
    }
}

/// Invoke payload for [`longhorn_agent_control_begin_selection`] and the
/// picker request [`begin_host_selection`] takes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BeginSelectionArgs {
    /// Open or save.
    pub kind: longhorn_agent_control::SelectionKind,
    /// Directory intent; only meaningful for open.
    #[serde(default)]
    pub directory: bool,
    /// Multiple paths; only meaningful for open.
    #[serde(default)]
    pub multiple: bool,
    /// Optional extension filters. Hints for the agent; not enforced.
    #[serde(default)]
    pub filters: Vec<longhorn_agent_control::SelectionFilter>,
    /// Dialog title, when the caller supplied one.
    #[serde(default)]
    pub title: Option<String>,
    /// Default-path hint, never an answer.
    #[serde(default)]
    pub default_path: Option<String>,
}

/// Outcome of [`begin_host_selection`].
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub enum HostSelection {
    /// Origin is human, missing, or malformed. Call the dialog plugin with
    /// the original options. Nothing was published.
    UsePlugin,
    /// Agent-originated request settled to the plugin-compatible result.
    ///
    /// `Ok` is one path, a path array, or `null` (explicit reject). `Err` is
    /// a serialized [`longhorn_agent_control::ToolError`] for expiry,
    /// shutdown, cancellation, or begin failure. There is no native-dialog
    /// fallback on this arm.
    Agent(Result<Value, String>),
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
) -> Result<Value, String> {
    settle_selection(&registry, request_from_webview(&webview, options)).await
}

/// Routes a consumer Rust picker through the pending-selection registry.
///
/// Agent origin begins a request bound to `webview`'s window and webview
/// labels and waits for MCP settlement with the same result shape as the
/// JS route. Human, missing, or malformed origin returns
/// [`HostSelection::UsePlugin`] without touching the registry. The caller
/// opens the dialog plugin only on that arm.
pub async fn begin_host_selection<R: Runtime>(
    origin: SelectionOrigin,
    webview: &Webview<R>,
    registry: &SelectionRegistry,
    options: BeginSelectionArgs,
) -> HostSelection {
    if !origin.is_agent() {
        return HostSelection::UsePlugin;
    }
    HostSelection::Agent(settle_selection(registry, request_from_webview(webview, options)).await)
}

fn request_from_webview<R: Runtime>(
    webview: &Webview<R>,
    options: BeginSelectionArgs,
) -> BeginSelectionRequest {
    BeginSelectionRequest {
        kind: options.kind,
        directory: options.directory,
        multiple: options.multiple,
        filters: options.filters,
        title: options.title,
        default_path: options.default_path,
        window: Some(webview.window().label().to_owned()),
        webview: Some(webview.label().to_owned()),
    }
}

async fn settle_selection(
    registry: &SelectionRegistry,
    request: BeginSelectionRequest,
) -> Result<Value, String> {
    let multiple = request.multiple;
    let wait = registry.begin(request).map_err(|error| error.to_string())?;
    let id = wait.id().clone();
    let outcome = wait.await;
    plugin_result(&id, multiple, &outcome)
        .map_err(|error| serde_json::to_string(&error).unwrap_or_else(|_| error.to_string()))
}

#[cfg(test)]
mod origin_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_agent_string_is_agent() {
        assert_eq!(
            SelectionOrigin::from_json(&json!("agent")),
            SelectionOrigin::Agent
        );
        assert!(SelectionOrigin::from_json(&json!("agent")).is_agent());
        assert_eq!(
            serde_json::from_value::<SelectionOrigin>(json!("agent")).unwrap(),
            SelectionOrigin::Agent
        );
    }

    #[test]
    fn human_missing_and_malformed_are_human() {
        for value in [
            json!("human"),
            json!("Agent"),
            json!(null),
            json!(true),
            json!(1),
            json!({"origin": "agent"}),
            json!(["agent"]),
        ] {
            assert_eq!(
                SelectionOrigin::from_json(&value),
                SelectionOrigin::Human,
                "{value}"
            );
        }
        assert_eq!(
            SelectionOrigin::from_optional_json(None),
            SelectionOrigin::Human
        );
        assert!(!SelectionOrigin::Human.is_agent());
        assert_eq!(SelectionOrigin::default(), SelectionOrigin::Human);
    }

    #[test]
    fn origin_round_trips_as_the_typescript_tokens() {
        assert_eq!(
            serde_json::to_value(SelectionOrigin::Agent).unwrap(),
            json!("agent")
        );
        assert_eq!(
            serde_json::to_value(SelectionOrigin::Human).unwrap(),
            json!("human")
        );
        assert_eq!(SELECTION_ORIGIN_AGENT, "agent");
        assert_eq!(SELECTION_ORIGIN_HUMAN, "human");
        assert_eq!(HOST_SELECTION_ENTRY, "begin_host_selection");
    }
}
