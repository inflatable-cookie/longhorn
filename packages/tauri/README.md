# @longhorn/tauri

Raw Tauri transport adapters for Longhorn framework-neutral clients.

`TauriTransport` adapts Tauri `invoke` and window-local events to the
structural `@longhorn/core` transport contract. It imports no Longhorn domain;
domain clients remain free of Tauri imports.
