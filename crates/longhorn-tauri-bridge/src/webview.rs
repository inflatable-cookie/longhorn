//! Window identity that survives multiwebview windows.

use tauri::{Manager, Runtime, Webview};

/// The UI webview of the window named `label`: the hosted webview sharing the
/// window's label, however many child webviews are attached.
///
/// Tauri's `WebviewWindow` exists only for a window whose every webview shares
/// its label, so a window hosting a child webview (a native-content island)
/// disappears from `get_webview_window`. Walking the `Window` does not.
pub(crate) fn ui_webview<R: Runtime>(manager: &impl Manager<R>, label: &str) -> Option<Webview<R>> {
    manager
        .get_window(label)?
        .webviews()
        .into_iter()
        .find(|webview| webview.label() == label)
}
