# 014 Command, Input, And Palette Boundary

Status: complete and promoted
Owner: Tom
Updated: 2026-07-30
Extends: `002-shared-desktop-systems-follow-up.md` and
`013-typed-bridge-and-backend-topology-boundary.md`

## Prompt

Revalidate the command, keymap, and palette boundary after the typed bridge,
configuration, settings, Svelte, and Poodle foundations are complete. Preserve
Loophole's useful depth without copying its product authority or forcing that
shape onto a smaller app.

## Evidence

### Longhorn

- The bridge owns session, correlation, ordering, retry, and topology
  structure. Domain packages own operation names, payloads, validation, and
  authority.
- The bridge must not become a product command bus.
- Configuration already owns registered, versioned domains, coordinated fresh
  mutation, migration, recovery, and exact durability receipts.
- Settings already admits optional pages by capability and supports
  consumer-owned renderers. It does not own product schema or command
  execution.
- Svelte state is per instance. Poodle owns the public `CommandPalette` and
  settings presentation primitives.

### Loophole Echo

- `echo-command-surface` separates command discovery metadata from execution.
- `echo-command-runtime` separates availability and execution planning, but its
  context, selection, arguments, and kernel routes are DAW-specific.
- `echo-input-resolution` supplies physical keyboard chords, platforms,
  contexts, presets, overrides, candidate reports, and extended trigger
  families.
- `echo-action-manager` adds reverse lookup, conflict reporting, palette
  search, shortcut labels, mutation, and macros.
- The four-crate split is useful evidence. Their product catalogue,
  selection hierarchy, preset set, and kernel planning are not shared
  contracts.
- The donor has two conflicting equal-rank rules: one resolver keeps earliest
  order while the action manager lets the last entry win. Longhorn must not
  preserve that ambiguity.

### Loophole Aura

- Palette search rejects stale async results and joins discovery with current
  availability.
- Keyboard routing suppresses text-input leakage and routes resolved commands
  through the same workspace command path.
- Keybinding settings expose capture, reverse lookup, conflict preview, and
  mutation results.
- Renderer wire types are handwritten and omit unsupported trigger families
  from display while retaining raw JSON.
- Some mutation paths update session state first and persist overrides
  best-effort. Persistence failure does not fail the mutation.
- Macros are renderer-orchestrated command sequences with delays. They can
  partially complete or race changing authority.

### Jetstream

- A small keyboard table resolves to the same command ids used by toolbar
  actions.
- It ignores repeat, suppresses shortcuts in editable targets, and consumes an
  event only after a binding resolves.
- It uses a semantic primary modifier: Command on macOS, Control elsewhere.
- It proves a second app needs the keyboard lane. It does not prove macros,
  MIDI, gestures, mouse bindings, release/repeat actions, or a rich context
  graph.

### Poodle

- Public Svelte `CommandPalette` props already cover controlled open/query
  state, items, selection, and shortcut labels.
- Longhorn should project command state into that component. It should not
  copy or fork the visual primitive.

## Donor Translation

| Concern | Retain | Change | Reject or defer |
| --- | --- | --- | --- |
| catalogue | one stable identity and discovery source | bounded sealed registry and typed metadata | product catalogue in Longhorn |
| arguments | structural argument declaration | closed validated v1 schema | arbitrary unchecked JSON |
| availability | current context projection and coded reasons | generation/revision evidence and fresh execution admission | renderer availability as authority |
| execution | one semantic command across surfaces | injected consumer route to local or typed domain operation | generic bridge or Tauri execute-by-string bus |
| keyboard | physical codes, canonical modifiers, context specificity | one normative resolver and semantic primary modifier | ambiguous equal-rank winner |
| overrides | read-only presets and sparse changes | stable binding ids, disable/replace/add directives, coordinated persistence | mutable base files and append-only shadow state |
| conflicts | candidate and conflict explainability | unresolved equal-rank conflict cannot persist or execute | “add anyway” with implicit winner |
| focus | text-input and palette ownership | explicit hot-context stack, repeat and IME gates | scattered target heuristics as authority |
| discovery | one search and shortcut-decoration engine | Rust semantics plus checked TypeScript conformance | client-specific ranking |
| UI | public Poodle palette and settings primitives | optional Svelte/Poodle adapters | copied visual components |
| macros | validation through commands | later authority and partial-receipt contract | renderer delay orchestration in v1 |
| extended input | shared trigger concept | extension seam after a second consumer | MIDI, gesture, mouse, playback in v1 |

## Boundary Decisions

### Registry and context

- `longhorn-command` owns bounded ids, registry construction, validation,
  sealing, deterministic generation/digest, projection, search, and input
  resolution.
- Apps and optional modules register command declarations and context
  declarations before sealing.
- One `global` root and one ordered hot-context path define v1 focus
  specificity. Consumers own context names and current focus/selection facts.
- A command declares discovery metadata, a closed argument schema, context and
  capability gates, visibility, input policy, and an opaque consumer route
  key. It does not declare a bridge operation name.
- Runtime availability is separate from registry identity.

### Arguments

The v1 argument schema supports no arguments or one bounded object containing
named boolean, finite number, bounded integer, bounded string, or closed-enum
fields. Required fields, defaults, ranges, and lengths are explicit.

Nested arbitrary JSON, arrays, binary values, and unknown fields fail
validation. Product commands needing richer payloads use a consumer-owned
typed workflow and may expose a simpler palette command that constructs that
payload under product authority.

### Availability and execution

- Availability is a projection over one registry generation and current
  context revision. Reasons use stable codes plus bounded optional detail.
- A visible or previously available command has no execution authority.
- An execution request carries request id, registry generation, command id,
  structurally validated arguments, and observed context revision.
- The authority-side admission path reloads current context, revalidates
  capability, context, availability, and arguments, then calls an injected
  consumer executor.
- The executor maps the admitted command to renderer-local behavior, a typed
  local domain API, or a domain-owned bridge operation.
- Longhorn supplies no generic Tauri or bridge command accepting
  `{ commandId, args }`.
- Outcomes distinguish unknown, stale registry, invalid arguments, unavailable,
  unauthorized, cancelled, rejected, failed, and indeterminate. Product
  receipts remain product-owned and may be carried only through a bounded
  consumer result seam.

### Keyboard and keymaps

v1 covers one physical keyboard chord on press:

- `KeyboardEvent.code`, not locale-dependent produced text
- canonical modifier ordering
- semantic `primary` modifier resolved to Meta on macOS and Control on Windows
  and Linux
- platform-specific presets where semantic primary is insufficient
- one active hot-context path with most-specific match
- repeat, composition/IME, text-input, capture-mode, and injected
  platform-reserved gates

The event is consumed only after one effective binding resolves. An admitted
binding remains consumed if later command availability rejects it; an
unbound, gated, reserved, or ambiguous chord is not consumed.

Base presets are immutable and consumer-supplied. Every base binding has a
stable id. Sparse overrides use explicit disable, replace, and add directives;
they never edit or copy the full base. Resolution first applies directives,
then selects the most-specific context. Multiple different invocations at the
same winning specificity are an unresolved conflict and cannot execute.

### Persistence

- `longhorn-command-config` registers the shared keymap selection and override
  schema as an ordinary user-config domain under contract 004.
- Mutation uses expected config revision, current registry generation, stable
  binding ids, validation, and coordinated publication.
- Conflict preview and commit are bound to the same proposed patch digest and
  base revision. A changed registry or keymap returns fresh state.
- Invalid, reserved, ambiguous, or unknown-command overrides do not persist.
- Session state becomes effective only from the authoritative receipt and
  snapshot. Persistence failure is not reported as successful rebinding.
- `longhorn-tauri-command` may expose registry, keymap, preview, and mutation
  queries over injected authorities. It exposes no generic command executor.

### Projection and UI

- Rust owns canonical search, ordering, conflict, and shortcut-label semantics.
  Checked TypeScript runs the same fixtures.
- Palette, menu, shortcut, help, and keybinding-settings records are
  projections of one registry and effective keymap.
- `@inflatable-cookie/longhorn-commands` is framework-neutral and accepts injected catalogue,
  keymap, availability, and executor ports.
- Optional Svelte state is per instance and follows contract 013.
- Optional Poodle bindings use public `CommandPalette` and settings primitives.
- The keybinding settings module registers through contract 005 only when
  commands and writable keymap configuration are composed.

## Package Consequences

- `longhorn-command`: registry, contexts, argument validation, availability
  admission, keyboard/keymap resolution, search, and projection
- `longhorn-command-config`: registered keymap domain and coordinated mutation
- `longhorn-tauri-command`: narrow registry/keymap query and mutation assembly
- `@inflatable-cookie/longhorn-commands`: checked protocol, clients, resolver helpers, and
  optional `/svelte` and `/poodle` subpaths
- `longhorn-bindings`: checked command/keymap protocol generation

`longhorn-command` depends only on core. The config and Tauri edges remain
optional. No command package depends on bridge, settings, Svelte, or Poodle.

## Deferred

- macros and multi-command transactions
- multi-stroke sequences
- key release and repeat actions
- mouse, wheel, gesture, playback, MIDI, and global system-wide hotkeys
- native menu accelerator registration
- live registry mutation and third-party command installation
- server-synchronized keymaps and multi-device conflict merge
- command automation and scripting

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/system-inventory.md`
- `../../architecture/package-topology.md`
- `../../contracts/006-command-action-and-input.md`
- `../../contracts/010-rust-typescript-ipc-and-events.md`
- `../../contracts/013-svelte-and-poodle-adapter-lifecycle.md`
- `../../roadmaps/g01/010-command-registry-keymaps-and-palette.md`
- Cards 056-061
