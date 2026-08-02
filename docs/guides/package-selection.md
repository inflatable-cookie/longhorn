# Choose Packages

Status: checked private adoption guidance
Updated: 2026-08-02
Governing contracts: [002](../contracts/002-composable-workspace-hosting.md),
[007](../contracts/007-optional-backend-topology.md),
[012](../contracts/012-distribution-and-compatibility.md), and
[013](../contracts/013-svelte-and-poodle-adapter-lifecycle.md)

## Rule

Start with authority the app needs. Select its pure package, then only the
storage, Tauri, Svelte, Poodle, or service adapters required to expose it.
There is no umbrella package. A visual similarity to another app is not a
dependency reason.

All names below are working private `0.1.0` names. Use the exact produced
artifacts and locks from the active private distribution receipt. Do not run a
registry install command.

## Proven Starting Shapes

| Shape | Use when | Hierarchy | TypeScript packages | Rust packages | Excluded by proof |
| --- | --- | --- | --- | --- | --- |
| minimal | preferences and a centralized settings host are enough | window | 4 | 6 | layout, Surface, bridge, history, native content |
| workspace | one or more windows need registered regions and panels | window → layout container → region → panel | 8 | 16 | Surface, transfer, bridge, history, native content |
| full hosting | content can move through displays, windows, Surfaces, regions, and panels | display → window → Surface → layout container → region → panel | 12 | 24 | bridge, native content, operations, notifications |
| optional server | local config/settings remain authoritative while a service is optional | window plus optional bridge | 5 | 8 | layout, Surface, history, native content |

The [Card 125 matrix](../architecture/greenfield-composition-matrix.md) records
the exact graph. Copy package selection from that shape’s `package.json` and
`src-tauri/Cargo.toml`; replace the neutral app identity and product policy.
`capability.json` is selection evidence, not a universal Tauri capability.

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
| optional service bridge | `longhorn-bridge`, `@longhorn/bridge` | Tauri, events, stream, or supervision subpaths |
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
