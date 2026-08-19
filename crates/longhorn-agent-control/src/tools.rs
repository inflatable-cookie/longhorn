//! Contract 022 tool vocabulary: requests, results, and errors for the
//! agent control surface.
//!
//! These types are the wire-independent core every host implements against.
//! Element refs are opaque strings stamped into the live DOM at snapshot
//! time and resolved against the live DOM on use — the core holds no ref
//! table, so a ref from any prior snapshot either resolves or fails
//! explicitly. `wait_for` predicates are DOM-relative only: no variant can
//! express a time-only or animation-frame wait, because WKWebView coalesces
//! DOM timers in every window state and stops `requestAnimationFrame`
//! entirely while the window is not key (contract 022, Card 227).

use std::{collections::BTreeSet, error::Error, fmt, str::FromStr};

use longhorn_core::{ClientSize, CommandId, OpaqueIdError, WindowId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// Opaque element reference stamped by the edge that produced a snapshot.
///
/// Resolution is the stamping edge's job against the live DOM; the core
/// treats the value as an opaque, bounded string and never stores it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementRef(String);

impl ElementRef {
    /// Validates and constructs the reference.
    pub fn new(value: impl Into<String>) -> Result<Self, OpaqueIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(OpaqueIdError::Empty);
        }
        if value.len() > longhorn_core::MAX_OPAQUE_ID_BYTES {
            return Err(OpaqueIdError::TooLong {
                maximum: longhorn_core::MAX_OPAQUE_ID_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the serialized reference.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ElementRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ElementRef {
    type Err = OpaqueIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl Serialize for ElementRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ElementRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// One node of the semantic element tree: role, name, value, state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SemanticNode {
    /// Reference stamped into the live DOM for this element.
    pub element_ref: ElementRef,
    /// Accessibility-style role, e.g. `button`, `textbox`.
    pub role: String,
    /// Accessible name, when the element has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Current value, when the role carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// State tokens, e.g. `disabled`, `checked`, `expanded`.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub states: BTreeSet<String>,
    /// Child elements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SemanticNode>,
}

/// Page-level state accompanying a snapshot or wait result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PageState {
    /// Current document URL.
    pub url: String,
    /// Current document title.
    pub title: String,
}

/// Per-window targeting shared by every windowed request: `None` addresses
/// the host's frontmost or only window.
pub type WindowTarget = Option<WindowId>;

/// `snapshot` request.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SnapshotRequest {
    /// Window to snapshot; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
}

/// `snapshot` result: the semantic tree with refs stamped into the live DOM.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SnapshotResult {
    /// Window the snapshot was taken from.
    pub window: WindowId,
    /// Page state at snapshot time.
    pub page: PageState,
    /// Root of the semantic element tree.
    pub root: SemanticNode,
}

/// `click` request: synthetic in-page click on a resolved ref.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ClickRequest {
    /// Window containing the element; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
    /// Element to click.
    pub element: ElementRef,
}

/// `type` request: synthetic in-page text entry into a resolved ref.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TypeRequest {
    /// Window containing the element; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
    /// Element to focus and type into.
    pub element: ElementRef,
    /// Text to enter.
    pub text: String,
}

/// Keyboard modifier held during a `press`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyModifier {
    /// Alt / Option.
    Alt,
    /// Control.
    Control,
    /// Meta / Command / Windows key.
    Meta,
    /// Shift.
    Shift,
}

/// `press` request: synthetic in-page key press.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PressRequest {
    /// Window containing the element; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
    /// Element to dispatch against; `None` targets the focused element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementRef>,
    /// Key name, e.g. `Enter`, `Escape`, `a`.
    pub key: String,
    /// Modifiers held during the press.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub modifiers: BTreeSet<KeyModifier>,
}

/// `scroll` request: synthetic in-page scroll.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScrollRequest {
    /// Window containing the element; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
    /// Element to scroll; `None` scrolls the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementRef>,
    /// Horizontal delta in logical pixels.
    pub delta_x: i32,
    /// Vertical delta in logical pixels.
    pub delta_y: i32,
}

/// `drag` request: synthetic in-page drag between two resolved refs.
///
/// There is deliberately no OS-level mode: drag dispatches untrusted DOM
/// events in-page, and native hover, OS drag-and-drop, and `isTrusted`
/// checks are out of scope (contract 022). No field can ever select one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DragRequest {
    /// Window containing the elements; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
    /// Element the drag starts on.
    pub source: ElementRef,
    /// Element the drag ends on.
    pub target: ElementRef,
}

/// Receipt for the action family (`click`, `type`, `press`, `scroll`,
/// `drag`, `resize_window`): the event was dispatched; nothing about its
/// effect is implied — observe with `snapshot` or `wait_for`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActionReceipt {}

/// `evaluate` request: run JavaScript in the page. Escape hatch, not the
/// primary path — full code execution inside the app.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EvaluateRequest {
    /// Window to evaluate in; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
    /// JavaScript source evaluated in the page's main world.
    pub js: String,
}

/// `evaluate` result.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EvaluateResult {
    /// JSON value the expression produced.
    pub value: serde_json::Value,
}

/// `wait_for` predicate over the semantic tree or page state.
///
/// Every variant is DOM-relative. There is no duration-only or
/// animation-frame variant and there never will be: elapsed time proves
/// nothing about page progress under WKWebView timer coalescing, and
/// rAF-driven visuals must not be awaited (contract 022).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "predicate",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WaitPredicate {
    /// Holds when the ref resolves against the live DOM.
    RefResolve {
        /// Element expected to resolve.
        element: ElementRef,
    },
    /// Holds when the ref no longer resolves against the live DOM.
    RefAbsent {
        /// Element expected to be gone.
        element: ElementRef,
    },
    /// Holds when the page URL contains the needle.
    PageUrlContains {
        /// Substring expected in the URL.
        needle: String,
    },
    /// Holds when the page title contains the needle.
    PageTitleContains {
        /// Substring expected in the title.
        needle: String,
    },
}

/// `wait_for` request: poll a predicate until it holds or the bound ends.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WaitForRequest {
    /// Window to wait in; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
    /// DOM-relative predicate to await.
    pub predicate: WaitPredicate,
    /// Hard bound in milliseconds; expiry is [`ToolError::WaitTimeout`].
    pub timeout_ms: u32,
}

/// `wait_for` result: the predicate held within the bound.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WaitForResult {}

/// `screenshot` request: fresh window image via webview snapshot capture.
/// Works occluded, unfocused, and minimized (Card 227).
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScreenshotRequest {
    /// Window to capture; `None` targets the frontmost window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: WindowTarget,
}

/// `screenshot` result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScreenshotResult {
    /// Window that was captured.
    pub window: WindowId,
    /// PNG image bytes. The MCP server maps these to base64 image content
    /// at the wire edge; the core vocabulary stays transport-neutral.
    pub png: Vec<u8>,
}

/// `command` request: invoke a registered contract-006 command by id. This
/// is the route to behavior behind native menus and dialogs; agents do not
/// click native chrome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandRequest {
    /// Command to invoke.
    pub command: CommandId,
    /// Command argument payload, when the command takes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argument: Option<serde_json::Value>,
}

/// `command` result.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandResult {
    /// Command output payload, when the command produces one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

/// `list_windows` request.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ListWindowsRequest {}

/// One window known to the host.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WindowInfo {
    /// Window identity used for per-window targeting.
    pub window: WindowId,
    /// Window title.
    pub title: String,
    /// Content size in logical pixels.
    pub size: ClientSize,
    /// Whether the window currently holds focus.
    pub focused: bool,
}

/// `list_windows` result.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ListWindowsResult {
    /// Every window the host exposes to the control surface.
    pub windows: Vec<WindowInfo>,
}

/// `resize_window` request.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResizeWindowRequest {
    /// Window to resize.
    pub window: WindowId,
    /// New content size in logical pixels.
    pub size: ClientSize,
}

/// Typed failure for every tool in the surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "error",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ToolError {
    /// The ref did not resolve against the live DOM — unknown or stale.
    /// Staleness fails explicitly; there is no server-side table to consult.
    UnresolvedRef {
        /// Ref that failed to resolve.
        element: ElementRef,
    },
    /// The targeted window does not exist.
    UnknownWindow {
        /// Window that was not found.
        window: WindowId,
    },
    /// The `wait_for` bound elapsed before the predicate held.
    WaitTimeout {
        /// Bound that elapsed, in milliseconds.
        timeout_ms: u32,
    },
    /// `evaluate` source raised or failed to serialize.
    EvaluationFailed {
        /// Host-provided failure detail.
        message: String,
    },
    /// A contract-006 command rejected or failed the invocation.
    CommandFailed {
        /// Command that failed.
        command: CommandId,
        /// Host-provided failure detail.
        message: String,
    },
    /// The request reached a surface that cannot serve it — e.g. a native
    /// surface asked for more than screenshots with no provider registered.
    Unsupported {
        /// What cannot be served, and why.
        message: String,
    },
}

impl fmt::Display for ToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnresolvedRef { element } => {
                write!(
                    formatter,
                    "element ref {element:?} does not resolve against the live DOM"
                )
            }
            Self::UnknownWindow { window } => {
                write!(formatter, "window {window:?} does not exist")
            }
            Self::WaitTimeout { timeout_ms } => {
                write!(formatter, "predicate did not hold within {timeout_ms} ms")
            }
            Self::EvaluationFailed { message } => {
                write!(formatter, "evaluate failed: {message}")
            }
            Self::CommandFailed { command, message } => {
                write!(formatter, "command {command:?} failed: {message}")
            }
            Self::Unsupported { message } => {
                write!(formatter, "unsupported: {message}")
            }
        }
    }
}

impl Error for ToolError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ref_a() -> ElementRef {
        ElementRef::new("e1").unwrap()
    }

    #[test]
    fn element_refs_are_bounded_opaque_strings() {
        assert_eq!(ElementRef::new("").unwrap_err(), OpaqueIdError::Empty);
        let long = "x".repeat(longhorn_core::MAX_OPAQUE_ID_BYTES + 1);
        assert!(matches!(
            ElementRef::new(long).unwrap_err(),
            OpaqueIdError::TooLong { .. }
        ));
    }

    #[test]
    fn wait_predicates_are_dom_relative_only() {
        // The variant set is the enforcement point: serde admits exactly the
        // DOM-relative predicates and nothing time- or animation-based.
        let predicate = WaitPredicate::RefResolve { element: ref_a() };
        let json = serde_json::to_string(&predicate).unwrap();
        assert_eq!(json, r#"{"predicate":"refResolve","element":"e1"}"#);
        assert_eq!(
            serde_json::from_str::<WaitPredicate>(&json).unwrap(),
            predicate
        );

        for rejected in [
            r#"{"predicate":"duration","ms":100}"#,
            r#"{"predicate":"animationFrame"}"#,
            r#"{"predicate":"timeout","ms":100}"#,
        ] {
            assert!(serde_json::from_str::<WaitPredicate>(rejected).is_err());
        }
    }

    #[test]
    fn drag_has_no_os_level_mode() {
        let drag = DragRequest {
            window: None,
            source: ref_a(),
            target: ElementRef::new("e2").unwrap(),
        };
        let json = serde_json::to_string(&drag).unwrap();
        assert_eq!(json, r#"{"source":"e1","target":"e2"}"#);
        // An `isTrusted`/OS-level field is not admitted on the wire.
        assert!(
            serde_json::from_str::<DragRequest>(r#"{"source":"e1","target":"e2","trusted":true}"#)
                .is_err()
        );
    }

    #[test]
    fn tool_errors_round_trip() {
        let error = ToolError::WaitTimeout { timeout_ms: 250 };
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(serde_json::from_str::<ToolError>(&json).unwrap(), error);
    }
}
