# 122 Jetstream Bridge, Command, And Keyboard Cutover

Status: planned
Owner: Tom
Roadmap: g01.016 batch 4
Governing refs: contracts 003, 006-007, 010, 012-013; Cards 113-114 and 121
Depends on: Card 121
Auto-start next card: no

## Objective

Replace Jetstream's ad hoc editor-state bridge and keyboard table with checked
Longhorn bridge and command mechanisms without creating generic product
execution authority.

## Repository Scope

- Longhorn: focused bridge/command adapters, fixtures, evidence, and docs.
- Jetstream: editor bridge, registry, keyboard, host composition, tests, and docs.
- Poodle: read-only exact artifact use where a projection is selected.

## Scope

- one editor-state bridge domain with listener-before-snapshot reconciliation
- exact authority/session/epoch and whole-snapshot gap recovery
- sealed Jetstream command/context registry and fresh availability
- deterministic physical-keyboard resolution for existing shortcuts
- consumer-injected execution mapping to existing product command dispatch
- typed viewport and field IPC kept outside the generic command registry
- renderer remount, reconnect, teardown, capability, and stale-state behavior

## Steps

1. Freeze current snapshot, event, command, shortcut, and IPC traces.
2. Register the editor-state domain and checked bridge authority.
3. Replace renderer event wiring with listener-first checked reconciliation.
4. Register existing product commands and contexts without product payloads.
5. Move shortcut resolution to the shared effective keymap.
6. Inject Jetstream's executor and revalidate availability at invocation.
7. Keep viewport, gizmo, selection, and field commands on narrow typed routes.
8. Remove the superseded generic bridge and shortcut table.

## Acceptance Criteria

- one bridge authority owns editor snapshot ordering and resync
- one command registry/keymap owns discovery and keyboard resolution
- Jetstream owns command meaning, availability inputs, execution, undo/save, and world mutation
- no execute-by-string Longhorn Tauri or bridge endpoint exists
- typing gates, repeat handling, consumption, and platform labels remain correct
- stale sessions and snapshot gaps reload fresh authority
- teardown removes listeners without stopping the engine
- no config/settings/Surface/history/operation dependency is introduced

## Stop Conditions

- the bridge must understand editor snapshot fields
- a shared command route would execute an unchecked string
- typed viewport input would be forced through the command registry
- renderer/world authority would move into Longhorn

## Next Task

Execute Card 123. Adopt backing-surface coordination around Jetstream's
engine-owned native view and Svelte viewport.
