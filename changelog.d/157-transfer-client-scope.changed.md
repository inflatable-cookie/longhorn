Raw `TauriEventTransport` callers listening to transfer client changes must pass `{ scope: "current-webview" }`. `TransferClient.connect()` applies it automatically.
