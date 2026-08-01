# @longhorn/native-content

Framework-neutral checked client for one Rust-owned native-content island
authority.

## Boundary

The package carries product-neutral desired and observed state, attach
generation, renderer-session epoch, proposals, and exact receipts. It has no
browser, plugin, GPU, Svelte, Poodle, semantic-input, or native-handle API.

`NativeContentClient.connect()` installs its event listener before requesting
a fresh host-issued client epoch. That epoch is separate from the island's
attach generation. Async results are installed only while their authority and
client epochs remain current and both independent state revisions are
monotonic.

Use `createDirectNativeContentPort` for injected in-process tests,
`SerializedNativeContentPort` for deterministic JSON-loopback proof, or the
`@longhorn/native-content/tauri` subpath with an injected Longhorn transport.
The root import has no Tauri dependency.

Capability examples under `examples/` authorize only access to these four
protocol commands. They do not authorize child content, navigation, plugins,
rendering, product mutation, or remote-webview capabilities.
