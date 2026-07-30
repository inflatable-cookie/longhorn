# g01.010 Command Registry, Keymaps, And Palette

Status: complete
Owner: Tom
Updated: 2026-07-30
Governing refs: contracts 001, 004-006, 010, 012, and 013; research memo 014

## Outcome

Ship a product-neutral command and keyboard system with one sealed catalogue,
fresh execution admission, durable sparse keymap overrides, checked clients,
and shared palette/menu/shortcut/settings projections.

Loophole can compose the rich form. Jetstream and future small apps can use a
flat catalogue and one global keyboard context without bridge, settings, or
Poodle dependencies.

## Boundary

- Product commands, context facts, authorization, execution, and domain
  receipts remain consumer-owned.
- Command ids never become generic bridge or Tauri operation names.
- Longhorn validates and admits an invocation, then calls an injected consumer
  executor.
- V1 covers single physical keyboard chords on press.
- Presets are immutable. Sparse disable, replace, and add directives persist
  through contract 004.
- Poodle owns visual primitives. Longhorn owns state, search, conflicts,
  capture, and dispatch binding.

## Batch 1: Registry And Admission

Goals:

- [x] seal one bounded command/context registry
- [x] validate a closed structural argument schema
- [x] project deterministic discovery and search records
- [x] revalidate current context and availability before injected execution
- [x] prove local and typed-domain routes without a generic command bus

Cards:

1. [056 Command Registry, Context, And Argument Foundation](batch-cards/056-command-registry-context-and-argument-foundation.md)
2. [057 Fresh Availability And Injected Execution Admission](batch-cards/057-fresh-availability-and-injected-execution-admission.md)

## Batch 2: Keyboard And Durable Keymaps

Goals:

- [x] normalize physical chords and platform modifiers
- [x] resolve one hot-context path deterministically
- [x] model immutable presets and stable sparse directives
- [x] reject ambiguous, reserved, invalid, or stale overrides
- [x] persist through coordinated configuration with exact receipts
- [x] generate checked TypeScript and expose narrow Tauri query/mutation seams

Cards:

3. [058 Deterministic Keyboard And Keymap Resolution](batch-cards/058-deterministic-keyboard-and-keymap-resolution.md)
4. [059 Config-backed Keymaps And Generated Host Protocol](batch-cards/059-config-backed-keymaps-and-generated-host-protocol.md)

## Batch 3: Client And Poodle Projections

Goals:

- [x] provide framework-neutral catalogue, keymap, availability, and executor
  ports
- [x] project palette, menu, shortcut, help, and keybinding settings from one
  state
- [x] keep Svelte sessions per instance and import-safe
- [x] bind public Poodle palette and settings primitives
- [x] register the optional settings module only when its capabilities exist

Card:

5. [060 Command Clients, Svelte Sessions, And Poodle Projections](batch-cards/060-command-clients-svelte-sessions-and-poodle-projections.md)

## Batch 4: Two-shape Proof And Closeout

Goals:

- [x] prove Loophole-shaped native and Svelte clients over one semantic core
- [x] prove a Jetstream-sized global keyboard composition
- [x] install from produced Rust and TypeScript artifacts
- [x] audit package, capability, payload, authority, and Poodle boundaries
- [x] publish composition and later migration guidance
- [x] close g01.010 without modifying donor repositories

Card:

6. [061 Command System Artifact Proof And Closeout](batch-cards/061-command-system-artifact-proof-and-closeout.md)

## Acceptance

- [x] every projection consumes one registry generation and command identity
- [x] Loophole and Jetstream shapes import no shared product command catalogue
- [x] stale renderer availability cannot authorize execution
- [x] one admitted command maps to a consumer-owned typed domain operation
- [x] no generic execute-by-string endpoint exists in Tauri or bridge packages
- [x] physical-key, focus, repeat, IME, reserved, and conflict behavior is
  deterministic
- [x] override preview and publication cannot drift across registry or keymap
  revisions
- [x] persistence failure never produces an effective rebind
- [x] Rust and TypeScript agree on command/keymap wire arguments and outcomes
- [x] Svelte/Poodle adapters use public APIs and leave no hidden singleton
- [x] produced artifacts prove rich and minimal compositions

## Deferred

- macros and multi-command transactions
- multi-stroke sequences
- release/repeat, mouse, wheel, gesture, playback, and MIDI triggers
- native menu accelerators and system-wide hotkeys
- command automation and scripting
- live plugin registration
- server-synchronized keymaps and multi-device merge
- donor cutover, which remains g01.015 and g01.016 work

## Readiness

| Card | State | Gate |
| --- | --- | --- |
| 056 | complete | sealed registry, arguments, context graph, discovery |
| 057 | complete | fresh facts, admission, executor ports, outcomes |
| 058 | complete | physical chords, sparse directives, deterministic resolution |
| 059 | complete | coordinated config, checked protocol, narrow Tauri host |
| 060 | complete | checked clients, per-instance state, public Poodle/settings edges |
| 061 | complete | isolated rich/minimal artifact proof and boundary audits |

## Stop Conditions

- a command id must double as a bridge operation name
- generic arguments require unchecked arbitrary JSON
- availability can only be computed from renderer-owned stale state
- keymap persistence would bypass configuration coordination or exact
  durability
- equal-rank conflicts require hidden binding-order policy
- a shared package must import a donor product type or Poodle private API

## Next Task

The g01.011 research gate is complete. Execute Card 062 from the compiled
history runway.
