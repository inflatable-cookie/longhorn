**Breaking:** the `longhorn_bridge_*` commands in `longhorn-tauri-bridge` take
`tauri::Webview<R>` instead of `WebviewWindow<R>`, admit only the window's UI
webview (child webviews fail with `invalidSession`), and the crate now enables
Tauri's `unstable` feature, under which Tauri creates each window's own webview
as a child view of the window. Invoke-handler registration is unchanged; Rust
code that calls a command function directly passes the window's `Webview`, and
apps should check window resize and focus behaviour after upgrading.
