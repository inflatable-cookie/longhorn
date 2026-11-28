`longhorn-tauri-bridge` no longer copies an arbitrary renderer route into an
`unknownRoute` error: only a well-formed route of at most 128 bytes is echoed.
