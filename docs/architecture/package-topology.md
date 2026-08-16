# Package Topology

Status: promoted  
Owner: Tom  
Updated: 2026-08-03
Contract: `../contracts/012-distribution-and-compatibility.md`

## Repository Shape

```text
crates/       Rust domain libraries and host adapters
packages/     TypeScript, Svelte, and Poodle packages
examples/     composition and artifact-install proofs
fixtures/     cross-language and consumer-neutral fixtures
```

Cargo and npm workspaces share release metadata. Effigy owns repository task
discovery and validation entry points.

## Rust Packages

| Package | Responsibility | Depends on |
| --- | --- | --- |
| `longhorn-core` | ids, revisions, typed units, geometry, errors | none |
| `longhorn-config` | domains, roots, coordinated safe writes, backup, ordinary restore, grouped custom-adapter restore, boot recovery | core, `cap-std`, `fs4` |
| `longhorn-config-age` | optional authenticated binary age v1 backup envelopes | config, exact `age` adapter |
| `longhorn-display` | known/observed displays and correlation | core |
| `longhorn-windowing` | placement, desired/apply plans, pure event attribution and settling, host-agnostic capture/flush model and placement port | core, display |
| `longhorn-windowing-config` | consumer-domain placement staging and coordinated flush | core, config, windowing |
| `longhorn-surfaces` | optional Surface identity, topology, lifecycle, pure host resolution, and the layout engine (regions, panels, normalization); a Surface is the layout | core |
| `longhorn-surfaces-config` | registered Surface and layout domains, coordinated lifecycle publication, layout mutation, debounce, and digest | core, config, surfaces |
| `longhorn-surface-windowing` | optional pure Surface/window plan projection and ordered shutdown | core, surfaces, windowing |
| `longhorn-transfer` | bounded sessions, leased targets, deterministic target resolution, and same-document panel commit | core, config, surfaces, surfaces-config |
| `longhorn-surface-transfer` | optional whole-Surface transfer and receipted provision coordination | core, config, surfaces, surfaces-config, transfer |
| `longhorn-settings` | optional sealed settings registry, authority protocol, snapshots, commands, and receipts | core |
| `longhorn-settings-config` | registered config-domain apply units plus storage, backup, restore, and recovery modules | core, config, settings |
| `longhorn-command` | optional sealed command/context registry, admission, keyboard/keymap resolution, search, and projections | core |
| `longhorn-command-config` | registered active-preset and sparse-override domain plus coordinated mutation | core, config, command |
| `longhorn-command-settings` | optional keybinding page registration over command/keymap capabilities | core, settings |
| `longhorn-history` | optional typed linear history, checked navigation, persistence envelopes, metadata protocol, projections, and transition receipts | core |
| `longhorn-history-tree` | optional immutable-node fork topology, stable branch refs, atomic LCA navigation, protected pruning, opaque checkpoints, and dense strict persistence | core, history |
| `longhorn-tauri-history-tree` | optional caller-aware Tauri host for bounded tree metadata and navigation | history-tree, Tauri |
| `longhorn-operation` | optional finite operation authority with bounded progress, cancellation, retention, retry lineage, and teardown | core |
| `longhorn-notifications` | optional finite retained notification ledger with explicit read/removal transitions and operation observation | core; optional operation feature |
| `longhorn-native-content` | pure desired/observed native-content state, generation, planning, proposals, and receipts | core |
| `longhorn-tauri-native-content-child-view` | isolated Tauri child-webview construction, native operation execution, and policy-admitted generation-checked navigation | core, native-content, Tauri |
| `longhorn-native-content-isolated-window` | generic process-isolated content coordination, bounded helper protocol, and injected lifecycle ports | core, native-content, serde |
| `longhorn-native-content-backing-surface` | generic full-host storage evidence, viewport clipping, renderer lifecycle, and physical input admission | core, native-content, serde |
| `longhorn-bridge` | exact-v1 bridge identity, authority-gated lifecycle, generic operation/reply, bounded retry/deduplication, ordered projection, optional job metadata, and feature-gated injected supervision | core |
| `longhorn-licence` | optional pure licence policy: opaque entitlements, independent use/update windows, trust basis, lease and grace, clock-regression refusal, Ed25519 verification | core, ed25519-dalek, serde |
| `longhorn-credential-keyring` | opt-in host-agnostic platform keychain backend for built-in and consumer-scoped credentials; locked is not absent | core, `keyring` |
| `longhorn-update` | optional pure update policy: channels, semver comparison, client-side staged rollout, mandatory-version floor, deferral, the restart interlock (gate plus quiescence probes), and installation-provenance classification | core, semver, sha2 |
| `longhorn-update-install` | Longhorn's update installer, one implementation for every host: minisign verification, bounded extraction, atomic replacement, injected escalation, and the installation-provenance probe | update, minisign-verify, tar, flate2 |
| `longhorn-browser` | host-agnostic system browser launch for contract 019's RFC 8252 flow: allowlisted URL validation plus a launcher that spawns a program directly and never a shell | serde |
| `longhorn-poodle` | projects Longhorn's domains into `poodle-specs`, the shared renderer-neutral contract Poodle's adapters consume | core, config, licence, notifications, operation, settings, update, `poodle-specs` |
| `longhorn-tauri-bridge` | narrow registered-domain handler assembly over the generic bridge protocol | core, bridge, Tauri plus adapted domains |
| `longhorn-tauri-config` | Tauri platform-path mapping plus injected storage, backup, restore, and recovery handlers | config, Tauri |
| `longhorn-tauri-windowing` | checked Tauri observation, managed identity, native mutation, lifecycle, capture, reveal, and flush | core, display, windowing, Tauri |
| `longhorn-tauri-transfer` | managed-window projection and transfer handler assembly | core, transfer, tauri-windowing, Tauri |
| `longhorn-tauri-settings` | settings command/event handler assembly over injected authorities | core, settings, Tauri |
| `longhorn-tauri-command` | command catalogue/keymap query, preview, and mutation assembly; no generic execution | core, command, command-config, Tauri |
| `longhorn-tauri-history` | registered metadata query and navigation assembly over injected history authorities | core, history, Tauri |
| `longhorn-tauri-licence` | narrow Tauri seam for the injected licence authority; credential material crosses inward only | core, licence, Tauri |
| `longhorn-tauri-operation` | read, manage, and cancel handlers over injected operation authority and executor ports | core, operation, Tauri |
| `longhorn-tauri-notifications` | bounded page and mutation handlers over an injected notification authority; app-wide invalidation hints | notifications, Tauri |
| `longhorn-tauri-update` | narrow Tauri seam for the injected update controller: commands, capability split, invalidation hint | core, update, Tauri |
| `longhorn-gpui-windowing` | GPUI host adapter: window create/destroy/observe, creation-time placement, lifecycle translation, synchronous close decision, quiescence probe, and refusal-typed display facts. Takes no `gpui` dependency — the backend is an injected seam, bound to real GPUI in `prototypes/gpui-windowing` | core, display, update, windowing |
| `longhorn-bindings` | checked TypeScript generation | publishable domains |

The bindings tool is development-only. Additional adapter crates stay narrow;
there is no all-capabilities `longhorn-tauri` crate.

A `tauri-*` prefix means a Tauri dependency, not a role. `longhorn-tauri-update`
carried the prefix without the dependency and was absorbed into
`longhorn-update` on 2026-08-09: its restart interlock is pure policy and
belonged with the rest of it. The name returned on 2026-08-12 with the
dependency to match — the crate now holds only the Tauri seam (commands,
capability split, invalidation hint) for the `UpdateController` that stays in
`longhorn-update`. `longhorn-update-native` became
`longhorn-update-install` in the same 2026-08-09 pass, because `-native` implied a
fallback to a plugin path that contract 018 no longer has.

Host adapters are per-backend and an application composes exactly one.
`longhorn-gpui-windowing` deliberately has no `gpui` dependency: `gpui` pulls
several hundred transitive crates and a Metal shader build, so binding it in
the workspace would tax every QA lane for one adapter. The seam it exposes is
bound to real GPUI by the excluded `prototypes/gpui-windowing` crate, which
is the same line Poodle draws between `packages/gpui/adapter` and
`packages/gpui/preview`.

Implemented generator slices cover config operations, settings, layout,
Surface, transfer, optional Surface-transfer, bridge, history, operation, and notification
protocols with Rust-produced golden fixtures.

## TypeScript Packages

Three published packages, grouped by peer requirement rather than by domain.
Card 164 collapsed the earlier eighteen; contract 012 records why the split
earned its keep on the Rust side and not here.

### `@inflatable-cookie/longhorn` — no peers

| Subpath | Responsibility |
| --- | --- |
| `/core` (also the root) | structural transport, checked subscription lifetime, and shared framework-neutral utilities |
| `/config` | checked storage-layout, backup, restore, and recovery client |
| `/layout` | generated layout snapshots, commands, receipts, and framework-neutral helpers |
| `/surfaces` | optional Surface protocol |
| `/transfer` | session, lease, target, and panel-transfer clients |
| `/surface-transfer` | optional whole-Surface transfer client |
| `/commands` | optional checked command and keymap clients |
| `/settings` | optional settings protocol, client, and registry projection |
| `/history`, `/history-tree` | checked metadata clients and bounded tree metadata clients |
| `/operation`, `/notifications` | checked finite-operation clients and the retained-ledger client |
| `/native-content` | checked native-content protocol with direct and serialized clients |
| `/bridge` | checked bridge session, topology, authority, lifecycle, retry, and conformance runtime, with optional `/bridge/stream` and `/bridge/supervision` |
| `/<domain>/protocol` | the generated types and compatibility guards alone, for consumers that want the wire contract without the client |

### `@inflatable-cookie/longhorn-poodle-svelte` — peers `svelte`, `@inflatable-cookie/poodle-svelte` (optional), `@inflatable-cookie/longhorn`

| Subpath | Responsibility |
| --- | --- |
| root, `/layout`, `/surfaces`, `/transfer`, `/surface-transfer` | reactive client state and actions |
| `/<domain>/svelte` | per-instance reactive controller for a domain |
| `/<domain>/poodle` | public-Poodle shell over that controller |
| `/poodle`, `/poodle/binding`, `/poodle/transfer` | public Tabs, DockRegion, and SplitView bindings |
| `/native-content` | per-instance native-content connection, supplied-element measurement, explicit policy, and teardown |

### `@inflatable-cookie/longhorn-tauri` — peers `@tauri-apps/api`, `@inflatable-cookie/longhorn`

| Subpath | Responsibility |
| --- | --- |
| root, `/events` | domain-free raw invoke transport plus the listen edge |
| `/bridge`, `/bridge-events` | invoke-only bridge transport; the events subpath adds listener-first resync and correlated job events |
| `/history`, `/history-tree`, `/notifications`, `/operation`, `/native-content` | per-domain host port construction, no domain logic |

A planned `@inflatable-cookie/longhorn-windowing` surface for window snapshots
and placement is not yet implemented.

Names are working names until registry verification. Packages appear only
when their milestone implements a usable slice.

The implemented TypeScript packages remain private until registry authority
is verified. `@inflatable-cookie/longhorn-tauri` contains only the raw Tauri transport adapter;
domain behavior remains in the framework-neutral packages.
`@inflatable-cookie/longhorn/bridge` adds no product payload vocabulary. Its checked session,
strict compatibility layer, direct adapter, deterministic serialized
loopback, and optional stream subpath compose with domain-owned generated
codecs and clients. `@inflatable-cookie/longhorn-tauri/bridge` uses the invoke-only transport;
`/bridge-events` adds listener-first resync and correlated job events.
Injected-clock connection and bounded query-retry controllers remain in the
framework-neutral root. `/supervision` is the only service runtime: it accepts
consumer-supplied operations and opaque credential references, rejects
external lifecycle ownership, and imports no production transport.
`@inflatable-cookie/longhorn-poodle-svelte` now provides the per-instance reactive lifecycle, generic
optimism, consumer-fed layout state, and optional domain adapters.

`@inflatable-cookie/longhorn/native-content` is generated from the pure Rust authority. Its root
owns listener-first connection, renderer epochs, bounded correlation, and
stale-result rejection over injected transports. `@inflatable-cookie/longhorn-tauri/native-content` maps only the
native-content protocol commands and event. Mechanism construction, browser
policy, plugin/GPU payloads, Svelte, and Poodle remain absent.

`@inflatable-cookie/longhorn-poodle-svelte/native-content` depends only on that checked client plus its
Svelte peer. One session owns one mounted connection, exact supplied-element
measurement, explicit scale and visibility/focus/input policy, serialized
desired updates, remount invalidation, and observer teardown. Poodle remains
consumer composition only; the package has no Poodle or private-DOM edge.

The implemented Svelte root is Surface-free. Optional layout, Surface,
transfer, and Surface-transfer bindings are subpath exports backed by optional
peers, not root re-exports. The implemented private Poodle root is also
Surface-free and owns public Tabs, DockRegion, and SplitView integration. It is
tested against the exact Card 038 preview artifact. A public peer range remains
a later release-lane gate, not an inferred promise from sibling source.

`@inflatable-cookie/longhorn/settings` keeps its framework-neutral root UI-free. Its optional
`/svelte` subpath owns per-instance settings sessions; `/poodle` composes one
modal, window, or panel shell over the same controller. Both use optional
peers, and the Poodle shell is verified against the exact Card 038 artifact.
Consumer renderer snippets retain product schemas, validation, and copy.

`@inflatable-cookie/longhorn/commands` keeps its framework-neutral root independent of Tauri,
Svelte, Poodle, settings, and bridge. It accepts injected catalogue, keymap,
availability, and executor ports. `/svelte` owns per-instance reactive state;
`/poodle` binds public controlled palette and settings primitives. Command
selection remains consumer dispatch. The package contains no generic product
execution transport. Clean artifact proof keeps Jetstream on the root alone
while Loophole selects Svelte, Poodle, settings, config, and Tauri edges
explicitly.

`@inflatable-cookie/longhorn/history` keeps its framework-neutral root independent of config,
bridge, Tauri, Svelte, Poodle, and consumer payloads. It carries checked
metadata snapshots, paged past/current/future entries, navigation commands,
receipts, and errors. `/svelte` owns per-instance reactive state. `/poodle`
composes a controlled linear history panel from public primitives. Product
payloads never cross the generic renderer protocol.

`longhorn-history` accepts typed payload policy and an atomic consumer apply
transaction. Structural persistence and committed transition receipts are
shared; canonical product snapshots, journal files, checkpoints, fsync,
autosave, replay, and project versions remain consumer authority.

`longhorn-history-tree` is a separate downward-only package. Cards 070-072 own
bounded branch identity and metadata, immutable single-parent nodes, canonical
child indexes, checked complete-state admission, divergent record, atomic LCA
navigation, current/named/pinned lineage protection, deterministic leaf
pruning, opaque checkpoint replay accounting, and a deterministic dense graph
envelope with independent structural/payload migration. The persistence API is
bytes-only and owns no storage or durability. Renderer clients and release
compatibility are not yet implemented.

Four produced-artifact greenfield roots prove direct package selection for
minimal config/settings, a Surface-free workspace, full hosting with linear
history, and an optional supervised service. The roots share no umbrella
package and make smaller Rust and TypeScript graphs an exact checked property.
See [Greenfield Composition Matrix](greenfield-composition-matrix.md).

## Operation And Notification Layers

Contracts 015-016 compile working package names `longhorn-operation`,
`longhorn-tauri-operation`, `@inflatable-cookie/longhorn/operation`,
`longhorn-notifications`, `longhorn-tauri-notifications`, and
`@inflatable-cookie/longhorn/notifications`. The pure `longhorn-operation` authority is
implemented through progress, cancellation, retention, retry lineage, and
teardown. `longhorn-tauri-operation` and `@inflatable-cookie/longhorn/operation` add generated
direct, serialized, Tauri, and optional bridge composition. The optional
`/svelte` subpath owns one controller lifetime per renderer instance.
`/poodle` maps the same state to public feedback primitives and accepts
consumer detail snippets.

The `longhorn-operation` root remains a pure transition authority. It
does not execute work, schedule queues, interpret product progress, persist a
durable scheduler, or require bridge. Direct, Tauri, and bridge-domain edges
must expose the same catalogue truth. `longhorn-bridge` job metadata may carry
one operation's correlation evidence but never owns the catalogue.

The implemented `longhorn-notifications` root is independently usable over
`longhorn-core`. It owns bounded records, explicit source/replacement identity,
unseen/seen state, dismiss/clear/prune receipts, count/weight retention, and
newest-first pages. The optional `operation` feature observes immutable
committed terminal receipts through consumer policy and publishes by durable
producer token. Operations never depend on notifications, and publication
failure has no operation mutation path. The checked notification client uses
listener-first reconciliation and bounded paging. Its per-instance Svelte
session keeps selection and transient toasts renderer-local. Public Poodle
`ToastHost` owns expiry timers; expiry never changes retained truth. Semantic
action references cross the wire as data and call an injected fresh-admission
executor at invocation. Native OS delivery is not part of the v1 graph.

The operation/notification artifact proof installs four isolated TypeScript
shapes from packed archives and compiles four offline Rust graphs from private
source inventories. It proves operation-only and notification-only minimality,
Soundcheck cancellation and late-progress invariance, Loophole direct/Tauri/
bridge parity, two-window isolation, remount, teardown, retained truth after
toast expiry, fresh action admission, and one public Poodle/Svelte runtime.
See [Operation And Notification Composition](operation-notification-composition.md).

## Promoted Native-content Coordination

Card 086 promotes a split production direction:

- `longhorn-native-content`: pure identity, generation, desired/observed
  state, planning, proposals, receipts, and stale-result rejection
- `longhorn-tauri-native-content-child-view`: Tauri child-view operations and
  injected consumer browser/security policy
- `longhorn-native-content-isolated-window`: generic content-area and
  process-boundary coordination over consumer-owned native content
- `longhorn-native-content-backing-surface`: generic clip, visibility, and
  input-gate coordination over consumer-owned native storage and rendering
- `@inflatable-cookie/longhorn/native-content`: generated checked client
- `@inflatable-cookie/longhorn-poodle-svelte/native-content`: per-instance lifecycle, viewport
  measurement, explicit gates, and teardown

These are g01.018 working production names, not released registry promises.
The mechanism crates depend downward on the pure kernel and never on each
other. The pure graph has no Tauri, browser, plugin, GPU, Svelte, or Poodle
edge. TypeScript depends on its owning Rust protocol. Svelte depends on the
framework-neutral client.

There is no Poodle-specific native-content package. Poodle retains panels,
overlays, and presentation. Consumers bind public Poodle layout to the Svelte
adapter through supplied elements and policy. `longhorn-windowing` retains
outer-window authority.

Cards 082-085 remain private evidence, not production source. Initial native
host support is macOS-only. Child-view Windows and Linux are unproved;
isolated-window and backing-surface Windows and Linux are unsupported. Live
native scale switching remains unproved for child-view and backing-surface.
Card 093 proves isolated production artifacts, exact optional graphs, and
three-shape Rust/renderer conformance. g01.014 Cards 094-101 compile the
Nucleus migration. Card 094 is the read-only behavior and policy freeze;
Card 095 admits the private artifact graph and both Nucleus consumer checks.
Cards 096-098 complete storage, protected-window, and project-layout authority
cutovers. Card 099 completes listener-first renderer state, public Poodle
composition, and explicit overlay geometry. Card 100 completes the native
Browser cutover through the child-view host, checked client, and Svelte
session. Nucleus retains browser policy and gains no Surface dependency.
Card 101 closes the migration with exact private artifacts, restart and
rollback drills, capability and duplicate-authority audits, and no Surface
edge in the Nucleus graph.
Soundcheck and Jetstream retain their sequential and consumer-authority gates. See
[Native-content Island Composition](native-content-island-composition.md).

## Optional History-tree Layer

Card 069 promotes a downward-only tree layer. `longhorn-history-tree`,
`longhorn-tauri-history-tree`, and `@inflatable-cookie/longhorn/history-tree` are now implemented
private-workspace packages. Card 074 proves their produced private artifacts;
none is published or implicitly enabled in a consumer.

The Rust tree crate will depend on `longhorn-core` and `longhorn-history` for
identity, typed payload policy, navigation steps, and rollback evidence. The
linear crate will not depend on it. Optional TypeScript, Tauri, Svelte, and
Poodle edges will expose metadata and bounded alternate projections only.
Product payloads, model apply, snapshot content, checkpoint content, storage,
and recovery remain consumer authority.

This boundary keeps minimal linear consumers unchanged. The retained private
prototype is evidence, not a workspace member or package source. The artifact
proof resolves document and Loophole-shaped consumers only from disposable
archives and rejects prototype or sibling-source resolution.

The history artifact proof installs minimal and Loophole-shaped TypeScript
consumers from packed archives and runs separate offline Rust graphs from
private source inventories. The minimal graph contains core and history only.
The rich graph adds the narrow Tauri adapter plus optional Svelte and Poodle
subpaths. Rust-produced metadata fixtures and isolated renderer traces match.
See [History Composition](history-composition.md).

`@inflatable-cookie/longhorn/config` keeps its framework-neutral root independent of settings,
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
├─ surfaces (layout engine) ─ surfaces-config ─ config
│    └─ surface-windowing ─ windowing
├─ transfer ─ surface-transfer
│    └─ surfaces-config
├─ settings ┬─ settings-config ─ config
│           └─ command-settings
├─ command ─ command-config
│                   └─ config
├─ history
├─ operation
├─ notifications
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
- The operation root imports no executor, scheduler, bridge, config, Tauri,
  async runtime, Svelte, Poodle, or consumer payload package.
- The notification root imports no operation, command, bridge, Tauri, async
  runtime, Svelte, or Poodle package. Optional observation and action execution
  compose at adapter edges.
- Operation cancellation receipts never claim terminal stop. Notification
  publication never changes an operation outcome.
- Forkable history remains a non-publishable prototype until contract 008's
  promotion gate passes.
- Service supervision and production network transports are optional adapter
  edges. Removing them leaves direct and Tauri-local compositions intact.
- The settings root imports no Surface, command, history, backend,
  Svelte, or Poodle package.
- `longhorn-settings-config` binds one built-in apply unit to one config domain;
  broader atomicity requires an explicit consumer transaction authority.
- Settings Svelte and Poodle entry points remain optional subpaths with peer
  dependencies. Removing settings removes their graph.

## Examples

Examples arrive with the capability they prove:

- minimal configuration and shell
- `app-shell-proof`: isolated Split-shell, Nucleus, and Loophole artifact installs
- `settings-composition-proof`: isolated Split-shell, Soundcheck, Loophole, and
  Nucleus settings installs
- `bridge-topology-proof`: isolated Split-shell, Jetstream, Soundcheck, Nucleus,
  and Loophole bridge installs plus query-only/full-host Rust graphs
- `history-system-proof`: isolated minimal and Loophole-shaped linear history
  installs plus pure and Tauri-hosted Rust graphs
- `operation-notification-proof`: isolated minimal operation, Soundcheck scan,
  Loophole render/notification, and notification-only installs
- `nucleus-no-surface-proof`: window-bound workspace without Surface packages
- optional local or remote service
- installation from produced release artifacts

Private consumers may use explicit sibling source links covered by exact clean
commit receipts. The settings verifier rewrites source links to produced
archives in clean temporary roots. Artifact claims come only from those clean
installs. Package-manager publication remains deferred to g01.016.
