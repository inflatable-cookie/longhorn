# Choose Packages

Status: checked private adoption guidance
Updated: 2026-08-02
Governing contracts: [002](../contracts/002-composable-workspace-hosting.md),
[007](../contracts/007-optional-backend-topology.md),
[012](../contracts/012-distribution-and-compatibility.md), and
[013](../contracts/013-svelte-and-poodle-adapter-lifecycle.md)

## Why This Matters

There is no "install Longhorn" — each system is a separate package, and every
package you add pulls in its own host adapter and capability surface. The
choice you make here determines the build, the security surface, and the
maintenance load of your app. Selecting the smallest proven shape that covers
your needs is the single biggest lever on integration cost, which is why this
guide comes first. Terms are defined in the [glossary](glossary.md).

## Rule

Start with the authority the app needs — the single owner of each durable
concern, such as config or settings. Select its pure package, then only the
storage, Tauri, Svelte, Poodle, or service adapters required to expose it.
There is no umbrella package. A visual similarity to another app is not a
dependency reason.

All names below are working private `0.1.0` names. Use the exact produced
artifacts and locks from the
[active private distribution receipt](../reference/private-0-1-candidate.md),
which also explains how the artifacts are produced and obtained. Do not run a
registry install command.

## Proven Starting Shapes

| Shape | Use when | Hierarchy | TypeScript packages | Rust packages | Excluded by proof |
| --- | --- | --- | --- | --- | --- |
| minimal | preferences and a centralized settings host are enough | window | 4 | 6 | layout, Surface, bridge, history, native content |
| workspace | one or more windows need registered regions and panels | window → layout container → region → panel | 8 | 16 | Surface, transfer, bridge, history, native content |
| full hosting | content can move through displays, windows, Surfaces, regions, and panels | display → window → Surface → layout container → region → panel | 12 | 24 | bridge, native content, operations, notifications |
| optional server | local config/settings remain authoritative while a service is optional | window plus optional bridge | 5 | 8 | layout, Surface, history, native content |

The [Card 125 matrix](../architecture/greenfield-composition-matrix.md) records
the exact graph. Copy package selection from that shape's `package.json` and
`src-tauri/Cargo.toml`; replace the neutral app identity and product policy.
`capability.json` is selection evidence, not a universal Tauri capability.

The minimal shape selects exactly four TypeScript and six Rust packages:

```json
{
  "name": "my-app",
  "dependencies": {
    "@longhorn/config": "0.1.0",
    "@longhorn/core": "0.1.0",
    "@longhorn/settings": "0.1.0",
    "@longhorn/tauri": "0.1.0",
    "@poodle/svelte": "0.1.0",
    "svelte": "5.38.6"
  }
}
```

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
| config, backup, restore | `longhorn-config`, `@longhorn/config` | config Tauri host, age encryption, Poodle recovery pages |
| settings registry and shell | `longhorn-settings`, `@longhorn/settings` | config apply units, Tauri host, `/svelte`, `/poodle` |
| display/window hosting | `longhorn-display`, `longhorn-windowing` | config persistence and Tauri window adapter |
| registered regions/panels | `longhorn-layout`, `@longhorn/layout` | config persistence, Svelte, Poodle bindings |
| optional Surfaces | `longhorn-surfaces`, `@longhorn/surfaces` | persistence and window-host projection |
| panel transfer | `longhorn-transfer`, `@longhorn/transfer` | Tauri transfer host and Svelte/Poodle interaction |
| whole-Surface transfer | Surface plus `longhorn-surface-transfer` | `surface-transfer` host feature and renderer subpath |
| commands and keymaps | `longhorn-command`, `@longhorn/commands` | config, settings, Tauri, `/svelte`, `/poodle` |
| linear history | `longhorn-history`, `@longhorn/history` | Tauri metadata, `/svelte`, `/poodle`; product journal stays local |
| optional service bridge | `longhorn-bridge`, `@longhorn/bridge` | Tauri, events, or stream subpaths |
| finite operations | `longhorn-operation`, `@longhorn/operation` | Tauri, bridge, Svelte, or Poodle edges |
| retained notifications | `longhorn-notifications`, `@longhorn/notifications` | Tauri, Svelte, Poodle, or operation observation |
| native-content island | `longhorn-native-content`, `@longhorn/native-content` | select exactly one host mechanism and optional Svelte lifetime |

## Held Surface (built, not selectable)

These systems are implemented and tested but not ready for consumer
implementation. Do not select them; the
[held-surface register](../reference/held-surface.md) is the authoritative
status list and names the trigger for each.

| Surface | Status | Awaits |
| --- | --- | --- |
| application updates (`longhorn-update`, `longhorn-tauri-update`) | held-for-consumer | the update renderer runway (g02.009) |
| licensing and activation (`longhorn-licence`) | held-for-consumer | the licensing renderer runway (g02.010) |
| bridge supervision and contract machinery | held-for-consumer | a bridge consumer with a real service topology |
| age encryption (`longhorn-config-age`) | held-for-consumer | a consumer that needs encrypted backups |
| fork-tree history (`longhorn-history-tree`, `@longhorn/history-tree`) | planning-evidence | the recorded g01.017 adoption decision |

## Add One System At A Time

| Need | Pure/client selection | Add only when needed |
| --- | --- | --- |
| config, backup, restore | `longhorn-config`, `@longhorn/config` | config Tauri host, Poodle recovery pages |
| settings registry and shell | `longhorn-settings`, `@longhorn/settings` | config apply units, Tauri host, `/svelte`, `/poodle` |
| display/window hosting | `longhorn-display`, `longhorn-windowing` | config persistence and Tauri window adapter |
| registered regions/panels | `longhorn-layout`, `@longhorn/layout` | config persistence, Svelte, Poodle bindings |
| optional Surfaces | `longhorn-surfaces`, `@longhorn/surfaces` | persistence and window-host projection |
| panel transfer | `longhorn-transfer`, `@longhorn/transfer` | Tauri transfer host and Svelte/Poodle interaction |
| whole-Surface transfer | Surface plus `longhorn-surface-transfer` | `surface-transfer` host feature and renderer subpath |
| commands and keymaps | `longhorn-command`, `@longhorn/commands` | config, settings, Tauri, `/svelte`, `/poodle` |
| linear history | `longhorn-history`, `@longhorn/history` | Tauri metadata, `/svelte`, `/poodle`; product journal stays local |
| optional service bridge | `longhorn-bridge`, `@longhorn/bridge` | Tauri, events, or stream subpaths |
| finite operations | `longhorn-operation`, `@longhorn/operation` | Tauri, bridge, Svelte, or Poodle edges |
| retained notifications | `longhorn-notifications`, `@longhorn/notifications` | Tauri, Svelte, Poodle, or operation observation |
| native-content island | `longhorn-native-content`, `@longhorn/native-content` | select exactly one host mechanism and optional Svelte lifetime |

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
