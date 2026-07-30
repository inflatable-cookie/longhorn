# Deterministic Keyboard And Keymap Resolution

Date: 2026-07-30
Card: 058
Roadmap: g01.010

## Result

Extended `longhorn-command` with pure press-only physical keyboard and
effective-keymap semantics. One immutable compiled map now owns runtime
resolution, conflict evidence, reverse lookup, and shortcut labels.

## Physical Chords

- bounded DOM `code` identity; produced text is not accepted as canonical input
- canonical Control, Alt, Shift, Meta facts
- semantic primary expands to Meta on macOS and Control on Windows and Linux
- redundant semantic/native modifiers fail
- bindings declare macOS, Windows, Linux, or any-platform posture
- labels use the same normalized platform chord as lookup

V1 still excludes release, repeat actions, multiple strokes, mouse, wheel,
gesture, playback, MIDI, global hotkeys, and native accelerators.

## Presets And Directives

One versioned consumer preset supplies stable base binding ids. Sparse state
contains only:

- disable one base id
- replace one base id while retaining its identity
- add one new stable binding id

Compilation validates counts, stable identities, base targets, command and
context references, command context admission, shortcut visibility, closed
arguments, modifier posture, and injected reserved policy for overrides.
Preset and directive input order do not affect the effective map.

## Resolver

Resolution validates one registered `global`-to-leaf context path, filters
platform and chord, then selects the deepest matching context. Every match
remains in the report:

- one canonical winner
- semantically equivalent equal-rank bindings
- lower-context shadowed candidates
- equal-rank different-invocation conflict candidates

Different invocations at equal specificity never dispatch. Binding order has
no winner authority. Static platform conflict projection and runtime conflict
outcomes use the same normalized effective bindings.

## Gates And Consumption

- repeat, composition, reserved, and editable-text gates do not dispatch
- text admission comes only from the registered command
- unbound, gated, and conflicting chords are not consumed
- capture records and consumes one non-reserved chord without dispatch
- one resolved binding is consumed before fresh execution admission

Later availability or product rejection therefore cannot leak a resolved chord
back into browser behavior.

## Projection

The effective map exposes stable binding order, preset/replacement/add source,
matched context, specificity, shadowing, winner, conflict, command-to-binding
reverse lookup, normalized chords, and platform labels.

Loophole fixtures use project, Surface, region, and panel contexts. Jetstream
uses one global context. Neither imports donor product types.

## Boundary Audit

- normal dependencies remain `longhorn-core`, Serde, `serde_json`, and SHA-256
- no config, filesystem, Tauri, browser, Svelte, Poodle, or donor dependency
- no persistence or native accelerator authority
- no renderer ordering or locale-dependent key text
- no product focus or execution route enters keymap state

## Validation

- `effigy test:command-core`
- 48 command contract fixtures plus core and doc tests
- `cargo clippy -p longhorn-core -p longhorn-command --all-targets -- -D warnings`
- `RUSTDOCFLAGS='-D warnings' cargo doc -p longhorn-command --no-deps`
- `effigy qa:northstar:g01-command-keymap`
- `effigy qa:northstar`

## Next

Card 059 is ready. Add coordinated active-preset and sparse-override
persistence, checked TypeScript generation, and narrow Tauri query/mutation
assembly. Keep product command execution outside the host adapter.
