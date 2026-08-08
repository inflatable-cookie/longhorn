# @inflatable-cookie/longhorn-tauri

Raw Tauri transport edges. The root is invoke-only. Import
`@inflatable-cookie/longhorn-tauri/events` only for hosts that publish or subscribe to events.

Raw Tauri transport adapters for Longhorn framework-neutral clients.

`TauriTransport` adapts Tauri `invoke` and window-local events to the
structural `@inflatable-cookie/longhorn-core` transport contract. It imports no Longhorn domain;
domain clients remain free of Tauri imports.
