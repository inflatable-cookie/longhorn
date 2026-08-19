//! The Tauri [`ControlHandler`]: window scope against the app's webview
//! windows, `command` through the host's [`CommandBridge`], `screenshot`
//! and the `evaluate` escape hatch through the macOS webview capture bridge
//! (Card 231), and typed `Unsupported` for the tools the g02.032 shim wires
//! (snapshot, input dispatch, `wait_for`). Non-macOS hosts compile and
//! answer typed `Unsupported` for capture and evaluate — the per-backend
//! evidence discipline of contract 020.
//!
//! Window identity is the Tauri window label: it is already the app's own
//! stable window name, so the control surface introduces no parallel
//! naming. Per-window targeting resolves a label to its live window at call
//! time; a request with no target addresses the focused window, falling
//! back to the first window by label when the app holds no focus — the
//! steady state while an agent drives the app unfocused.

use std::sync::Arc;

use longhorn_agent_control::{
    ActionReceipt, ClickRequest, CommandRequest, CommandResult, ControlHandler, DragRequest,
    EvaluateRequest, EvaluateResult, ListWindowsRequest, ListWindowsResult, PressRequest,
    ResizeWindowRequest, ScreenshotRequest, ScreenshotResult, ScrollRequest, SnapshotRequest,
    SnapshotResult, ToolError, TypeRequest, WaitForRequest, WaitForResult, WindowInfo,
    WindowTarget,
};
use longhorn_core::{ClientSize, WindowId};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

use crate::bridge::CommandBridge;

/// Host authority for the control surface, backed by the Tauri app handle.
pub struct TauriControlHandler<R: Runtime> {
    app: AppHandle<R>,
    commands: Arc<dyn CommandBridge>,
}

impl<R: Runtime> TauriControlHandler<R> {
    /// Creates the handler over `app`, routing `command` through `commands`.
    pub fn new(app: AppHandle<R>, commands: Arc<dyn CommandBridge>) -> Self {
        Self { app, commands }
    }

    /// Every webview window, sorted by label for deterministic targeting.
    fn windows(&self) -> Vec<(String, WebviewWindow<R>)> {
        let mut windows: Vec<_> = self.app.webview_windows().into_iter().collect();
        windows.sort_by(|(left, _), (right, _)| left.cmp(right));
        windows
    }

    /// Resolves a request's window target to a live window.
    fn resolve_window(
        &self,
        target: &WindowTarget,
    ) -> Result<(WindowId, WebviewWindow<R>), ToolError> {
        match target {
            Some(window) => {
                let label = window.as_str();
                self.app
                    .get_webview_window(label)
                    .map(|live| (window.clone(), live))
                    .ok_or_else(|| ToolError::UnknownWindow {
                        window: window.clone(),
                    })
            }
            None => {
                let windows = self.windows();
                let (label, live) = windows
                    .iter()
                    .find(|(_, window)| window.is_focused().unwrap_or(false))
                    .or_else(|| windows.first())
                    .ok_or_else(|| ToolError::Unsupported {
                        message: "the app has no webview windows to target".to_owned(),
                    })?;
                let window =
                    WindowId::new(label.clone()).map_err(|error| ToolError::Unsupported {
                        message: format!(
                            "window label {label:?} is not a valid window id: {error}"
                        ),
                    })?;
                Ok((window, live.clone()))
            }
        }
    }

    /// Reads one window's [`WindowInfo`].
    fn window_info(label: &str, window: &WebviewWindow<R>) -> Result<WindowInfo, ToolError> {
        let id = WindowId::new(label.to_owned()).map_err(|error| ToolError::Unsupported {
            message: format!("window label {label:?} is not a valid window id: {error}"),
        })?;
        let title = window.title().map_err(|error| ToolError::Unsupported {
            message: format!("window {label:?} title read failed: {error}"),
        })?;
        let scale = window
            .scale_factor()
            .map_err(|error| ToolError::Unsupported {
                message: format!("window {label:?} scale factor read failed: {error}"),
            })?;
        let physical = window
            .inner_size()
            .map_err(|error| ToolError::Unsupported {
                message: format!("window {label:?} size read failed: {error}"),
            })?;
        let logical = physical.to_logical::<f64>(scale);
        let size = ClientSize::new(logical.width, logical.height).map_err(|error| {
            ToolError::Unsupported {
                message: format!("window {label:?} reported an invalid client size: {error}"),
            }
        })?;
        Ok(WindowInfo {
            window: id,
            title,
            size,
            focused: window.is_focused().unwrap_or(false),
        })
    }
}

/// Typed `Unsupported` for a tool a later lane wires.
fn unwired(tool: &str) -> ToolError {
    ToolError::Unsupported {
        message: format!("{tool} is not wired in the Tauri host yet (g02.032)"),
    }
}

impl<R: Runtime> ControlHandler for TauriControlHandler<R> {
    async fn snapshot(&self, _request: SnapshotRequest) -> Result<SnapshotResult, ToolError> {
        Err(unwired("snapshot"))
    }

    async fn click(&self, _request: ClickRequest) -> Result<ActionReceipt, ToolError> {
        Err(unwired("click"))
    }

    async fn r#type(&self, _request: TypeRequest) -> Result<ActionReceipt, ToolError> {
        Err(unwired("type"))
    }

    async fn press(&self, _request: PressRequest) -> Result<ActionReceipt, ToolError> {
        Err(unwired("press"))
    }

    async fn scroll(&self, _request: ScrollRequest) -> Result<ActionReceipt, ToolError> {
        Err(unwired("scroll"))
    }

    async fn drag(&self, _request: DragRequest) -> Result<ActionReceipt, ToolError> {
        Err(unwired("drag"))
    }

    async fn evaluate(&self, request: EvaluateRequest) -> Result<EvaluateResult, ToolError> {
        let (_, window) = self.resolve_window(&request.window)?;
        #[cfg(target_os = "macos")]
        {
            let value = crate::capture::evaluate_webview(&window, request.js).await?;
            Ok(EvaluateResult { value })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (window, request);
            Err(ToolError::Unsupported {
                message: "evaluate is implemented on macOS only (contract 020)".to_owned(),
            })
        }
    }

    async fn wait_for(&self, _request: WaitForRequest) -> Result<WaitForResult, ToolError> {
        Err(unwired("wait_for"))
    }

    async fn screenshot(&self, request: ScreenshotRequest) -> Result<ScreenshotResult, ToolError> {
        let (window, live) = self.resolve_window(&request.window)?;
        #[cfg(target_os = "macos")]
        {
            let png = crate::capture::screenshot_webview(&live).await?;
            Ok(ScreenshotResult { window, png })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = live;
            Err(ToolError::Unsupported {
                message: "screenshot capture is implemented on macOS only (contract 020)"
                    .to_owned(),
            })
        }
    }

    async fn command(&self, request: CommandRequest) -> Result<CommandResult, ToolError> {
        let output = self
            .commands
            .invoke_command(&request.command, request.argument)?;
        Ok(CommandResult { output })
    }

    async fn list_windows(
        &self,
        _request: ListWindowsRequest,
    ) -> Result<ListWindowsResult, ToolError> {
        let windows = self
            .windows()
            .iter()
            .map(|(label, window)| Self::window_info(label, window))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ListWindowsResult { windows })
    }

    async fn resize_window(
        &self,
        request: ResizeWindowRequest,
    ) -> Result<ActionReceipt, ToolError> {
        let (_, window) = self.resolve_window(&Some(request.window.clone()))?;
        window
            .set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                request.size.width().get(),
                request.size.height().get(),
            )))
            .map_err(|error| ToolError::Unsupported {
                message: format!("window {:?} resize failed: {error}", request.window),
            })?;
        Ok(ActionReceipt {})
    }
}
