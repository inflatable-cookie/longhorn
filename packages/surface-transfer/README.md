# @inflatable-cookie/longhorn-surface-transfer

Optional framework-neutral whole-Surface transfer client.

Rust serde types are authoritative. Regenerate and check the committed
protocol and fixture with:

```sh
effigy generate:surface-transfer
effigy check:surface-transfer-bindings
```

This package composes `@inflatable-cookie/longhorn-surfaces` and `@inflatable-cookie/longhorn-transfer`. The base
transfer package remains usable without Surface state.

The client accepts the same injected transport seam as the base transfer
client. It contains no raw Tauri call, browser global, Svelte store, Poodle
component, provisioning policy, or product window metadata.
