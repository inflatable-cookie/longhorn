# 006 Command, Action, And Input

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
Primary donor: Loophole Echo/Aura

## Model

A command registry is the single catalogue for palettes, menus, shortcuts,
settings, automation, and help.

A command declares:

- stable namespaced id
- label, description, category, and optional icon token
- typed argument contract
- context and capability requirements
- availability query
- execution route
- default bindings supplied by a keymap, not by the command itself

## Execution

- Registration and execution are separate.
- Availability is resolved against a current context snapshot.
- Execution revalidates authority-side state.
- Unknown, unavailable, invalid, cancelled, and failed are distinct outcomes.
- Product commands remain consumer-owned and register through the same seam.
- The palette is a projection of the registry, never another command list.

## Input

- Triggers are normalized independently from commands.
- Keymaps supply defaults; user overrides are sparse.
- Context specificity and override precedence are deterministic.
- Conflicts are reportable before persistence.
- Text-input focus and reserved OS/browser chords are explicit gates.
- Rebinding persists through contract 004 and excludes invalid overrides.
- Macros remain optional and cannot bypass command validation.

## UI

- Poodle owns palette, menu, list, input, and shortcut presentation.
- Longhorn owns search records, availability, dispatch, chord capture helpers,
  conflict models, and Svelte bindings.

## Acceptance

- Loophole commands and a small Jetstream command set share the registry
  without importing DAW types
- palette, menu, and keybinding settings use the same command ids
- unavailable commands cannot execute through a shortcut race
- conflicts and override precedence have fixture coverage
- renderer and host agree on command arguments and outcomes

