# Command System Composition

Status: promoted  
Owner: Tom  
Updated: 2026-07-30  
Contracts: `../contracts/004-configuration-storage-backup-and-recovery.md`,
`../contracts/005-settings-and-system-registration.md`,
`../contracts/006-command-action-and-input.md`,
`../contracts/010-rust-typescript-ipc-and-events.md`,
`../contracts/012-distribution-and-compatibility.md`, and
`../contracts/013-svelte-and-poodle-adapter-lifecycle.md`

## Selection Rule

Start with the semantic catalogue and fresh executor admission. Add durable
keymaps, Tauri hosting, Svelte state, Poodle rendering, and settings
registration only where the app needs them.

| Need | Rust | TypeScript | Capability |
| --- | --- | --- | --- |
| Registry, search, keyboard | `longhorn-command` | `@longhorn/commands` | none |
| Durable sparse overrides | add `longhorn-command-config` | same root client | config domain authority |
| Catalogue/keymap Tauri host | add `longhorn-tauri-command` | injected ports | command read; mutate when writable |
| Per-window reactive state | same Rust graph | add `/svelte` | listen/unlisten when host emits hints |
| Palette and keybinding UI | same Rust graph | add `/poodle` | no grant from visibility |
| Settings navigation | add `longhorn-command-settings` | add `@longhorn/settings` | catalogue plus writable-keymap composition |

The `@longhorn/commands` root has no Longhorn dependency and keeps Svelte and
Poodle as optional peers. Importing the root does not install config,
settings, Tauri, bridge, Svelte, or Poodle.

## Proven Shapes

| Shape | Contexts | Rust graph | TypeScript graph | UI/settings |
| --- | --- | --- | --- | --- |
| Jetstream | `global` | command, core | commands | none |
| Loophole | global → project → surface → region → panel | command, config, settings, command adapters, Tauri adapter | commands, core, settings | Svelte session, public Poodle palette/keybindings |

These are composition proofs. They do not prescribe donor catalogues, context
facts, command labels, product permissions, routes, or UI framing.

Nucleus can choose a middle shape: global plus workspace/editor contexts,
durable overrides if useful, and no Surface contexts. The command system does
not require the display → window → Surface → region → panel hierarchy.

## Authority And Execution

The host seals one registry generation. Product code owns:

- command and context declarations
- current context and capability facts
- dynamic availability
- authorization
- local renderer actions
- typed domain operations and terminal receipts

Longhorn validates identity, arguments, generation, current context,
capabilities, and fresh availability. It then calls an injected executor with
the admitted semantic invocation and opaque route. A command ID is not a
Tauri command name, bridge operation name, or generic execute-by-string
endpoint.

Local and typed-domain routes are peers. A panel-close command can map to
renderer-local behavior. A transport command can map to a typed transport
operation. Both retain the admitted command ID and request correlation.

## Catalogue And Projection

Use one catalogue generation, availability snapshot, and effective keymap for
palette, keyboard, menus, help, and keybinding settings.

1. Register contexts, capabilities, commands, and immutable presets.
2. Seal the registry and compile the effective keymap.
3. Load renderer listeners before authoritative snapshots.
4. Join catalogue, keymap, and availability only when generations agree.
5. Project each discovery surface from that joined state.
6. Revalidate fresh native facts before execution.

Projection visibility is not authorization. A visible or enabled renderer
record cannot bypass fresh host admission.

## Keymap Rules

Presets are immutable. Persist only the active preset ID and sparse disable,
replace, or add directives through `longhorn-command-config`.

- preview binds registry generation, keymap revision, preset version, and
  canonical patch digest
- commit accepts only exact preview evidence
- equal-specificity distinct invocations are conflicts
- failed persistence leaves the prior effective keymap installed
- recovery and unavailable source postures remain explicit
- renderer refresh ignores older availability or keymap revisions

V1 resolves one physical key chord on key press. Repeat, IME composition,
editable focus, reserved chords, capture, conflicts, and unbound input have
explicit non-dispatch postures.

## Svelte, Poodle, And Settings

Create one `CommandSession` per mounted window or panel. Stop it on unmount so
keyboard and host listeners are removed. Do not place session state in a
module singleton.

`@longhorn/commands/poodle` uses public controlled Poodle APIs. Longhorn owns
projection and interaction binding; Poodle owns visual primitives. The
consumer still owns the application shell, labels, category presentation,
icon resolution, and renderer placement.

Register `longhorn-command-settings` only when the host composes both the
sealed catalogue and writable-keymap capabilities. The settings page owns no
configuration scope or apply unit. Keymap mutation stays with the command
authority.

## Tauri Capability Posture

A read-only host grants:

- `allow-longhorn-command-read`
- event listen/unlisten only when invalidation events are consumed

A writable keybinding host adds
`allow-longhorn-command-mutate`. Execution is absent from the shared Tauri
handler. Product operations use their own typed authority.

## Migration From Existing Apps

Conformance proves the shared pieces can express an app shape. Cutover is a
separate consumer migration.

1. Inventory semantic commands, context facts, shortcuts, menus, palette
   entries, help entries, authorization, and execution routes.
2. Assign stable command, context, capability, preset, and binding IDs.
3. Build the sealed registry beside the current system without moving product
   authority.
4. Map old local actions and domain calls to injected typed executors.
5. Run discovery, focus, conflict, stale-fact, and execution traces against
   both systems.
6. Import existing shortcuts into sparse overrides through an explicit
   versioned migration. Preserve old files until durable publication is
   receipted.
7. Move palette, menu, help, and keyboard projections to one joined state.
8. Add optional Svelte/Poodle/settings edges only after core parity.
9. Narrow Tauri permissions.
10. Remove the old registry and keymap only after restart, recovery, and
    rollback evidence passes.

Do not keep a silent fallback between old and new command authorities. A
failed migration leaves the existing system selected.

## Artifact Evidence

`proof:command-system-artifacts` packs `@longhorn/core`,
`@longhorn/settings`, and `@longhorn/commands`, verifies the exact Poodle
artifact set, and installs clean Jetstream and Loophole consumers. It rejects
workspace aliases, sibling source resolution, optional-edge drift, duplicate
Svelte runtimes, capability drift, donor payloads, and generic execution
buses.

Private Rust crates are inventoried with
`cargo package --list --allow-dirty`, archived, unpacked into a clean
workspace, and run offline. Jetstream resolves only `longhorn-command` and
`longhorn-core`. Loophole selects config, settings, command settings, and
Tauri adapters explicitly. Registry-normalized Rust publication remains a
release-lane gate while the interdependent crates are private.

## Retained, Changed, Rejected, Deferred

| Class | Result |
| --- | --- |
| retained | product catalogues, contexts, availability, authorization, local actions, typed domain operations, labels, icons, shell placement |
| changed | registry sealing, search, physical keyboard resolution, sparse override persistence, checked joins, lifecycle teardown, shared projections |
| rejected | generic execute-by-string bus, renderer-owned authority, binding-order conflict winners, silent persistence fallback, donor payloads in shared packages |
| deferred | donor cutover, native accelerators, global hotkeys, multi-stroke sequences, macros, extended triggers, automation, live plugins, synchronized keymaps |

## Proof

Run:

```sh
effigy proof:command-system-artifacts
```

Sources live in `examples/command-system-proof/`. Closeout evidence is in
`../logs/2026-07/30-command-system-artifact-proof-and-closeout.md`.
