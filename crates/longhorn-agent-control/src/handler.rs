//! The host-implemented authority behind the control surface.
//!
//! The stateless server (Card 229) dispatches every tool call to this
//! trait. The host owns all webview, window, and command mechanics; the
//! core owns the wire. `evaluate` and `command` are full code execution in
//! the app — implementations must treat the bearer token as the entire
//! trust boundary, because it is.

use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, ClickRequest, CommandRequest, CommandResult, DragRequest, EvaluateRequest,
    EvaluateResult, ListWindowsRequest, ListWindowsResult, PressRequest, ResizeWindowRequest,
    ScreenshotRequest, ScreenshotResult, ScrollRequest, SetFileInputRequest, SnapshotRequest,
    SnapshotResult, ToolError, TypeRequest, ValidatedSetFileInputRequest, WaitForRequest,
    WaitForResult,
};

/// One batch from the current document's page-event ring.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PageEventBatch {
    /// Identity of the document whose ring produced this batch.
    pub epoch: String,
    /// Events after the requested cursor, or all retained events after a reload.
    pub events: Vec<PageEvent>,
    /// Sequence number the ring will assign to the next event.
    pub next_seq: u64,
    /// Count of events evicted from this document's ring.
    pub dropped: u64,
}

/// One entry in the page-event ring.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PageEvent {
    /// A captured page console message.
    Console {
        /// Sequence of the message within the document.
        seq: u64,
        /// Console level reported by the page.
        level: String,
        /// Text logged by the page.
        text: String,
    },
    /// An unhandled page error or rejected promise.
    Error {
        /// Sequence of the error within the document.
        seq: u64,
        /// Error message reported by the page.
        message: String,
    },
    /// Initial document load or a URL change.
    Navigation {
        /// Sequence of the navigation within the document.
        seq: u64,
        /// New page URL.
        url: String,
    },
}

impl PageEvent {
    pub(crate) const fn sequence(&self) -> u64 {
        match self {
            Self::Console { seq, .. } | Self::Error { seq, .. } | Self::Navigation { seq, .. } => {
                *seq
            }
        }
    }
}

/// Tool authority the host implements and the MCP server dispatches to.
///
/// Stateless by construction: every method takes a self-contained request,
/// and no call may rely on an earlier one. Refs resolve against the live
/// DOM at call time; a stale ref fails explicitly with
/// [`ToolError::UnresolvedRef`].
pub trait ControlHandler: Send + Sync + 'static {
    /// Semantic element tree of the webview, refs stamped into the live DOM.
    fn snapshot(
        &self,
        request: SnapshotRequest,
    ) -> impl Future<Output = Result<SnapshotResult, ToolError>> + Send;

    /// Synthetic in-page click on a resolved ref.
    fn click(
        &self,
        request: ClickRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send;

    /// Synthetic in-page text entry into a resolved ref.
    fn r#type(
        &self,
        request: TypeRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send;

    /// Synthetic in-page key press.
    fn press(
        &self,
        request: PressRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send;

    /// Synthetic in-page scroll.
    fn scroll(
        &self,
        request: ScrollRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send;

    /// Synthetic in-page drag between resolved refs. Untrusted DOM events
    /// only; there is no OS-level mode.
    fn drag(
        &self,
        request: DragRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send;

    /// Supply inline file content to an HTML `<input type="file">`.
    ///
    /// The agent is the byte source: the request carries name, optional
    /// media type, and base64 content. There is no path argument. Hosts
    /// call [`SetFileInputRequest::validate`] before any page effect.
    fn set_file_input(
        &self,
        request: SetFileInputRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send;

    /// Dispatches a `set_file_input` request proven valid via
    /// [`SetFileInputRequest::into_validated`].
    ///
    /// The default implementation forwards to [`ControlHandler::set_file_input`];
    /// hosts that validate inside `set_file_input` override this to avoid
    /// decoding the payload twice on the MCP path while keeping unchecked
    /// requests impossible to construct.
    fn set_file_input_prevalidated(
        &self,
        request: ValidatedSetFileInputRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send {
        self.set_file_input(request.into_inner())
    }

    /// Runs JavaScript in the page. Escape hatch, not the primary path.
    fn evaluate(
        &self,
        request: EvaluateRequest,
    ) -> impl Future<Output = Result<EvaluateResult, ToolError>> + Send;

    /// Reads the default page's event ring without changing picker origin.
    ///
    /// A different document epoch resets the read cursor to zero. This
    /// host-internal read is available independently of MCP `evaluate`.
    fn read_page_events(
        &self,
        since_seq: u64,
        since_epoch: Option<&str>,
    ) -> impl Future<Output = Result<PageEventBatch, ToolError>> + Send;

    /// Polls a DOM-relative predicate until it holds or the bound ends.
    fn wait_for(
        &self,
        request: WaitForRequest,
    ) -> impl Future<Output = Result<WaitForResult, ToolError>> + Send;

    /// Fresh window image via webview snapshot capture; works occluded,
    /// unfocused, and minimized.
    fn screenshot(
        &self,
        request: ScreenshotRequest,
    ) -> impl Future<Output = Result<ScreenshotResult, ToolError>> + Send;

    /// Invokes a registered contract-006 command by id — the route to
    /// behavior behind native menus and dialogs.
    fn command(
        &self,
        request: CommandRequest,
    ) -> impl Future<Output = Result<CommandResult, ToolError>> + Send;

    /// Lists every window the host exposes to the control surface.
    fn list_windows(
        &self,
        request: ListWindowsRequest,
    ) -> impl Future<Output = Result<ListWindowsResult, ToolError>> + Send;

    /// Resizes one window's content area.
    fn resize_window(
        &self,
        request: ResizeWindowRequest,
    ) -> impl Future<Output = Result<ActionReceipt, ToolError>> + Send;
}
