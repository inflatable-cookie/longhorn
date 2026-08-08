# @inflatable-cookie/longhorn-commands

Checked clients and projections for the Rust-authoritative command catalogue,
availability, keyboard, and configuration-backed keymap protocol.

The root export provides:

- generated wire types and fail-closed compatibility guards
- injected catalogue, keymap, availability, and executor ports
- generation-checked controller state and stale-result rejection
- canonical search, menu, palette, help, shortcut, reverse-lookup, and
  conflict projections
- browser-event normalization and exact Rust-compatible keyboard resolution

It has no Tauri, Svelte, Poodle, bridge, or product execution dependency.
Execution always calls the consumer-supplied executor.

Optional subpaths:

- `@inflatable-cookie/longhorn-commands/svelte`: one per-instance session and mounted lifecycle
- `@inflatable-cookie/longhorn-commands/poodle`: controlled public Poodle palette and keybinding
  settings bindings
