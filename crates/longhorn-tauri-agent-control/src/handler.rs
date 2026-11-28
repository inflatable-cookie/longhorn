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
//! steady state while an agent drives the app unfocused. Semantic and input
//! tools take an optional `webview` label: absent means the UI webview;
//! present means an opted-in child of that window. Refs stamped in a child
//! are namespaced to that child's label so they cannot resolve in another
//! webview.

use std::{collections::BTreeSet, future::Future, sync::Arc, time::Duration};

use longhorn_agent_control::{
    ActionReceipt, ClickRequest, CommandRequest, CommandResult, ControlHandler, DragRequest,
    EvaluateRequest, EvaluateResult, ListWindowsRequest, ListWindowsResult, PageEventBatch,
    PageEventReader, PressRequest, ResizeWindowRequest, ScreenshotRequest, ScreenshotResult,
    ScrollRequest, SetFileInputRequest, SnapshotRequest, SnapshotResult, ToolError, TypeRequest,
    ValidatedSetFileInputRequest, WaitForRequest, WaitForResult, WebviewLabel, WebviewTarget,
    WindowTarget,
};
use longhorn_core::WindowId;
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, Webview, Window};
use tokio::sync::oneshot;

use crate::{bridge::CommandBridge, shim};

mod window_ops;

use window_ops::{window_focused, window_info};

/// Bound a tool's target resolution and JavaScript evaluation together.
const WEBVIEW_EVAL_TIMEOUT: Duration = Duration::from_secs(5);

async fn bound_evaluation<T>(
    evaluation: impl Future<Output = Result<T, ToolError>>,
) -> Result<T, ToolError> {
    tokio::time::timeout(WEBVIEW_EVAL_TIMEOUT, evaluation)
        .await
        .map_err(|_| ToolError::EvaluationFailed {
            message: format!(
                "webview evaluation timed out after {} ms",
                WEBVIEW_EVAL_TIMEOUT.as_millis()
            ),
        })?
}

/// Wry's synchronous getters can wait on the main thread. Queue those
/// operations and await their result instead, so shutdown can drop the wait.
pub(crate) async fn on_main_thread<R: Runtime, T: Send + 'static>(
    app: &AppHandle<R>,
    operation: impl FnOnce() -> Result<T, ToolError> + Send + 'static,
) -> Result<T, ToolError> {
    let (tx, rx) = oneshot::channel();
    app.run_on_main_thread(move || {
        if !tx.is_closed() {
            let _ = tx.send(operation());
        }
    })
    .map_err(|error| ToolError::HostFailure {
        message: format!("main-thread dispatch failed: {error}"),
    })?;
    rx.await.map_err(|_| ToolError::HostFailure {
        message: "the main thread dropped the operation result".to_owned(),
    })?
}

/// Host authority for the control surface, backed by the Tauri app handle.
pub struct TauriControlHandler<R: Runtime> {
    app: AppHandle<R>,
    commands: Arc<dyn CommandBridge>,
    /// Child-webview labels opted in as semantic targets at mount. Empty
    /// means today's UI-webview-only behavior.
    semantic_children: BTreeSet<String>,
}

/// One resolved window + the webview a semantic/input call will drive.
/// The default is the UI webview (same label as the window). An opted-in
/// child is selected only when the request names it.
struct LiveTarget<R: Runtime> {
    window: Window<R>,
    webview: Webview<R>,
    webview_label: String,
    is_ui: bool,
}

impl<R: Runtime> Clone for LiveTarget<R> {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            webview: self.webview.clone(),
            webview_label: self.webview_label.clone(),
            is_ui: self.is_ui,
        }
    }
}

impl<R: Runtime> LiveTarget<R> {
    /// Child refs are namespaced in the DOM as `{encodeURIComponent(label)}:eN`
    /// so a colliding `eN` in another webview cannot resolve. UI webviews
    /// stamp unprefixed `eN` — today's wire.
    fn with_ref_prefix(&self, js: String) -> String {
        let prefix = if self.is_ui {
            "''".to_owned()
        } else {
            format!(
                "encodeURIComponent({})",
                Value::String(self.webview_label.clone())
            )
        };
        format!("globalThis.__longhornAgentRefPrefix = {prefix};\n{js}")
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
    pub fn new(
        app: AppHandle<R>,
        commands: Arc<dyn CommandBridge>,
        semantic_children: BTreeSet<String>,
    ) -> Self {
        Self {
            app,
            commands,
            semantic_children,
        }
    }

    /// Every targetable window, sorted by label for deterministic
    /// targeting.
    ///
    /// Enumeration walks `Manager::windows`, not `webview_windows`: tauri's
    /// `WebviewWindow` exists only for a window whose single webview shares
    /// its label, so a window gaining a child webview (a native-content
    /// island like Figmatic's preview) silently vanishes from
    /// `webview_windows()` (Figmatic adoption finding, 2026-08-20). The
    /// app's UI webview is the one sharing the window's label. Child
    /// webviews are semantic only when opted in at mount.
    fn windows(&self) -> Vec<(String, LiveTarget<R>)> {
        let mut windows: Vec<_> = self
            .app
            .windows()
            .into_iter()
            .filter_map(|(label, window)| {
                ui_webview(&window).map(|webview| {
                    (
                        label.clone(),
                        LiveTarget {
                            window,
                            webview,
                            webview_label: label,
                            is_ui: true,
                        },
                    )
                })
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
                                    webview_label: window.as_str().to_owned(),
                                    is_ui: true,
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
                let (label, live) = match windows.as_slice() {
                    [] => {
                        return Err(ToolError::Unsupported {
                            message: "the app has no webview windows to target".to_owned(),
                        });
                    }
                    [first, ..] => {
                        let mut focused = first;
                        for window in &windows {
                            if window_focused(&window.0, &window.1.window)? {
                                focused = window;
                                break;
                            }
                        }
                        focused
                    }
                };
                let window =
                    WindowId::new(label.clone()).map_err(|error| ToolError::HostFailure {
                        message: format!(
                            "window label {label:?} is not a valid window id: {error}"
                        ),
                    })?;
                Ok((window, live.clone()))
            }
        }
    }

    /// Resolves a semantic/input target: window as today, then an optional
    /// child webview. Absent `webview` is the UI webview. A hosted child
    /// that was not opted in at mount answers typed `Unsupported`; a label
    /// that matches no hosted webview answers [`ToolError::UnknownWebview`].
    fn resolve_semantic(
        &self,
        window: &WindowTarget,
        webview: &WebviewTarget,
    ) -> Result<(WindowId, LiveTarget<R>), ToolError> {
        let (window_id, mut live) = self.resolve_window(window)?;
        let Some(label) = webview else {
            return Ok((window_id, live));
        };
        let hosted = live
            .window
            .webviews()
            .into_iter()
            .find(|candidate| candidate.label() == label.as_str())
            .ok_or_else(|| ToolError::UnknownWebview {
                webview: label.clone(),
            })?;
        let is_ui = hosted.label() == live.window.label();
        if !is_ui && !self.semantic_children.contains(label.as_str()) {
            return Err(ToolError::Unsupported {
                message: format!(
                    "child webview {:?} is not opted in as a semantic target; name it with AgentControlConfig::with_semantic_child at mount",
                    label.as_str()
                ),
            });
        }
        live.webview = hosted;
        live.webview_label = label.as_str().to_owned();
        live.is_ui = is_ui;
        Ok((window_id, live))
    }

    fn snapshot_webview_field(live: &LiveTarget<R>) -> Result<WebviewTarget, ToolError> {
        if live.is_ui {
            return Ok(None);
        }
        WebviewLabel::new(live.webview_label.clone())
            .map(Some)
            .map_err(|error| ToolError::HostFailure {
                message: format!(
                    "webview label {:?} is not a valid webview id: {error}",
                    live.webview_label
                ),
            })
    }

    async fn with_handler_on_main<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&Self) -> Result<T, ToolError> + Send + 'static,
    ) -> Result<T, ToolError> {
        let handler = Self {
            app: self.app.clone(),
            commands: Arc::clone(&self.commands),
            semantic_children: self.semantic_children.clone(),
        };
        on_main_thread(&self.app, move || operation(&handler)).await
    }

    async fn eval_js(
        &self,
        window: &WindowTarget,
        webview: &WebviewTarget,
        js: String,
    ) -> Result<(WindowId, LiveTarget<R>, serde_json::Value), ToolError> {
        bound_evaluation(self.eval_js_unbounded(window, webview, js)).await
    }

    async fn eval_js_unbounded(
        &self,
        window: &WindowTarget,
        webview: &WebviewTarget,
        js: String,
    ) -> Result<(WindowId, LiveTarget<R>, Value), ToolError> {
        let window = window.clone();
        let webview = webview.clone();
        let (window_id, live) = self
            .with_handler_on_main(move |handler| handler.resolve_semantic(&window, &webview))
            .await?;
        let js = live.with_ref_prefix(js);
        let value = eval_webview(&live.webview, js).await?;
        Ok((window_id, live, value))
    }
}

async fn eval_webview<R: Runtime>(webview: &Webview<R>, js: String) -> Result<Value, ToolError> {
    #[cfg(target_os = "macos")]
    {
        crate::capture::evaluate_webview(webview, js).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (webview, js);
        Err(ToolError::Unsupported {
            message: "evaluate is implemented on macOS only (contract 020)".to_owned(),
        })
    }
}

impl<R: Runtime> ControlHandler for TauriControlHandler<R> {
    async fn snapshot(&self, request: SnapshotRequest) -> Result<SnapshotResult, ToolError> {
        let (window, live, value) = self
            .eval_js(&request.window, &request.webview, shim::snapshot_js())
            .await?;
        let (page, root) = shim::decode_snapshot(value)?;
        Ok(SnapshotResult {
            window,
            webview: Self::snapshot_webview_field(&live)?,
            page,
            root,
        })
    }

    async fn click(&self, request: ClickRequest) -> Result<ActionReceipt, ToolError> {
        let (_, _, value) = self
            .eval_js(
                &request.window,
                &request.webview,
                shim::click_js(request.element.as_str()),
            )
            .await?;
        shim::decode_action(value)
    }

    async fn r#type(&self, request: TypeRequest) -> Result<ActionReceipt, ToolError> {
        let (_, _, value) = self
            .eval_js(
                &request.window,
                &request.webview,
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
        let (_, _, value) = self
            .eval_js(
                &request.window,
                &request.webview,
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
        let (_, _, value) = self
            .eval_js(
                &request.window,
                &request.webview,
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
        let (_, _, value) = self
            .eval_js(
                &request.window,
                &request.webview,
                shim::drag_js(request.source.as_str(), request.target.as_str()),
            )
            .await?;
        shim::decode_action(value)
    }

    async fn set_file_input(
        &self,
        request: SetFileInputRequest,
    ) -> Result<ActionReceipt, ToolError> {
        let validated = request.into_validated()?;
        self.set_file_input_prevalidated(validated).await
    }

    async fn set_file_input_prevalidated(
        &self,
        request: ValidatedSetFileInputRequest,
    ) -> Result<ActionReceipt, ToolError> {
        let request = request.as_request();
        let (_, _, value) = self
            .eval_js(
                &request.window,
                &request.webview,
                shim::set_file_input_js(request.element.as_str(), &request.files),
            )
            .await?;
        shim::decode_action(value)
    }

    async fn evaluate(&self, request: EvaluateRequest) -> Result<EvaluateResult, ToolError> {
        let marked = crate::shim::compose_evaluate_js(&request.js);
        let (_, _, value) = self
            .eval_js(&request.window, &request.webview, marked)
            .await?;
        Ok(EvaluateResult { value })
    }

    async fn read_page_events(
        &self,
        since_seq: u64,
        since_epoch: Option<&str>,
    ) -> Result<PageEventBatch, ToolError> {
        bound_evaluation(self.page_event_reader().read(since_seq, since_epoch)).await
    }

    fn page_event_reader(&self) -> impl PageEventReader {
        DefaultPageEvents {
            handler: self,
            webview: None,
        }
    }

    async fn wait_for(&self, request: WaitForRequest) -> Result<WaitForResult, ToolError> {
        request.validate()?;
        let js = shim::wait_for_js(&request.predicate);
        let window = request.window.clone();
        let webview = request.webview.clone();
        shim::poll_until(request.timeout_ms, || {
            let js = js.clone();
            let window = window.clone();
            let webview = webview.clone();
            async move {
                let (_, _, value) = self.eval_js_unbounded(&window, &webview, js).await?;
                shim::decode_wait(value)
            }
        })
        .await
    }

    async fn screenshot(&self, request: ScreenshotRequest) -> Result<ScreenshotResult, ToolError> {
        let (window, live) = self
            .with_handler_on_main(move |handler| handler.resolve_window(&request.window))
            .await?;
        #[cfg(target_os = "macos")]
        {
            // The whole window, child webviews composed in (Card 238) —
            // not the UI webview alone.
            let png = crate::capture::screenshot_window(&live.window).await?;
            Ok(ScreenshotResult { window, png })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (window, live);
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
        self.with_handler_on_main(|handler| {
            let windows = handler
                .windows()
                .iter()
                .map(|(label, live)| window_info(label, &live.window))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ListWindowsResult { windows })
        })
        .await
    }

    async fn resize_window(
        &self,
        request: ResizeWindowRequest,
    ) -> Result<ActionReceipt, ToolError> {
        self.with_handler_on_main(move |handler| {
            let (_, live) = handler.resolve_window(&Some(request.window.clone()))?;
            window_ops::resize_window(&request, &live.window)
        })
        .await
    }
}

/// Keep the resolved webview even when focus or the document epoch changes.
struct DefaultPageEvents<'a, R: Runtime> {
    handler: &'a TauriControlHandler<R>,
    webview: Option<Webview<R>>,
}

impl<R: Runtime> DefaultPageEvents<'_, R> {
    async fn target(&mut self) -> Result<&Webview<R>, ToolError> {
        let webview = match &mut self.webview {
            Some(webview) => webview,
            slot @ None => {
                let (_, live) = self
                    .handler
                    .with_handler_on_main(|handler| handler.resolve_window(&None))
                    .await?;
                slot.insert(live.webview)
            }
        };
        Ok(webview)
    }
}

impl<R: Runtime> PageEventReader for DefaultPageEvents<'_, R> {
    async fn read(
        &mut self,
        since_seq: u64,
        since_epoch: Option<&str>,
    ) -> Result<PageEventBatch, ToolError> {
        let webview = self.target().await?;
        let js = shim::read_events_js(since_seq, since_epoch);
        shim::decode_events(eval_webview(webview, js).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::NoCommandBridge;
    use longhorn_agent_control::{ElementRef, FileInputFile, MAX_FILE_INPUT_BYTES};
    use tauri::WebviewWindowBuilder;

    fn zeros_base64(len: usize) -> String {
        let full_groups = len / 3;
        let rem = len % 3;
        let mut encoded = "AAAA".repeat(full_groups);
        match rem {
            1 => encoded.push_str("AA=="),
            2 => encoded.push_str("AAA="),
            _ => {}
        }
        encoded
    }

    fn file_request(files: Vec<FileInputFile>) -> SetFileInputRequest {
        SetFileInputRequest {
            window: None,
            webview: None,
            element: ElementRef::new("e1").unwrap(),
            files,
        }
    }

    fn handler(
        app: &tauri::App<tauri::test::MockRuntime>,
    ) -> TauriControlHandler<tauri::test::MockRuntime> {
        TauriControlHandler::new(
            app.handle().clone(),
            Arc::new(NoCommandBridge),
            BTreeSet::new(),
        )
    }

    #[tokio::test]
    async fn a_listener_retains_its_webview_when_the_default_changes() {
        let app = tauri::test::mock_app();
        WebviewWindowBuilder::new(app.handle(), "b", Default::default())
            .build()
            .unwrap();
        let handler = handler(&app);
        let mut reader = DefaultPageEvents {
            handler: &handler,
            webview: None,
        };
        assert_eq!(reader.target().await.unwrap().label(), "b");

        // The mock runtime cannot change focus. Add an earlier label
        // to make the same default resolver select a different webview.
        WebviewWindowBuilder::new(app.handle(), "a", Default::default())
            .build()
            .unwrap();
        let mut new_reader = DefaultPageEvents {
            handler: &handler,
            webview: None,
        };
        assert_eq!(new_reader.target().await.unwrap().label(), "a");
        assert_eq!(reader.target().await.unwrap().label(), "b");
    }

    #[tokio::test]
    async fn invalid_host_window_label_reports_host_failure() {
        let app = tauri::test::mock_app();
        let label = "w".repeat(longhorn_core::MAX_OPAQUE_ID_BYTES + 1);
        WebviewWindowBuilder::new(app.handle(), label, Default::default())
            .build()
            .unwrap();

        let error = handler(&app)
            .list_windows(ListWindowsRequest {})
            .await
            .unwrap_err();
        assert!(matches!(error, ToolError::HostFailure { .. }));
    }

    #[tokio::test]
    async fn set_file_input_refuses_over_cap_before_webview() {
        let app = tauri::test::mock_app();
        WebviewWindowBuilder::new(app.handle(), "main", Default::default())
            .build()
            .unwrap();
        let error = handler(&app)
            .set_file_input(file_request(vec![FileInputFile {
                name: "blob.bin".to_owned(),
                media_type: None,
                content_base64: zeros_base64(MAX_FILE_INPUT_BYTES + 1),
            }]))
            .await
            .unwrap_err();
        assert_eq!(
            error,
            ToolError::InvalidArgument {
                message: "set_file_input decoded total exceeds 8 MiB".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn set_file_input_refuses_invalid_payloads_before_webview() {
        let app = tauri::test::mock_app();
        WebviewWindowBuilder::new(app.handle(), "main", Default::default())
            .build()
            .unwrap();
        let cases = [
            (Vec::new(), "at least one file"),
            (
                vec![FileInputFile {
                    name: String::new(),
                    media_type: None,
                    content_base64: zeros_base64(3),
                }],
                "non-empty",
            ),
            (
                vec![FileInputFile {
                    name: "note.txt".to_owned(),
                    media_type: None,
                    content_base64: "@@@".to_owned(),
                }],
                "base64",
            ),
        ];
        for (files, needle) in cases {
            let error = handler(&app)
                .set_file_input(file_request(files))
                .await
                .unwrap_err();
            match error {
                ToolError::InvalidArgument { message } => {
                    assert!(
                        message.contains(needle),
                        "expected {needle:?} in {message:?}"
                    );
                }
                other => panic!("expected InvalidArgument, got {other:?}"),
            }
        }
    }
}
