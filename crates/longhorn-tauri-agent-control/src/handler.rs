//! The Tauri [`ControlHandler`]: window scope against the app's webview
//! windows, `command` through the host's [`CommandBridge`], `screenshot`
//! and the `evaluate` escape hatch through the macOS webview capture bridge
//! (Card 231), and the Card 232 semantic tools (`snapshot`, input dispatch,
//! `wait_for`) marshalled through that same evaluate path. Non-macOS hosts
//! compile and answer typed `Unsupported` for capture, evaluate, and the
//! semantic tools — the per-backend evidence discipline of contract 020.
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
use tauri::{AppHandle, Manager, Runtime, Webview, Window};

use crate::{bridge::CommandBridge, shim};

/// Host authority for the control surface, backed by the Tauri app handle.
pub struct TauriControlHandler<R: Runtime> {
    app: AppHandle<R>,
    commands: Arc<dyn CommandBridge>,
}

/// One targetable window with its UI webview (the webview sharing the
/// window's label). Child webviews attached to the window are not semantic
/// targets; they stay reachable only through the native-surface seam.
struct LiveTarget<R: Runtime> {
    window: Window<R>,
    webview: Webview<R>,
}

impl<R: Runtime> Clone for LiveTarget<R> {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            webview: self.webview.clone(),
        }
    }
}

/// The window's UI webview: same label as the window, however many child
/// webviews are attached.
fn ui_webview<R: Runtime>(window: &Window<R>) -> Option<Webview<R>> {
    window
        .webviews()
        .into_iter()
        .find(|webview| webview.label() == window.label())
}

impl<R: Runtime> TauriControlHandler<R> {
    /// Creates the handler over `app`, routing `command` through `commands`.
    pub fn new(app: AppHandle<R>, commands: Arc<dyn CommandBridge>) -> Self {
        Self { app, commands }
    }

    /// Every targetable window, sorted by label for deterministic
    /// targeting.
    ///
    /// Enumeration walks `Manager::windows`, not `webview_windows`: tauri's
    /// `WebviewWindow` exists only for a window whose single webview shares
    /// its label, so a window gaining a child webview (a native-content
    /// island like Figmatic's preview) silently vanishes from
    /// `webview_windows()` (Figmatic adoption finding, 2026-08-20). The
    /// app's UI webview is the one sharing the window's label; child
    /// webviews are not semantic targets.
    fn windows(&self) -> Vec<(String, LiveTarget<R>)> {
        let mut windows: Vec<_> = self
            .app
            .windows()
            .into_iter()
            .filter_map(|(label, window)| {
                ui_webview(&window).map(|webview| (label, LiveTarget { window, webview }))
            })
            .collect();
        windows.sort_by(|(left, _), (right, _)| left.cmp(right));
        windows
    }

    /// Resolves a request's window target to a live window.
    fn resolve_window(
        &self,
        target: &WindowTarget,
    ) -> Result<(WindowId, LiveTarget<R>), ToolError> {
        match target {
            Some(window) => {
                let label = window.as_str();
                self.app
                    .get_window(label)
                    .and_then(|live| {
                        ui_webview(&live).map(|webview| {
                            (
                                window.clone(),
                                LiveTarget {
                                    window: live,
                                    webview,
                                },
                            )
                        })
                    })
                    .ok_or_else(|| ToolError::UnknownWindow {
                        window: window.clone(),
                    })
            }
            None => {
                let windows = self.windows();
                let (label, live) = windows
                    .iter()
                    .find(|(_, live)| live.window.is_focused().unwrap_or(false))
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
    fn window_info(label: &str, window: &Window<R>) -> Result<WindowInfo, ToolError> {
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

    async fn eval_js(
        &self,
        target: &WindowTarget,
        js: String,
    ) -> Result<(longhorn_core::WindowId, serde_json::Value), ToolError> {
        let (window, live) = self.resolve_window(target)?;
        #[cfg(target_os = "macos")]
        {
            let value = crate::capture::evaluate_webview(&live.webview, js).await?;
            Ok((window, value))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (live, js);
            Err(ToolError::Unsupported {
                message: "evaluate is implemented on macOS only (contract 020)".to_owned(),
            })
        }
    }
}

impl<R: Runtime> ControlHandler for TauriControlHandler<R> {
    async fn snapshot(&self, request: SnapshotRequest) -> Result<SnapshotResult, ToolError> {
        let (window, value) = self.eval_js(&request.window, shim::snapshot_js()).await?;
        let (page, root) = shim::decode_snapshot(value)?;
        Ok(SnapshotResult { window, page, root })
    }

    async fn click(&self, request: ClickRequest) -> Result<ActionReceipt, ToolError> {
        let (_, value) = self
            .eval_js(&request.window, shim::click_js(request.element.as_str()))
            .await?;
        shim::decode_action(value)
    }

    async fn r#type(&self, request: TypeRequest) -> Result<ActionReceipt, ToolError> {
        let (_, value) = self
            .eval_js(
                &request.window,
                shim::type_js(request.element.as_str(), &request.text),
            )
            .await?;
        shim::decode_action(value)
    }

    async fn press(&self, request: PressRequest) -> Result<ActionReceipt, ToolError> {
        let modifiers: Vec<String> = request
            .modifiers
            .iter()
            .map(|modifier| match modifier {
                longhorn_agent_control::KeyModifier::Alt => "alt",
                longhorn_agent_control::KeyModifier::Control => "control",
                longhorn_agent_control::KeyModifier::Meta => "meta",
                longhorn_agent_control::KeyModifier::Shift => "shift",
            })
            .map(str::to_owned)
            .collect();
        let (_, value) = self
            .eval_js(
                &request.window,
                shim::press_js(
                    &request.key,
                    &modifiers,
                    request.element.as_ref().map(|element| element.as_str()),
                ),
            )
            .await?;
        shim::decode_action(value)
    }

    async fn scroll(&self, request: ScrollRequest) -> Result<ActionReceipt, ToolError> {
        let (_, value) = self
            .eval_js(
                &request.window,
                shim::scroll_js(
                    request.delta_x,
                    request.delta_y,
                    request.element.as_ref().map(|element| element.as_str()),
                ),
            )
            .await?;
        shim::decode_action(value)
    }

    async fn drag(&self, request: DragRequest) -> Result<ActionReceipt, ToolError> {
        let (_, value) = self
            .eval_js(
                &request.window,
                shim::drag_js(request.source.as_str(), request.target.as_str()),
            )
            .await?;
        shim::decode_action(value)
    }

    async fn evaluate(&self, request: EvaluateRequest) -> Result<EvaluateResult, ToolError> {
        let (_, value) = self.eval_js(&request.window, request.js).await?;
        Ok(EvaluateResult { value })
    }

    async fn wait_for(&self, request: WaitForRequest) -> Result<WaitForResult, ToolError> {
        let js = shim::wait_for_js(&request.predicate);
        let window = request.window.clone();
        shim::poll_until(request.timeout_ms, || {
            let js = js.clone();
            let window = window.clone();
            async move {
                let (_, value) = self.eval_js(&window, js).await?;
                shim::decode_wait(value)
            }
        })
        .await
    }

    async fn screenshot(&self, request: ScreenshotRequest) -> Result<ScreenshotResult, ToolError> {
        let (window, live) = self.resolve_window(&request.window)?;
        #[cfg(target_os = "macos")]
        {
            // The whole window, child webviews composed in (Card 238) —
            // not the UI webview alone.
            let png = crate::capture::screenshot_window(&live.window).await?;
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
            .map(|(label, live)| Self::window_info(label, &live.window))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ListWindowsResult { windows })
    }

    async fn resize_window(
        &self,
        request: ResizeWindowRequest,
    ) -> Result<ActionReceipt, ToolError> {
        let (_, live) = self.resolve_window(&Some(request.window.clone()))?;
        live.window
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
