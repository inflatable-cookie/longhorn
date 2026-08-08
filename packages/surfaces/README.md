# @inflatable-cookie/longhorn-surfaces

Framework-neutral TypeScript protocol and client for `longhorn-surfaces`.

Rust serde types are authoritative. Regenerate and check the committed
protocol and fixture with:

```sh
effigy generate:surfaces
effigy check:surface-bindings
```

The client accepts the structural `@inflatable-cookie/longhorn-core` transport and checked
connection lifetime. It contains no raw Tauri call, browser global, Svelte
store, or Poodle component. Subscriptions attach the event listener before
querying current authority, reconcile by epoch and revision, and support
disposal while listener registration is still pending.

Unknown protocol versions, mutation variants, response statuses, and rejection
codes fail explicitly at the transport boundary.
