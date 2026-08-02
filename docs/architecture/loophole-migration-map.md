# Loophole Migration Map

Status: frozen baseline; storage cutover complete; window cutover ready
Owner: Tom
Updated: 2026-08-01
Governing refs: contracts 002-014; `../roadmaps/g01/015-loophole-full-hosting-migration.md`

## Baseline

The audit used clean Loophole commit
`da08b50e7cc69b7d13636c94fc571a64db4ae8ca` on `main` and clean Poodle
commit `208532f0d18dcd1683cdef157e370d0ba0f0d3b3` on `main`.
Aura is the active Tauri/Svelte host. Pulse owns project and runtime truth.
Echo contains shared Loophole UI mechanisms. Chorus contracts are product
authority. The Aura and Pulse `reference/legacy-app` trees are cold evidence,
not active implementation.

`effigy health` passed Aura's Svelte check, then stopped before Rust validation
on one pre-existing Cargo graph conflict:

```text
pulse rusqlite 0.31 -> libsqlite3-sys 0.28
soundcheck rusqlite 0.40.1 -> libsqlite3-sys 0.38.1
```

Both native libraries claim `links = "sqlite3"`. Clean donor health is an
admission gate. The audit did not alter Loophole, Poodle, Soundcheck, Signal,
or any Cargo manifest.

## Product Hierarchy

The live hierarchy remains:

```text
display -> window -> Surface -> region -> panel
```

A Surface has one of two habitats:

- `regional`: the canonical eight-region layout
- `focused-panel`: one eligible singleton panel directly fills the Surface

The regional ids are `topStrip`, `bottomStrip`, `leftStrip`, `rightStrip`,
`left`, `right`, `centerTop`, and `centerBottom`. Echo uses snake-case aliases.
Those ids, their structural arrangement, presence rules, and product panel
eligibility remain Loophole registration policy.

Focused-panel promotion is one atomic product action: create an adjacent
Surface, retain the source placement as fallback, move the singleton panel,
and activate the new Surface. Closing it returns the panel to its fallback or
catalogue default. Fullscreen first moves the focused Surface through the
ordinary window/display path, then toggles transient native fullscreen. It is
not a second layout tree and is not persisted.

## Current Authority Map

| Concern | Current implementation | Longhorn target | Retained Loophole authority |
| --- | --- | --- | --- |
| path resolution | Longhorn profile through `echo-storage-profile`; receipted Aura import | configuration layout and transition | product identity, scope, legacy candidates, retention |
| configuration IO | Echo config/profile crates plus Aura/Pulse stores | registered domains, atomic mutation, backup/recovery | schemas, writer choice, server/project boundaries |
| display facts | Longhorn inventory and correlation fed by the Loophole machine helper | complete | machine labels, canonical platform evidence, and Loophole diagnostics |
| window plan/apply | Longhorn placement, protected/dynamic Tauri host, lifecycle, reveal, and shutdown | complete | logical roles, main retag, close, fullscreen, and empty-host policy |
| workspace document | Echo profile/layout types; Aura `shell.rs` | distinct registered window, Surface, and layout domains | workspace preset scope and project restore input |
| layout mutation | registered Longhorn layout domain with Aura compatibility projection | complete | eight-region schema and panel policy |
| Surface lifecycle | registered Longhorn Surface domain with focused attachment | complete | focused habitat, eligibility, presence, fallback policy |
| panel drag | public Poodle source/target and Longhorn armed sessions and leases | complete | allowed targets and product presentation |
| Surface drag | checked whole-Surface adapter; typed fallback to Aura empty-display spawn | complete | empty-display provision and source-window policy |
| renderer state | listener-first clients, epochs, leases, and Aura projection | transfer slice complete | bodies, labels, icons, chrome, workflow state |
| settings | Aura five-tab modal plus host stores | registry, checked sessions, shared shell and storage pages | app, hardware, keybinding, and workspace page content |
| commands | Echo registry/runtime/input/action crates plus Aura WASM | registry, keyboard keymap, palette and settings projections | command catalogue, availability, execution, extended input |
| history | Longhorn linear authority behind the Pulse adapter, canonical project envelope, Pulse transaction/journal, and paged Aura panel | complete | every payload, apply, journal, and durability/recovery decision |
| backend topology | embedded or remote Pulse composition | no g01.015 cutover | Pulse authority and transport lifecycle remain current |
| plugin/native hosts | Aura/Signal native windows | no g01.015 cutover | all current policy and mechanism remain current |

## Echo Disposition

Donor code is evidence, not automatically the shared contract. Generic Echo
mechanics move to Longhorn only after one vertical cutover passes.

### Replace with Longhorn mechanism

- `echo-os-paths`
- generic `echo-configuration` store and atomic-file mechanics
- generic portions of `echo-machine` and `echo-display-inventory`
- generic planning in `echo-windowing`
- generic identity, normalization, and mutation in `echo-ui-layout`
- generic keyboard/keymap, discovery, palette, and action-manager mechanics
- generic persisted linear history stack mechanics in `pulse-history`

### Split at the consumer edge

- `echo-profile-config`: keep Loophole workspace/profile schemas and adapters;
  remove generic path, mutation, and lifecycle authority
- `echo-ui-layout`: keep eight-region aliases, panel catalogue rules, focused
  eligibility, and Loophole repair/migration code
- command crates: keep product specs, contexts, availability, execution, and
  unsupported extended-trigger adapters
- `pulse-history`: keep the 83-variant payload, inverse/coalesce/no-op policy,
  codec, product transaction, labels, and project integration

### Retain outside this migration

- `echo-runtime-seams`, `echo-selection`, IPC codecs/transports, bootstrap
  policy, editorial model, and other Pulse/Aura/Spark product seams
- Signal and Soundcheck authorities
- Aura renderer composition and host-specific policy

The current Echo crates may be reorganized after cutover. The acceptance test
is removal of duplicate active mechanism, not deletion of every crate name.

## Storage Baseline And Gate

Aura's canonical app id is `com.inflatablecookie.loophole`. Chorus separately
chooses `Loophole` as the shared product storage root so Aura, Spark, embedded
Pulse, local Pulse, Signal, and helpers resolve the same files. This is a valid
use of Longhorn's opt-in stable storage identity:

```text
canonical app id: com.inflatablecookie.loophole
stable storage name: Loophole
```

It should not use the canonical id as its effective storage leaf. The stable
name is immutable app identity, not a user preference or display-name
derivation.

Current Echo behavior is close to the Chorus target but not to the Longhorn
transition contract. It uses a product root with `config`, `cache`, `state`,
and `logs` children, accepts `LOOPHOLE_USERDATA`, and imports old Aura data by
copying missing files. It has no fixed locator, durable journal, complete
conflict inventory, verification receipt, native database adapter, or
receipt-bound cleanup.

Card 103 selects one exact Chorus matrix and implements it as
`shared-product-root-v1`:

| Platform | Shared durable parent | Exact product root |
| --- | --- | --- |
| macOS | `Application Support` | `Application Support/Loophole` |
| Windows | roaming `%APPDATA%` | `%APPDATA%\Loophole` |
| Linux | `$XDG_DATA_HOME` | `$XDG_DATA_HOME/Loophole` |

The explicit stable name is exactly `Loophole` on every platform. No
platform-specific case normalization or display-name derivation applies. The
canonical id still fixes app and bootstrap-locator identity. The new injected
`shared-data` fact preserves the roaming/local distinction without changing
`platform-native-v1` or disguising the product choice as per-purpose
overrides.

After selection, migration must inventory every current Echo root and old
Tauri identifier root, use registered adapters, commit the canonical-id fixed
locator last, retain sources, and expose diagnostics. No dual-write or silent
old-path read survives cutover.

Card 105 completes that cutover. Aura, embedded Pulse, the local brokered Pulse
host, Spark, and Echo profile config use `echo-storage-profile`. The old
`echo-os-paths` crate is no longer an active workspace member or dependency.
Existing `Loophole` roots adopt in place. The old canonical-id Tauri root is
eligible only when the established product root has no recognized durable
domain. Sources and unknown files remain retained.

User config and machine state now have separate live roots. Project files,
Pulse journals/autosaves/media, remote server data, and Soundcheck's SQLite
database stay outside Loophole app-profile authority. Five durable renderer
preference keys import once into a registered Longhorn domain; `localStorage`
is retained only as the explicit legacy source during first host hydration.

Aura exposes a diagnostic projection containing canonical identity, stable
name, selected profile, selection origin, effective leaf, root paths and
provenance, warnings, layout digest, locator, and transition receipt.

## Window And Surface Cutover

The current pure Aura planner retags `main`, creates `workspace-*` windows,
moves them, and closes stale secondary windows. It uses a three-second
programmatic suppression window, a sliding five-second user-move window, and
a 300 ms geometry debounce. Native application happens in
`bootstrap/windows.rs`.

Longhorn replaces those heuristics with explicit apply generations, current
readback, bounded settling, attributed events, flush receipts, hidden reveal,
and exact partial-failure evidence. Loophole retains:

- `main` as the protected boot host and its logical retag rule
- dynamic workspace window roles and titles
- moving the last Surface disables the source window
- display adoption and empty-display provision policy
- focused-Surface fullscreen policy
- plugin editor and other non-workspace native windows

Window placement, Surface topology, and region/panel layout become distinct
registered domains. A mutation in one cannot rewrite the other two. Pulse's
saved project shell remains authoritative restore input; Longhorn and the
Loophole adapter validate and resolve it.

`AuraShellHostService` currently combines these domains in one large service
and performs best-effort whole-file persistence. A failed write is logged but
the method still returns the in-memory snapshot. Cutover must publish through
checked persistence before returning success.

Card 106 completes the display and workspace-window slice. Aura now owns a
registered `loophole.window-placement` machine-state domain separate from the
workspace document. Its first publication imports retained canonical display
facts and every per-display window geometry, then writes a digest-bound
receipt without deleting either source. Later capture writes only this domain.

One Longhorn host owns hidden restore, exact apply generations, fresh readback,
settling, capture, guarded reveal, user close, flush, and teardown for `main`
plus `workspace-*`. `main` remains the protected native slot and may change
logical identity without changing its transport label. Longhorn now moves
lifecycle identity with that retag. Plugin editor windows never enter the
managed registry or capability wildcard.

The Loophole machine helper remains a consumer platform-evidence provider. It
supplies canonical ids, built-in status, labels, and logical display facts;
Longhorn owns correlation, known-display retention, arrangement signatures,
placement resolution, and native lifecycle. `echo-configuration` is read only
during the one-shot import. `echo-windowing` remains active only for the shell
topology that Cards 107-108 replace. The old Aura planner, three/five-second
suppression windows, geometry timer, duplicate Tauri builder, and window
coordinator are gone.

## Renderer And Poodle

Current same-window panel movement uses Poodle's local
`application/x-poodle-panel-drag` payload and Aura-owned drag state. Current
whole-Surface cross-window movement reports a screen point to the host.

Poodle's current public `DockRegion` seam already supports
`externalDragSource` and `externalDropTarget`, including pointer-time prepare,
synchronous dragstart payload, explicit cancellation, and drop-time
revalidation. Longhorn can therefore use the public seam without private DOM,
MIME, portal, or component-state knowledge.

The target keeps Poodle local reorder for same-region presentation, but every
gesture advertised as cross-window uses a host-created Longhorn session.
Panels commit through one registered layout domain. Whole Surfaces retain
their layout-container binding. Optimism rolls back to the authoritative
receipt on failure.

Aura's density, size, and other durable preferences currently include
`localStorage` authority. Those values move to registered user-config or
settings domains. Renderer memory remains projection and transient UI state.

## Settings And Commands

Aura's modal currently exposes App, Appearance, Hardware, Keybindings, and
Workspace tabs. The migration adopts the Longhorn registry, per-instance
session, Poodle shell, storage/backup diagnostics, and checked config apply.
Loophole retains page renderers, hardware semantics, plugin policy, workspace
configuration, managed constraints, and activation behavior.

The command palette and keybinding UI already share Echo registry and search
semantics. Longhorn can replace the generic keyboard line while preserving
product command ids and the Aura execution adapter.

Longhorn contract 006 does not claim mouse button, wheel, gesture, playback,
MIDI, macro, native accelerator, or automation semantics. Loophole's input
contract does. g01.015 therefore migrates keyboard resolution, sparse
overrides, conflict explanation, palette search, and shortcut projection.
Extended triggers remain behind a named Loophole adapter until a later
Longhorn contract promotes them. Closeout must not claim the corresponding
Echo code is redundant.

## History Preservation

`PulseHistoryMutation` has 83 live variants. Pulse owns their fields,
inverse, no-op, coalescing meaning, runtime application, tempo/cache
reconciliation, labels, project versions, snapshots, autosave, and journal
recovery.

The current linear stack provides:

- full persisted undo and redo stacks
- default limit 100
- explicit grouping and automatic 750 ms grouping by key
- undo, redo, and position jump
- only eight recent applied entries in the renderer projection

The session journal is a project-adjacent JSONL file. It records mutation,
undo, and redo; clean save removes it; autosave retains the unabsorbed suffix;
recovery replays through Pulse apply and history. The journal is fenced to one
app version and intentionally does not fsync every entry.

Longhorn adoption is complete, linear, and structural:

1. register a Pulse-owned payload codec and inverse/coalesce/no-op policy;
2. import the complete applied and future ordering without replaying entries
   into the already canonical project snapshot;
3. preserve ids, labels, revisions, limits, groups, and retained baseline;
4. run undo, redo, and checkout through plan/apply/commit and one atomic Pulse
   transaction or verified rollback;
5. combine Longhorn transition receipts with the existing Pulse journal;
6. expose authoritative paged past and future metadata to Aura;
7. reject corrupt or future history visibly instead of silently returning an
   empty stack.

The canonical Longhorn envelope is stored beside a complete legacy rollback
projection. Canonical and legacy disagreement is rejected. Legacy-only state
imports directly without replay. Aura's active panel reads authoritative
paged metadata through the checked Tauri host. The old eight-entry
`PulseSessionSnapshot` remains only as a compatibility projection and
external-mutation invalidation signal; it is not an active history authority.

Branch mode stays disabled. g01.017 implementation and a later product
decision are both required before fork-tree adoption. Project versions remain
separate from undo branches.

## Cutover Order And Rollback

Each vertical slice starts from exact clean receipts, proves the selected
private artifact graph outside sibling resolution, preserves source data, and
ends with one active authority:

1. storage contract/profile and donor admission
2. storage transition and registered domains
3. display/window host
4. registered layout authority
5. Surface lifecycle and host binding
6. renderer/Poodle transfer
7. settings and keyboard/command shell
8. linear history
9. restart, rollback, duplicate removal, and packaged conformance

Rollback uses the previous app build plus recorded source, transition journal,
locator, and migration receipts. It does not mean dual-write or silent legacy
fallback. Source cleanup requires a separate receipt-bound action after the
replacement has survived restart and prior-build readback.

## Deferred Systems

The audit also found reusable-looking operation, notification, bridge,
supervision, plugin editor, and native-content seams. They remain outside
g01.015 because changing them is not required to transfer the workspace shell
authorities above. Their product adapters stay in Loophole and their later
adoption must use separate cards. Package-manager publication remains deferred.
