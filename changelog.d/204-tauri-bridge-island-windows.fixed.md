`longhorn-tauri-bridge` now works from a window that hosts a child webview,
such as a native-content island: the window's UI webview can invoke every
bridge command and receives domain, progress and terminal events.
