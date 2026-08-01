# @longhorn/operation

Framework-neutral checked clients for Longhorn's payload-free operation
catalogue. The root export has no Tauri, bridge, Svelte, or Poodle dependency.

Use `@longhorn/operation/tauri` for local Tauri invocation and
`@longhorn/operation/bridge` for an optional negotiated bridge domain.

Use `@longhorn/operation/svelte` for a per-instance listener lifecycle and
request-keyed cancellation or dismissal state. Use
`@longhorn/operation/poodle` for a controlled panel built only from public
Poodle feedback primitives. Product detail remains an injected snippet.
