# @inflatable-cookie/longhorn-layout

Framework-neutral TypeScript protocol for `longhorn-layout`.

Rust serde types are authoritative. `src/generated/protocol.ts` is checked in
and regenerated with:

```sh
effigy generate:layout
effigy check:layout-bindings
```

Import durable protocol types, compatibility guards, integer-ratio helpers,
and ordinary visibility projection from the package root:

```ts
import {
  LAYOUT_PROTOCOL_VERSION,
  assertCompatibleLayoutMutationCommand,
  assertLayoutProtocolVersion,
  layoutRatioFromMillionths,
  type LayoutDocument,
  type LayoutMutationRequest,
} from "@inflatable-cookie/longhorn-layout";
```

Call the compatibility guards at a transport or stored-fixture boundary before
treating unknown data as a generated protocol type. Unknown protocol versions,
command variants, outcome variants, and rejection codes fail explicitly.

The package has no browser, Tauri, Svelte, or Poodle dependency. Transient
visibility still requires Rust registry policy; the TypeScript helper projects
only ordinary occupied/empty visibility where behavior is exact.

Rust-generated Loophole and Nucleus fixture files under `fixtures/layout/`
exercise the same resolver and mutation sequence. Package tests consume their
expected snapshots directly without duplicating mutation behavior.
