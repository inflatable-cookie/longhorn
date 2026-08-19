//! The host-implemented authority behind the control surface.
//!
//! The stateless server (Card 229) dispatches every tool call to this
//! trait. The host owns all webview, window, and command mechanics; the
//! core owns the wire. `evaluate` and `command` are full code execution in
//! the app — implementations must treat the bearer token as the entire
//! trust boundary, because it is.

use std::future::Future;

use crate::{
    ActionReceipt, ClickRequest, CommandRequest, CommandResult, DragRequest, EvaluateRequest,
    EvaluateResult, ListWindowsRequest, ListWindowsResult, PressRequest, ResizeWindowRequest,
    ScreenshotRequest, ScreenshotResult, ScrollRequest, SnapshotRequest, SnapshotResult, ToolError,
    TypeRequest, WaitForRequest, WaitForResult,
};

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

    /// Runs JavaScript in the page. Escape hatch, not the primary path.
    fn evaluate(
        &self,
        request: EvaluateRequest,
    ) -> impl Future<Output = Result<EvaluateResult, ToolError>> + Send;

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
