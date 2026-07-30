# 058 Deterministic Keyboard And Keymap Resolution

Status: complete
Owner: Tom
Roadmap: g01.010 batch 2
Governing refs: contract 006; research memo 014
Depends on: Card 057
Auto-start next card: no
Completed: 2026-07-30

## Objective

Implement deterministic single-chord keyboard normalization, immutable presets,
sparse override directives, effective-keymap resolution, and explainable
conflicts in the pure command crate.

## Scope

- physical keyboard code and canonical modifier model
- semantic primary modifier and platform resolution
- press-only single-chord v1 trigger
- stable preset and binding identities
- disable, replace, and add override directives
- one ordered hot-context path and most-specific resolution
- candidate, source, shadowing, winner, and conflict reports
- repeat, IME/composition, text-input, capture, and reserved-chord gates
- reverse command-to-binding lookup and shortcut labels
- Jetstream and Loophole-shaped keyboard fixtures

## Public Behavior

The effective keymap applies sparse directives to one immutable preset, then
resolves platform, trigger, and most-specific context. Equal-specificity
different invocations are unresolved conflicts. Binding order never silently
chooses a winner.

An unbound, gated, reserved, or ambiguous chord is not consumed. One resolved
binding is consumed before execution. Text-input permission comes from the
command declaration and cannot be widened by a user override.

## Out Of Scope

- filesystem persistence or migration
- TypeScript DOM event adapters
- multi-stroke, release, repeat, mouse, wheel, gesture, playback, MIDI, or
  global hotkey triggers
- macros or native menu accelerator installation

## Steps

1. Define physical codes, canonical modifiers, platforms, and chord identity.
2. Define immutable preset, base binding, and sparse override schemas.
3. Validate stable ids, command arguments, context references, and platform
   posture against the sealed registry.
4. Apply disable, replace, and add directives deterministically.
5. Resolve one hot-context path and report all candidates.
6. Reject equal-rank ambiguity instead of applying hidden order.
7. Add repeat, composition, text-input, capture, and injected reserved gates.
8. Add reverse lookup, source projection, and platform shortcut labels.
9. Prove rich and minimal donor shapes plus insertion-order invariance.

## Acceptance Criteria

- physical code meaning is independent of produced text
- semantic primary resolves per platform without ambiguous duplicate modifiers
- presets are immutable and every base binding has stable identity
- removing a default persists as a directive, not absence from a copied file
- replace and add cannot leave an implicit equal-rank winner
- most-specific context wins across one validated hot path
- capture mode never dispatches
- repeat and composition never dispatch ordinary v1 commands
- text fields suppress commands unless the declaration admits them
- reserved and unbound chords remain unconsumed
- reverse lookup and conflict UI consume runtime resolver records

## Evidence Required

- platform chord normalization matrix
- preset/directive validation and resolution matrix
- context specificity and ambiguity fixtures
- focus, repeat, IME, capture, reserved, and consumption fixtures
- reverse lookup and label fixtures
- Loophole and Jetstream-shaped effective keymaps
- focused Rust and Effigy checks

## Stop Conditions

- locale-dependent `key` text is required as canonical identity
- removal requires copying the full base preset
- equal-rank conflicts require “first” or “last” hidden precedence
- an override can widen a command's text-input policy
- extended triggers must enter the v1 schema without second-app evidence

## Next Task

Card 059 is ready. Persist active preset selection and sparse directives
through coordinated configuration, then generate the checked host protocol.

## Result

`longhorn-command` now compiles one versioned immutable preset plus sparse
disable, replace, and stable add directives into an immutable effective
keymap. Preset and directive input order carry no precedence. Stable binding
ids, command ids, registered contexts, command context admission, shortcut
visibility, closed arguments, semantic modifiers, and injected reserved
override policy validate before state becomes effective.

Physical input uses bounded DOM `code` identity and canonical Control, Alt,
Shift, Meta facts. Semantic primary expands to Meta on macOS and Control on
Windows and Linux. Redundant semantic/native modifiers fail validation.
Platform labels use the same normalized chords as runtime lookup.

Resolution validates one registered root-to-leaf hot-context path, filters
platform and chord, then selects the deepest matching context. Lower matches
remain visible as shadowed candidates. Equal-specificity different
invocations produce an unresolved conflict. Equal invocations may share a
trigger without semantic ambiguity; the lowest stable binding id is only the
canonical evidence representative.

Repeat, composition, editable-text, and injected reserved gates never
dispatch or consume. Unbound and conflicting chords remain unconsumed.
Capture records and consumes a non-reserved chord without dispatch. A resolved
binding is consumed before later execution admission. Candidate, source,
shadow, winner, conflict, reverse lookup, and shortcut-label records all come
from the same effective map.

## Validation

- `effigy test:command-core`
- `cargo clippy -p longhorn-core -p longhorn-command --all-targets -- -D warnings`
- `RUSTDOCFLAGS='-D warnings' cargo doc -p longhorn-command --no-deps`
- `effigy qa:northstar:g01-command-keymap`
- 48 command contract fixtures plus core and doc tests
- Loophole full-context and Jetstream global-context keymap fixtures
- dependency audit: pure crate remains on core, Serde, `serde_json`, and SHA-256
