Tauri host failures display their diagnostic message in Longhorn panels. The domain protocols now export generated host error types. Bridge domain, progress, and terminal events stay in the webview that owns the session.

### Breaking

- Catch `TauriHostError` from `TauriTransport.invoke` for typed host failures. `message`, `code`, and `retryable` are available on the error instance. Migrate checks that expect a plain rejection object or enumerate its fields.
- Use `TauriBridgeStreamSource` and `listenTauriBridgeJob` for bridge events. Raw listeners must migrate from Tauri `listen` to local DOM `CustomEvent` listeners on `window`, reading `detail`. Deploy matched Rust and renderer adapters. Custom bridge transports must honor the requested scope and keep delivery local to the session's renderer.
- Raw `TauriEventTransport` callers listening to transfer client changes must pass `{ scope: "current-webview" }`. `TransferClient.connect()` applies it automatically.
- Use `TauriTransport` with `createTauriUpdatePort`, or pass an `onProgress` Tauri channel when invoking prepare directly. The update adapter no longer exports or publishes `UPDATE_PROGRESS_EVENT`. Snapshot callers should read the plain snapshot and drop `includeProgressCapabilities` and `scopedPrepareProgress`.
