# 006 Command, Action, And Input

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-30
Evidence: `../research/translation-memos/002-shared-desktop-systems-follow-up.md`;
`../research/translation-memos/014-command-input-and-palette-boundary.md`

## Boundary

Longhorn owns product-neutral command registration, discovery, structural
argument validation, availability admission, keyboard resolution, keymap
storage, and presentation projections. Apps and optional modules own command
meaning, current context facts, product authorization, execution routes, and
domain receipts.

A command id is semantic identity. It is not a bridge operation name. Longhorn
provides no generic Tauri or bridge execute-by-string bus.

## Registry

The host registers command and context declarations, then seals one registry
generation and deterministic digest before serving it.

A command declares:

- stable bounded namespaced id
- label, description, category path, keywords, and optional icon token
- palette, menu, shortcut, settings, help, and hidden visibility
- closed v1 argument schema
- allowed contexts and required composed capabilities
- text-input admission policy
- opaque consumer route key

Default bindings belong to keymap presets, not command declarations. Duplicate
ids, unknown contexts or capabilities, invalid schemas, invalid defaults, and
unbounded metadata fail registration.

Consumers own context names. V1 registers one finite rooted context tree below
`global` and projects one ordered hot-context path. Registry identity does not
change with focus, selection, connection, or runtime availability.

## Arguments

V1 accepts no arguments or one bounded object with named:

- boolean fields
- finite numbers with optional bounds
- bounded integers with optional bounds
- bounded strings
- closed string enums

Required fields and defaults are explicit. Unknown fields, nested arbitrary
JSON, arrays, non-finite numbers, invalid defaults, and out-of-range values
fail. Product semantic validation still runs in the injected executor.

Commands requiring richer payloads stay behind product-owned typed workflows
or expose a simpler registered command that constructs the domain request
under product authority.

## Availability And Execution

Availability snapshots name registry generation and current context revision.
Each command has available, unavailable, hidden, or unsupported posture plus a
stable coded reason and bounded optional detail.

Palette visibility and renderer availability are advisory projections.
Execution:

1. validates request id, registry generation, command id, and arguments
2. reloads current consumer context and capability facts
3. reruns context, availability, and product admission
4. calls an injected consumer executor only after admission
5. returns a typed command outcome

The executor maps the command to renderer-local behavior, a typed local domain
API, or a domain-owned bridge operation. Bridge session, capability, and
authority checks remain contract-010 and domain concerns. Command capability
admission never grants bridge or product authority.

Outcomes distinguish unknown command, stale registry, invalid arguments,
unavailable, unauthorized, cancelled, rejected, failed, and indeterminate.
The consumer may attach bounded product evidence, but Longhorn does not
reinterpret a domain receipt.

## Keyboard

V1 covers one physical keyboard chord on press:

- `KeyboardEvent.code`, not produced text
- canonical modifier ordering
- semantic `primary` modifier: Meta on macOS, Control on Windows and Linux
- explicit macOS, Windows, Linux, or any-platform preset posture
- one current hot-context path; most-specific matching context wins
- repeat, composition/IME, text-input, capture-mode, and injected
  platform-reserved gates

Text-input admission comes from the command declaration, not an override.
Capture mode records a chord without dispatch. An unbound, gated, reserved, or
ambiguous chord is not consumed. One resolved binding is consumed before
execution; a later availability rejection does not leak the chord to browser
or OS behavior.

Multi-stroke input, release/repeat actions, mouse, wheel, gesture, playback,
MIDI, global system hotkeys, and native menu accelerator installation are
deferred.

## Keymaps And Conflicts

Apps provide immutable versioned presets. Every base binding has a stable
binding id, platform, trigger, context id, command id, and validated arguments.

Sparse user overrides are explicit:

- disable one base binding id
- replace one base binding id
- add one stable override binding id

Overrides never copy or mutate the full base preset. Resolution applies
directives, filters platform and trigger, then selects the most-specific
context. Multiple different invocations at the same winning specificity are
an unresolved conflict. No binding order silently selects a winner.

Candidate, source, matched-context, shadowing, conflict, and winner records
come from the same resolver used at runtime. Invalid, unknown-command,
reserved, or unresolved override state cannot persist or execute.

## Persistence

`longhorn-command-config` registers active preset selection and sparse
overrides as an ordinary user-config domain under contract 004.

Mutation carries expected keymap revision and registry generation. Conflict
preview and commit bind the proposed patch digest to the same base revision.
A changed registry or keymap returns fresh authoritative state.

Coordinated publication, migration, recovery, durability, backup, and restore
remain contract-004 behavior. Session state becomes effective from the
authoritative receipt and snapshot. A failed write is not a successful
rebind.

Settings may register the shared keybinding page only when command and writable
keymap capabilities are composed. Settings owns navigation and session shell,
not command or keymap authority.

## Projection And UI

Palette, menu, shortcut, help, and keybinding-settings records project one
registry and effective keymap.

Rust owns canonical search, ranking, shortcut labels, candidate ordering, and
conflict semantics. Checked TypeScript passes the same fixtures.

- `@inflatable-cookie/longhorn/commands` owns framework-neutral clients and injected catalogue,
  keymap, availability, and executor ports.
- Optional Svelte state is per instance and follows contract 013.
- Optional Poodle bindings use public controlled `CommandPalette` and settings
  primitives.
- Consumer copy, icons, menu placement, and command handlers remain
  consumer-owned.

No Longhorn visual adapter calls raw Tauri IPC or copies a Poodle component.

## Package Shape

- `longhorn-command`: pure registry, contexts, arguments, admission, keyboard
  resolver, search, and projections
- `longhorn-command-config`: registered keymap domain and coordinated mutation
- `longhorn-tauri-command`: narrow registry/keymap query and mutation assembly;
  no generic command execution
- `@inflatable-cookie/longhorn/commands`: checked protocol and optional `/svelte` and `/poodle`
  subpaths
- `longhorn-bindings`: checked command/keymap protocol generation

The pure package depends only on core. Config, Tauri, Svelte, Poodle, settings,
and bridge stay optional downstream edges.

## Deferred

- macros and multi-command transactions
- command automation or scripting
- live registry mutation and third-party command installation
- server-synchronized keymaps and multi-device conflict merge
- native accelerator registration and extended input families

## Acceptance

- Loophole-shaped and Jetstream-shaped catalogues share the registry without
  importing DAW or editor types
- palette, menu, shortcut, help, and settings projections use the same ids
- stale renderer availability cannot authorize execution
- a product command maps to a typed domain operation outside Longhorn
- no generic command execution appears in bridge or Tauri packages
- focus, repeat, IME, reserved, conflict, and unavailable races are covered
- override preview and durable commit cannot drift across revisions
- Rust and TypeScript agree on arguments, search, conflicts, and outcomes
- Poodle integration uses only public controlled APIs
