# @inflatable-cookie/longhorn

The framework-neutral tier: fourteen domains behind one package, with no
dependencies and no peers. Rust serde types are authoritative; the generated
protocol modules are checked against them.

```sh
effigy check:bindings
effigy test:ts
```

Every domain is a subpath. `@inflatable-cookie/longhorn` itself resolves to
`core`, the base every other domain builds on.

| Subpath | Domain |
| --- | --- |
| `/core` | identity, opaque values, transports, diagnostics |
| `/agent-control` | in-page semantic shim (dev control surface, contract 022). Synthetic events are untrusted (`isTrusted` stays false); `wait_for` is DOM-relative. |
| `/config` | configuration store, backup, restore, recovery |
| `/layout` | containers, regions, panels, sizing |
| `/surfaces`, `/surface-transfer` | optional Surface documents and their transfer |
| `/transfer` | cross-window drag sessions |
| `/native-content` | native content islands |
| `/bridge` | typed bridge and optional backend topology |
| `/commands` | registry, keymaps, palette |
| `/settings` | settings authority and registry projection |
| `/notifications`, `/operation` | async operations and their notifications |
| `/history`, `/history-tree` | history kernel and optional forkable tree |

Each domain with a Rust authority also exposes `/<domain>/protocol` — the
generated types and compatibility guards on their own, for consumers that want
the wire contract without the client.

Nothing here imports a UI framework, a host runtime, or Poodle. That is
enforced, not merely intended: `tests/boundary.test.ts` fails the build if any
source file in this package acquires such an edge. The reactive bindings live
in `@inflatable-cookie/longhorn-poodle-svelte` and the host edges in
`@inflatable-cookie/longhorn-tauri`, both of which peer on this package.
