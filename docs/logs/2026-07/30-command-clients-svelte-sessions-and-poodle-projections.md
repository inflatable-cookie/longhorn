# Command Clients, Svelte Sessions, And Poodle Projections

Date: 2026-07-30
Card: 060
Roadmap: g01.010

## Result

Added checked framework-neutral command clients, generation-safe joined state,
canonical browser projections, per-instance Svelte sessions, public Poodle
bindings, and capability-gated keybinding settings registration.

## Framework-neutral Boundary

`@longhorn/commands` accepts injected catalogue, keymap, availability, and
executor ports. It attaches invalidation listeners before loading snapshots,
joins matching registry generations and digests, rejects stale async results,
and handles late listener completion after disposal.

One controller projects palette, menu, help, shortcut, reverse lookup,
conflict, and keybinding settings records. Palette and keyboard dispatch call
the same consumer executor. No raw Tauri, bridge, product route, or generic
`{ commandId, args }` execution transport exists.

Recovery, unavailable, failure, dirty, previewing, committing, conflict,
saved, and per-command availability states stay distinct.

## Cross-language Semantics

Rust-generated bindings and fixtures now include availability, search,
shortcut, keyboard input, candidate, conflict, capture, gate, resolution, and
consumption records.

TypeScript runs the same fixtures for:

- canonical search scores and ordering
- macOS, Windows, and Linux shortcut labels
- repeat, IME, reserved, and editable-text gates
- capture without dispatch
- equal-specificity conflict without consumption
- resolved winner with consumption
- unbound input without consumption

Browser helpers use `KeyboardEvent.code` and consume only captured or resolved
chords. Availability is still advisory; the injected executor retains fresh
product admission.

## Optional UI And Settings

`@longhorn/commands/svelte` owns one session's listeners, query, palette,
capture, draft access, and teardown. Mounted multi-instance tests prove no
hidden global state. Late listener, repeated start/stop, capture cleanup, and
SSR imports are covered.

`@longhorn/commands/poodle` uses only public controlled exports from exact
Poodle preview set
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.
Product category labels, copy, icons, handlers, and keymap patch construction
remain injected.

`longhorn-command-settings` registers the keybinding page only when both
command-catalogue and writable-keymap capabilities are composed. The crate
owns no command execution, persistence, settings scope, or apply unit.

## Validation

- focused command binding, Rust core/config/settings, TypeScript, Svelte,
  package, and Poodle-preview Effigy selectors
- focused Clippy with warnings denied
- Rust formatting check
- 48 command-core, 13 command-config, 2 command-settings, 16 framework-neutral
  TypeScript, and 4 mounted/SSR fixtures

## Next

Card 061 is ready. Prove rich Loophole-shaped and minimal Jetstream-shaped
compositions from produced artifacts, publish guidance, and close g01.010.
