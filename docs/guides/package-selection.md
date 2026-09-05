# Choose Packages

Status: checked private adoption guidance
Updated: 2026-08-15
Governing contracts: [002](../contracts/002-composable-workspace-hosting.md),
[007](../contracts/007-optional-backend-topology.md),
[012](../contracts/012-distribution-and-compatibility.md), and
[013](../contracts/013-svelte-and-poodle-adapter-lifecycle.md)

## Why This Matters

There is no "install Longhorn" — each system is a separate Rust crate, and
every crate you add pulls in its own host adapter and capability surface. The
TypeScript side mirrors that selection through export subpaths of one
package. The choice you make here determines the build, the security surface,
and the maintenance load of your app. Selecting the smallest proven shape
that covers your needs is the single biggest lever on integration cost, which
is why this guide comes first. Terms are defined in the
[glossary](glossary.md).

## Rule

Start with the authority the app needs — the single owner of each durable
concern, such as config or settings. Select its pure package, then only the
storage, Tauri, Svelte, Poodle, or service adapters required to expose it.
There is no umbrella package. A visual similarity to another app is not a
dependency reason.

Nothing below is on a registry today — there is no npm/crates.io install of
Longhorn or Poodle. TypeScript pins the three packages by `file:` path
against a Longhorn checkout (recipe below). Rust resolves crates against the
Cargo inventory and source archives of the
[private 0.1 candidate](../reference/private-0-1-candidate.md), whose receipt
is frozen history: it records the 17-TypeScript-package tree of 2026-08-02,
not the current three-package tree. Published versions arrive with g02.014,
blocked on Poodle v0.2.0.

## Proven Starting Shapes

| Shape | Use when | Hierarchy | Excluded by proof |
| --- | --- | --- | --- |
| minimal | preferences and a centralized settings host are enough | window | layout/Surface, bridge, history, native content |
| workspace | one or more windows need registered regions and panels | window → Surface → region → panel | transfer, bridge, history, native content |
| full hosting | content can move through displays, windows, Surfaces, regions, and panels | display → window → Surface → region → panel | bridge, native content, operations, notifications |
| optional server | local config/settings remain authoritative while a service is optional | window plus optional bridge | layout/Surface, history, native content |

A Surface is the layout — there is no separate layout container — so the
workspace and full-hosting chains name Surface where they once named one.

The checked roots in the
[greenfield compositions](../../examples/greenfield-compositions/README.md)
are the live equivalents of these shapes. Copy package selection from that
shape's `package.json` and `src-tauri/Cargo.toml`; replace the neutral app
identity and product policy. `capability.json` is selection evidence, not a
universal Tauri capability. The
[Card 125 matrix](../architecture/greenfield-composition-matrix.md) records
the shape boundaries and extension points.

The minimal shape selects two Longhorn TypeScript packages — one dependency
on `@inflatable-cookie/longhorn` plus its Tauri peer — and six Rust crates:

```json
{
  "name": "my-app",
  "dependencies": {
    "@inflatable-cookie/longhorn": "file:../longhorn/packages/longhorn",
    "@inflatable-cookie/longhorn-tauri": "file:../longhorn/packages/longhorn-tauri",
    "@tauri-apps/api": "2.10.1",
    "svelte": "5.56.8"
  },
  "overrides": {
    "@inflatable-cookie/longhorn": "file:../longhorn/packages/longhorn"
  }
}
```

`./config`, `./settings`, and the other system names are export subpaths of
the one package, not installable packages — never depend on
`@inflatable-cookie/longhorn/config` directly. The `overrides` entry is
required: `longhorn-tauri` peer-depends on `@inflatable-cookie/longhorn` at
exact `0.1.0`, and without the override bun reaches the registry for it and
404s. Shapes that select Svelte/Poodle bindings add
`@inflatable-cookie/longhorn-poodle-svelte` by the same `file:` discipline;
the Poodle packages it peers on are unpublished until Poodle v0.2.0 and pin
from a Poodle checkout the same way. The published-version path arrives with
g02.014. The full recipe, including the development-link alternative, is in
[Getting Started](getting-started.md).

```toml
[dependencies]
longhorn-config = "0.1.0"
longhorn-core = "0.1.0"
longhorn-settings = "0.1.0"
longhorn-settings-config = "0.1.0"
longhorn-tauri-config = "0.1.0"
longhorn-tauri-settings = "0.1.0"
tauri = "2"
```

The proof manifests inside this repo resolve `longhorn-*` through the
workspace (`longhorn-config.workspace = true`). That form is proof-only:
a standalone consumer resolves the same names from the produced private
artifacts described in the [distribution reference](../reference/private-0-1-candidate.md).

## Add One System At A Time

| Need | Pure/client selection | Add only when needed |
| --- | --- | --- |
| config, backup, restore | `longhorn-config`, `@inflatable-cookie/longhorn/config` | config Tauri host, age encryption, Poodle recovery pages |
| settings registry and shell | `longhorn-settings`, `@inflatable-cookie/longhorn/settings` | config apply units, Tauri host, `/svelte`, `/poodle` |
| display/window hosting | `longhorn-display`, `longhorn-windowing` | config persistence and Tauri window adapter |
| registered regions and panels (layout) | `longhorn-surfaces` — a Surface is the layout — `@inflatable-cookie/longhorn/layout` | `longhorn-surfaces-config` persistence, Svelte, Poodle bindings |
| Surfaces projected into window hosts | add `longhorn-surface-windowing`, `@inflatable-cookie/longhorn/surfaces` | persistence and window-host projection |
| panel transfer | `longhorn-transfer`, `@inflatable-cookie/longhorn/transfer` | Tauri transfer host and Svelte/Poodle interaction |
| whole-Surface transfer | Surface plus `longhorn-surface-transfer` | `surface-transfer` host feature and renderer subpath |
| commands and keymaps | `longhorn-command`, `@inflatable-cookie/longhorn/commands` | config, settings, Tauri, `/svelte`, `/poodle` |
| linear history | `longhorn-history`, `@inflatable-cookie/longhorn/history` | Tauri metadata, `/svelte`, `/poodle`; product journal stays local |
| optional service bridge | `longhorn-bridge`, `@inflatable-cookie/longhorn/bridge` | Tauri, events, or stream subpaths |
| finite operations | `longhorn-operation`, `@inflatable-cookie/longhorn/operation` | Tauri, bridge, Svelte, or Poodle edges |
| retained notifications | `longhorn-notifications`, `@inflatable-cookie/longhorn/notifications` | Tauri, Svelte, Poodle, or operation observation |
| native-content island | `longhorn-native-content`, `@inflatable-cookie/longhorn/native-content` | select exactly one host mechanism and optional Svelte lifetime |

## Held Surface (built, not selectable)

These systems are implemented and tested but not ready for consumer
implementation. Do not select them; the
[held-surface register](../reference/held-surface.md) is the authoritative
status list and names the trigger for each.

| Surface | Status | Awaits |
| --- | --- | --- |
| bridge supervision and contract machinery | held-for-consumer | a bridge consumer with a real service topology |
| age encryption (`longhorn-config-age`) | held-for-consumer | a consumer that needs encrypted backups |
| fork-tree history (`longhorn-history-tree`, `@inflatable-cookie/longhorn/history-tree`) | planning-evidence | the recorded g01.017 adoption decision |

Application update (`longhorn-update`, `@inflatable-cookie/longhorn/update`)
and licensing (`longhorn-licence`, `@inflatable-cookie/longhorn/licence`)
graduated from this register on 2026-08-15 — g02.009 and g02.010 delivered
their crates, renderer surfaces, Tauri hosts, and packaged proofs — and are
ordinary selectable systems.

The [generated API surface](../reference/api-surface.md) is authoritative for
current features and export subpaths.

## Poodle And Svelte

Poodle owns visual primitives. Longhorn adapters use public controlled props,
snippets, and events; product bodies, labels, icons, framing, and policy remain
in the app. Never import Poodle internals or copy component source.

Create Svelte sessions per window or mounted host. Listener registration comes
before the first snapshot. Loading, unsupported, reconnecting, and failed
states stay visible. Stop releases listeners, timers, leases, and optimistic
projection. Renderer state never becomes durable fallback truth.

## Package-choice Checklist

- name each durable and runtime authority
- select the smallest proven shape
- record every optional edge and why it exists
- use one Svelte, Poodle, and Tauri runtime
- generate TypeScript from Rust-owned serialized contracts
- grant only commands used by the selected host
- prove the install from produced artifacts outside sibling workspaces
- keep product schemas, commands, payloads, executors, and presentation local
- fail visibly when a required capability or protocol is unavailable
