# Command, Input, And Palette Compilation

Date: 2026-07-30
State: complete research and planning batch

## Outcome

- re-audited Loophole Echo/Aura and Jetstream read-only
- checked the donor command split against completed bridge, config, settings,
  Svelte, and Poodle contracts
- separated command identity from bridge operation identity
- selected consumer-injected local or typed-domain execution routes
- rejected a generic shared Tauri execute-by-string endpoint
- selected single physical keyboard chords on press for v1
- fixed one hot-context path, semantic primary modifier, focus/repeat/IME
  gates, and exact event-consumption posture
- replaced append-order override semantics with stable binding ids and sparse
  disable, replace, and add directives
- made equal-rank ambiguity an invalid conflict, not a hidden winner
- bound conflict preview and durable commit to registry generation, config
  revision, and patch digest
- kept settings optional and Poodle visual
- deferred macros, extended triggers, native accelerators, automation, and
  synchronized keymaps
- promoted memo 014 into architecture and compiled contract 006
- compiled g01.010 into Cards 056-061
- made Card 056 the sole ready card

## Donor Evidence

Loophole supplies separate catalogue, availability/runtime, input resolution,
action/conflict, palette, and rebinding layers. Its strongest reusable ideas
are one discovery identity, physical-key chords, context specificity, reverse
lookup, candidate reports, and shared search.

The donor also supplies negative evidence. Command arguments and availability
reasons are stringly, context and runtime routes are DAW-specific, resolver
tie rules disagree, renderer wire types are handwritten, unsupported trigger
types can disappear from display, and some rebinding paths report session
success before best-effort persistence.

Jetstream proves the small second-app floor: one flat command table, semantic
Command/Control modifier, no repeat, no editable-target leakage, and event
consumption only after resolution. It does not justify macros or non-keyboard
trigger families.

Poodle already exposes a public controlled `CommandPalette`. Longhorn needs a
state and dispatch adapter, not another visual component.

## Contract Decisions

- Registry and context declarations seal into one deterministic generation.
- Arguments use a closed bounded structural v1 schema.
- Availability is a revisioned projection, never execution authority.
- Authority-side admission reloads context and capabilities before calling an
  injected consumer executor.
- Consumer executors map commands to renderer-local behavior or typed domain
  operations.
- Command ids never become bridge operation names.
- V1 keymaps use physical codes, canonical modifiers, immutable presets, and
  stable sparse directives.
- Equal-rank different invocations cannot persist or execute.
- Keymap configuration uses coordinated fresh mutation and exact durability.
- Palette, menu, shortcut, help, and settings records project the same registry
  and effective keymap.
- Svelte state is per instance. Poodle and settings remain optional edges.

## Compiled Runway

1. Card 056 — command registry, context, and argument foundation
2. Card 057 — fresh availability and injected execution admission
3. Card 058 — deterministic keyboard and keymap resolution
4. Card 059 — config-backed keymaps and generated host protocol
5. Card 060 — command clients, Svelte sessions, and Poodle projections
6. Card 061 — artifact proof and closeout

Card 056 is ready. Cards 057-061 remain planned so implementation cannot
outrun the invariants established by the preceding card.

## Limits

- no consumer repository was modified
- no product command catalogue, context snapshot, route, or receipt moved into
  Longhorn
- no generic product execution transport was added
- no macro, non-keyboard trigger, or native accelerator support was claimed
- no public package name or compatibility range was claimed
- donor cutover remains g01.015 and g01.016 work

## Validation

- focused g01.010 Northstar path checks passed
- documentation links and indexes passed
- `git diff --check` passed
- one ready card and five planned cards are indexed
- no code changed in this batch, so Rust and frontend suites were not repeated

## Posture

`strict-ready`

## Next

Execute Card 056. Stop if registry sealing requires product state, a bridge
route, arbitrary JSON, or a UI dependency.
