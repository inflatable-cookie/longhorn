//! Wire argument shapes for the MCP tools.
//!
//! These mirror the Card 228 vocabulary but stay flat and self-describing:
//! every identifier crosses as a plain string so the generated JSON schema
//! is exact, and conversion into the vocabulary validates. The vocabulary
//! types themselves carry no schema dependency.

use std::collections::BTreeSet;

use longhorn_core::{ClientSize, CommandId, WindowId};
use rmcp::{ErrorData, schemars::JsonSchema};
use serde::Deserialize;

use crate::{
    ClickRequest, CommandRequest, DragRequest, EvaluateRequest, KeyModifier, PressRequest,
    ResizeWindowRequest, ScreenshotRequest, ScrollRequest, SnapshotRequest, TypeRequest,
    WaitForRequest, WaitPredicate,
};

/// Invalid wire input; surfaced as a JSON-RPC invalid-params error.
fn invalid_params(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(message.into(), None)
}

/// Parses an optional window target.
fn window_target(value: Option<String>) -> Result<Option<WindowId>, ErrorData> {
    value
        .map(|value| {
            WindowId::new(value.clone())
                .map_err(|_| invalid_params(format!("invalid window id {value:?}")))
        })
        .transpose()
}

/// Parses one element ref.
fn element_ref(value: &str) -> Result<crate::ElementRef, ErrorData> {
    crate::ElementRef::new(value)
        .map_err(|_| invalid_params(format!("invalid element ref {value:?}")))
}

/// `snapshot` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SnapshotArgs {
    /// Window id to snapshot; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
}

impl SnapshotArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<SnapshotRequest, ErrorData> {
        Ok(SnapshotRequest {
            window: window_target(self.window)?,
        })
    }
}

/// `click` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ClickArgs {
    /// Window id containing the element; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
    /// Element ref from a prior snapshot.
    pub element: String,
}

impl ClickArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<ClickRequest, ErrorData> {
        Ok(ClickRequest {
            window: window_target(self.window)?,
            element: element_ref(&self.element)?,
        })
    }
}

/// `type` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TypeArgs {
    /// Window id containing the element; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
    /// Element ref from a prior snapshot.
    pub element: String,
    /// Text to enter.
    pub text: String,
}

impl TypeArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<TypeRequest, ErrorData> {
        Ok(TypeRequest {
            window: window_target(self.window)?,
            element: element_ref(&self.element)?,
            text: self.text,
        })
    }
}

/// `press` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PressArgs {
    /// Window id containing the element; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
    /// Element ref to dispatch against; omit for the focused element.
    #[serde(default)]
    pub element: Option<String>,
    /// Key name, e.g. `Enter`, `Escape`, `a`.
    pub key: String,
    /// Modifiers held during the press: `alt`, `control`, `meta`, `shift`.
    #[serde(default)]
    pub modifiers: Vec<String>,
}

impl PressArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<PressRequest, ErrorData> {
        let modifiers = self
            .modifiers
            .iter()
            .map(|modifier| match modifier.as_str() {
                "alt" => Ok(KeyModifier::Alt),
                "control" => Ok(KeyModifier::Control),
                "meta" => Ok(KeyModifier::Meta),
                "shift" => Ok(KeyModifier::Shift),
                other => Err(invalid_params(format!("unknown key modifier {other:?}"))),
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        Ok(PressRequest {
            window: window_target(self.window)?,
            element: self.element.as_deref().map(element_ref).transpose()?,
            key: self.key,
            modifiers,
        })
    }
}

/// `scroll` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScrollArgs {
    /// Window id containing the element; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
    /// Element ref to scroll; omit to scroll the document.
    #[serde(default)]
    pub element: Option<String>,
    /// Horizontal delta in logical pixels.
    pub delta_x: i32,
    /// Vertical delta in logical pixels.
    pub delta_y: i32,
}

impl ScrollArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<ScrollRequest, ErrorData> {
        Ok(ScrollRequest {
            window: window_target(self.window)?,
            element: self.element.as_deref().map(element_ref).transpose()?,
            delta_x: self.delta_x,
            delta_y: self.delta_y,
        })
    }
}

/// `drag` arguments. Synthetic in-page events only; there is no OS-level
/// mode and no field can select one.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DragArgs {
    /// Window id containing the elements; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
    /// Element ref the drag starts on.
    pub source: String,
    /// Element ref the drag ends on.
    pub target: String,
}

impl DragArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<DragRequest, ErrorData> {
        Ok(DragRequest {
            window: window_target(self.window)?,
            source: element_ref(&self.source)?,
            target: element_ref(&self.target)?,
        })
    }
}

/// `evaluate` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EvaluateArgs {
    /// Window id to evaluate in; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
    /// JavaScript source evaluated in the page's main world.
    pub js: String,
}

impl EvaluateArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<EvaluateRequest, ErrorData> {
        Ok(EvaluateRequest {
            window: window_target(self.window)?,
            js: self.js,
        })
    }
}

/// `wait_for` predicate, wire shape. DOM-relative only: no duration-only
/// or animation-frame variant exists to select.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(
    tag = "predicate",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WaitPredicateArgs {
    /// Holds when the ref resolves against the live DOM.
    RefResolve {
        /// Element ref expected to resolve.
        element: String,
    },
    /// Holds when the ref no longer resolves against the live DOM.
    RefAbsent {
        /// Element ref expected to be gone.
        element: String,
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

/// `wait_for` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WaitForArgs {
    /// Window id to wait in; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
    /// DOM-relative predicate to await.
    pub predicate: WaitPredicateArgs,
    /// Hard bound in milliseconds.
    pub timeout_ms: u32,
}

impl WaitForArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<WaitForRequest, ErrorData> {
        let predicate = match self.predicate {
            WaitPredicateArgs::RefResolve { element } => WaitPredicate::RefResolve {
                element: element_ref(&element)?,
            },
            WaitPredicateArgs::RefAbsent { element } => WaitPredicate::RefAbsent {
                element: element_ref(&element)?,
            },
            WaitPredicateArgs::PageUrlContains { needle } => {
                WaitPredicate::PageUrlContains { needle }
            }
            WaitPredicateArgs::PageTitleContains { needle } => {
                WaitPredicate::PageTitleContains { needle }
            }
        };
        Ok(WaitForRequest {
            window: window_target(self.window)?,
            predicate,
            timeout_ms: self.timeout_ms,
        })
    }
}

/// `screenshot` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScreenshotArgs {
    /// Window id to capture; omit for the frontmost window.
    #[serde(default)]
    pub window: Option<String>,
}

impl ScreenshotArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<ScreenshotRequest, ErrorData> {
        Ok(ScreenshotRequest {
            window: window_target(self.window)?,
        })
    }
}

/// `command` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandArgs {
    /// Contract-006 command id to invoke.
    pub command: String,
    /// Command argument payload, when the command takes one.
    #[serde(default)]
    pub argument: Option<serde_json::Value>,
}

impl CommandArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<CommandRequest, ErrorData> {
        let command = CommandId::new(self.command.clone())
            .map_err(|_| invalid_params(format!("invalid command id {:?}", self.command)))?;
        Ok(CommandRequest {
            command,
            argument: self.argument,
        })
    }
}

/// `resize_window` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResizeWindowArgs {
    /// Window id to resize.
    pub window: String,
    /// New content width in logical pixels.
    pub width: f64,
    /// New content height in logical pixels.
    pub height: f64,
}

impl ResizeWindowArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<ResizeWindowRequest, ErrorData> {
        let window = WindowId::new(self.window.clone())
            .map_err(|_| invalid_params(format!("invalid window id {:?}", self.window)))?;
        let size = ClientSize::new(self.width, self.height)
            .map_err(|error| invalid_params(format!("invalid window size: {error}")))?;
        Ok(ResizeWindowRequest { window, size })
    }
}
