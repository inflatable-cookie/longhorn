# 060 Command Clients, Svelte Sessions, And Poodle Projections

Status: complete
Owner: Tom
Roadmap: g01.010 batch 3
Governing refs: contracts 005, 006, 010, 012, and 013; research memo 014
Depends on: Card 059
Auto-start next card: no
Completed: 2026-07-30

## Objective

Build framework-neutral command clients, per-instance Svelte state, shared
palette/menu/shortcut/help/settings projections, and public-Poodle adapters
over injected execution and checked host state.

## Scope

- `@inflatable-cookie/longhorn-commands` framework-neutral client and controller
- injected catalogue, keymap, availability, and executor ports
- stale-safe search and availability joins
- keyboard event normalization, gating, capture, and consumption helpers
- palette, menu, shortcut, help, reverse-lookup, and conflict projections
- optional `/svelte` per-instance session state
- optional `/poodle` command palette and keybinding settings bindings
- `longhorn-command-settings` optional settings registry module
- loading, unavailable, conflict, recovery, failure, and dirty states
- SSR/import safety and mounted teardown

## Public Behavior

All surfaces project the same sealed registry and effective keymap. Palette
selection and keyboard resolution call one injected executor. The client does
not construct a generic Tauri execution request.

Async search, availability, keymap, and mutation results are generation- and
revision-checked. One Svelte session owns its listeners, capture state, drafts,
and disposal. Poodle receives controlled records and callbacks only.

The settings module appears only when command and writable-keymap capabilities
are composed. Settings supplies navigation and shell state; command packages
retain keymap and dispatch authority.

## Out Of Scope

- copied or private Poodle components
- raw Tauri calls in Svelte or Poodle code
- product command handlers or copy
- native menu accelerator registration
- macros, automation, or extended triggers
- donor repository cutover

## Steps

1. Implement injected framework-neutral clients and session controller.
2. Join catalogue, effective keymap, and current availability by checked
   generation/revision.
3. Implement canonical palette, menu, shortcut, help, and settings projectors.
4. Adapt browser keyboard events, repeat/IME/text/capture/reserved gates, and
   exact consumption behavior.
5. Add stale-safe search and mutation request handling.
6. Add per-instance Svelte lifecycle and explicit state.
7. Bind public controlled Poodle `CommandPalette` and settings primitives.
8. Register the optional settings module through a narrow adapter package.
9. Add SSR, late-listener, repeated mount, capture teardown, and multi-instance
   tests.
10. Audit imports, peers, product authority, and execution routing.

## Acceptance Criteria

- every projection uses one command id and effective binding source
- stale search or availability cannot overwrite newer state
- keyboard and palette dispatch call the same injected executor
- no client sends `{ commandId, args }` to raw Tauri or bridge transport
- browser events are consumed exactly as the Rust resolution receipt states
- capture mode suppresses dispatch and cleans up exactly once
- Svelte sessions share no hidden global state
- SSR/build import touches no browser global
- Poodle integration uses public controlled props, snippets, and callbacks
- settings navigation appears only with composed command/keymap capabilities
- product copy, icons, route mapping, and handlers remain injected

## Evidence Required

- framework-neutral controller and stale-result tests
- cross-language projector and keyboard fixtures
- mounted Svelte lifecycle and multi-instance tests
- public Poodle palette/settings component tests
- settings admission and missing-capability fixtures
- SSR/import-safety and package-boundary audits
- focused frontend and Effigy checks

## Stop Conditions

- a UI adapter needs raw Tauri execution
- Poodle lacks a required public controlled seam
- one global singleton is required for keyboard or palette state
- settings must own command execution or keymap persistence
- Rust and browser resolution cannot share one semantic fixture

## Next Task

Card 061 is ready. Build rich and minimal artifact-installed command-system
proofs, publish composition guidance, and close g01.010.

## Result

`@inflatable-cookie/longhorn-commands` now has injected catalogue, keymap, availability, and
executor ports. One framework-neutral controller joins matching registry
generations and digests, rejects stale loads, search results, and mutation
results, and keeps recovery, unavailable, failed, dirty, conflict, and saved
postures explicit.

Rust-generated types and fixtures now cover availability, canonical search,
shortcut projection, keyboard input, candidate reports, gates, capture,
conflict, resolution, and consumption. TypeScript runs the same search,
shortcut, and keyboard cases. Browser helpers use `KeyboardEvent.code`,
normalize modifiers, detect editable targets, apply repeat/IME/reserved/text
gates, and consume only captured or resolved chords.

Palette, menu, help, shortcut, reverse-lookup, conflict, and keybinding
settings records project one joined catalogue and effective keymap. Palette
and keyboard dispatch call the same injected executor. No client or visual
adapter constructs a Tauri or bridge execution request.

The optional `/svelte` subpath owns one session's keyboard listener, palette
query, capture state, draft access, and teardown. Mounted multi-instance,
capture, repeated-start, and late-listener tests leave no shared state or
listener. SSR imports touch no browser global.

The optional `/poodle` subpath uses public `CommandPalette`, `TextInput`,
`Button`, and `Callout` exports from the exact Card 038 preview artifact.
Controlled records and callbacks retain product copy, category labels, icons,
handlers, and patch construction in the consumer.

`longhorn-command-settings` registers one keybinding renderer and page. Seal
admits it only when both command-catalogue and writable-keymap capabilities
are composed. It owns no command execution, keymap persistence, settings
scope, or apply unit.

## Validation

- `effigy check:command-bindings`
- `effigy test:command-core`
- `effigy test:command-config`
- `effigy test:command-settings`
- `effigy check:commands-ts`
- `effigy check:commands-svelte`
- `effigy test:commands-ts`
- `effigy test:commands-svelte`
- `effigy check:commands-package`
- `effigy verify:poodle-preview`
- focused Clippy with warnings denied
- `cargo fmt --all --check`
- 48 command-core, 13 command-config, 2 command-settings, 16 framework-neutral
  TypeScript, and 4 mounted/SSR fixtures
