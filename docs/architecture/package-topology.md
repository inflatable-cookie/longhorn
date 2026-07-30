# Package Topology

Status: promoted  
Owner: Tom  
Updated: 2026-07-30
Contract: `../contracts/012-distribution-and-compatibility.md`

## Repository Shape

```text
crates/       Rust domain libraries and host adapters
packages/     TypeScript, Svelte, and Poodle packages
tools/        binding and repository tools
examples/     composition and artifact-install proofs
fixtures/     cross-language and consumer-neutral fixtures
```

Cargo and npm workspaces share release metadata. Effigy owns repository task
discovery and validation entry points.

## Rust Packages

| Package | Responsibility | Depends on |
| --- | --- | --- |
| `longhorn-core` | ids, revisions, typed units, geometry, errors | none |
| `longhorn-config` | domains, roots, coordinated safe writes, backup, restore | core, `cap-std`, `fs4` |
| `longhorn-config-age` | optional authenticated binary age v1 backup envelopes | config, exact `age` adapter |
| `longhorn-display` | known/observed displays and correlation | core |
| `longhorn-windowing` | placement, desired/apply plans, pure event attribution and settling | core, display |
| `longhorn-windowing-config` | consumer-domain placement staging and coordinated flush | core, config, tauri-windowing |
| `longhorn-layout` | containers, regions, panels, normalization | core |
| `longhorn-layout-config` | registered layout domains, coordinated mutation, debounce, and flush | core, config, layout |
| `longhorn-surfaces` | optional Surface identity, topology, lifecycle, and pure host resolution | core |
| `longhorn-surfaces-config` | registered Surface domains and coordinated lifecycle publication | core, config, layout, surfaces |
| `longhorn-surface-windowing` | optional pure Surface/window plan projection and ordered shutdown | core, surfaces, windowing |
| `longhorn-transfer` | bounded sessions, leased targets, deterministic target resolution, and same-document panel commit | core, config, layout, layout-config |
| `longhorn-surface-transfer` | optional whole-Surface transfer and receipted provision coordination | core, config, layout, surfaces, surfaces-config, transfer |
| `longhorn-settings` | optional sealed settings registry, authority protocol, snapshots, commands, and receipts | core |
| `longhorn-settings-config` | registered config-domain apply units plus storage, backup, restore, and recovery modules | core, config, settings |
| `longhorn-command` | optional sealed command/context registry, admission, keyboard/keymap resolution, search, and projections | core |
| `longhorn-command-config` | registered active-preset and sparse-override domain plus coordinated mutation | core, config, command |
| `longhorn-command-settings` | optional keybinding page registration over command/keymap capabilities | core, settings |
| `longhorn-history` | optional typed linear history, checked navigation, persistence envelopes, projections, and transition receipts | core |
| `longhorn-bridge` | exact-v1 bridge identity, authority-gated lifecycle, generic operation/reply, bounded retry/deduplication, ordered projection, optional job metadata, and feature-gated injected supervision | core |
| `longhorn-tauri-bridge` | narrow registered-domain handler assembly over the generic bridge protocol | core, bridge, Tauri plus adapted domains |
| `longhorn-tauri-config` | Tauri platform-path mapping plus injected storage, backup, restore, and recovery handlers | config, Tauri |
| `longhorn-tauri-windowing` | checked Tauri observation, managed identity, native mutation, lifecycle, capture, reveal, and flush | core, display, windowing, Tauri |
| `longhorn-tauri-transfer` | managed-window projection and transfer handler assembly | core, transfer, tauri-windowing, Tauri |
| `longhorn-tauri-settings` | settings command/event handler assembly over injected authorities | core, settings, Tauri |
| `longhorn-tauri-command` | command catalogue/keymap query, preview, and mutation assembly; no generic execution | core, command, command-config, Tauri |
| `longhorn-tauri-history` | registered metadata query and navigation assembly over injected history authorities | core, history, Tauri |
| `longhorn-bindings` | checked TypeScript generation | publishable domains |

The bindings tool is development-only. Additional adapter crates stay narrow;
there is no all-capabilities `longhorn-tauri` crate.

Implemented generator slices cover config operations, settings, layout,
Surface, transfer, optional Surface-transfer, and bridge protocols with
Rust-produced golden fixtures.

## TypeScript Packages

| Package | Responsibility |
| --- | --- |
| `@longhorn/core` | structural transport, checked subscription lifetime, and shared framework-neutral utilities |
| `@longhorn/config` | checked storage-layout, backup, restore, and recovery client |
| `@longhorn/windowing` | window snapshots and placement client |
| `@longhorn/layout` | generated layout snapshots, commands, receipts, and framework-neutral helpers |
| `@longhorn/surfaces` | optional Surface protocol |
| `@longhorn/transfer` | session, lease, target, and panel-transfer clients |
| `@longhorn/surface-transfer` | optional whole-Surface transfer client |
| `@longhorn/commands` | optional checked command/keymap clients and `/svelte` plus `/poodle` projections |
| `@longhorn/history` | optional history client |
| `@longhorn/bridge` | checked bridge session, topology, authority, lifecycle, retry, and conformance runtime with optional `/supervision` |
| `@longhorn/tauri` | domain-free raw invoke transport plus optional `/events` listen edge |
| `@longhorn/svelte` | reactive client state, actions, and optional capability subpaths |
| `@longhorn/poodle` | public Tabs, DockRegion, and SplitView bindings |
| `@longhorn/settings` | optional settings protocol, client, session state, and Svelte/Poodle shell subpaths |

Names are working names until registry verification. Packages appear only
when their milestone implements a usable slice.

The implemented TypeScript packages remain private until registry authority
is verified. `@longhorn/tauri` contains only the raw Tauri transport adapter;
domain behavior remains in the framework-neutral packages.
`@longhorn/bridge` adds no product payload vocabulary. Its checked session,
strict compatibility layer, direct adapter, deterministic serialized
loopback, and optional stream subpath compose with domain-owned generated
codecs and clients. Its `/tauri` subpath uses the invoke-only transport;
`/tauri-events` adds listener-first resync and correlated job events.
Injected-clock connection and bounded query-retry controllers remain in the
framework-neutral root. `/supervision` is the only service runtime: it accepts
consumer-supplied operations and opaque credential references, rejects
external lifecycle ownership, and imports no production transport.
`@longhorn/svelte` now provides the per-instance reactive lifecycle, generic
optimism, consumer-fed layout state, and optional domain adapters.

The implemented Svelte root is Surface-free. Optional layout, Surface,
transfer, and Surface-transfer bindings are subpath exports backed by optional
peers, not root re-exports. The implemented private Poodle root is also
Surface-free and owns public Tabs, DockRegion, and SplitView integration.
It is tested against the exact Card 038 preview artifact. A published peer
range remains a release gate, not an inferred promise from sibling source.

`@longhorn/settings` keeps its framework-neutral root UI-free. Its optional
`/svelte` subpath owns per-instance settings sessions; `/poodle` composes one
modal, window, or panel shell over the same controller. Both use optional
peers, and the Poodle shell is verified against the exact Card 038 artifact.
Consumer renderer snippets retain product schemas, validation, and copy.

`@longhorn/commands` keeps its framework-neutral root independent of Tauri,
Svelte, Poodle, settings, and bridge. It accepts injected catalogue, keymap,
availability, and executor ports. `/svelte` owns per-instance reactive state;
`/poodle` binds public controlled palette and settings primitives. Command
selection remains consumer dispatch. The package contains no generic product
execution transport. Clean artifact proof keeps Jetstream on the root alone
while Loophole selects Svelte, Poodle, settings, config, and Tauri edges
explicitly.

`@longhorn/history` keeps its framework-neutral root independent of config,
bridge, Tauri, Svelte, Poodle, and consumer payloads. It carries checked
metadata snapshots, paged past/current/future entries, navigation commands,
receipts, and errors. `/svelte` owns per-instance reactive state. `/poodle`
composes a controlled linear history panel from public primitives. Product
payloads never cross the generic renderer protocol.

`longhorn-history` accepts typed payload policy and an atomic consumer apply
transaction. Structural persistence and committed transition receipts are
shared; canonical product snapshots, journal files, checkpoints, fsync,
autosave, replay, and project versions remain consumer authority. No branch
package exists before the g01.011 prototype decision.

`@longhorn/config` keeps its framework-neutral root independent of settings,
Svelte, Poodle, and Tauri. Its optional `/poodle` subpath exposes storage,
backup, restore, and recovery pages over public Poodle primitives. The
settings root does not import it. `longhorn-settings-config` registers its
renderer keys and admits each page only when the matching base capability is
composed.

Config-operation commands carry built-in profile choices, inventoried archive
digests, explicit restore choices, and host-issued confirmation digests.
Executable transition, retention, and restore plans, portable-root and export
pickers, restore archive selection and unlock, pending-publication
coordination, encryption identities, filesystem capabilities, journals, and
committed receipt custody stay in injected host authority.

Checked conformance artifacts cover Loophole and Nucleus shapes without
placing donor host identity or product payload in either package.

The bridge artifact proof installs five isolated TypeScript consumers from
packed archives and compiles separate query-only and full-host Rust graphs
from private source inventories. It proves exact subpath imports, Tauri
permissions, protocol compatibility, adapter parity, retry bounds, lifecycle
invalidation, opaque credential references, and product-neutral payload and
authority seams. See
[Bridge Topology Composition](bridge-topology-composition.md).

## Dependency Rules

```text
core
├─ config
├─ display ─ windowing
├─ layout ─ layout-config ─ config
├─ surfaces ┬─ surfaces-config ─ config
│           └─ surface-windowing ─ windowing
├─ transfer ─ surface-transfer
│    ├─ windowing
│    └─ layout-config
├─ settings ┬─ settings-config ─ config
│           └─ command-settings
├─ command ─ command-config
│                   └─ config
├─ history
└─ bridge

domain packages -> narrow host adapters -> Svelte/Poodle presentation
```

- Arrows never point from a pure domain into a host or UI adapter.
- Optional packages cannot become dependencies of their foundations.
- Generated bindings live in the TypeScript package for their Rust domain.
- Poodle, Svelte, and Tauri remain peer dependencies at adapter edges.
- Optional adapter subpaths use optional peers and cannot leak into root
  resolution.
- Poodle drag integration uses public extension points only.
- Product schemas and panel bodies never enter this graph.
- Bridge session metadata never becomes an untyped product command bus.
- Command ids do not become bridge operation names. Consumer executors map an
  admitted command to renderer-local behavior or one typed domain operation.
- The command root imports no config, Tauri, settings, bridge, Svelte, or
  Poodle package.
- The history root imports no config, bridge, Tauri, async runtime, Svelte,
  Poodle, or consumer payload package.
- History payload policy and atomic product apply remain injected. Generic
  renderer messages contain metadata, never payloads.
- Forkable history remains a non-publishable prototype until contract 008's
  promotion gate passes.
- Service supervision and production network transports are optional adapter
  edges. Removing them leaves direct and Tauri-local compositions intact.
- The settings root imports no layout, Surface, command, history, backend,
  Svelte, or Poodle package.
- `longhorn-settings-config` binds one built-in apply unit to one config domain;
  broader atomicity requires an explicit consumer transaction authority.
- Settings Svelte and Poodle entry points remain optional subpaths with peer
  dependencies. Removing settings removes their graph.

## Examples

Examples arrive with the capability they prove:

- minimal configuration and shell
- `app-shell-proof`: isolated Bovine, Nucleus, and Loophole artifact installs
- `settings-composition-proof`: isolated Bovine, Soundcheck, Loophole, and
  Nucleus settings installs
- `bridge-topology-proof`: isolated Bovine, Jetstream, Soundcheck, Nucleus,
  and Loophole bridge installs plus query-only/full-host Rust graphs
- `nucleus-no-surface-proof`: window-bound workspace without Surface packages
- `tauri-transfer-proof`: direct and optional full Surface transfer hosting
- optional local or remote service
- installation from produced release artifacts

Local path dependencies are proof inputs only. The settings verifier rewrites
them to produced archives in clean temporary roots. Consumer migration targets
published prerelease artifacts with exact lockfile resolution.
