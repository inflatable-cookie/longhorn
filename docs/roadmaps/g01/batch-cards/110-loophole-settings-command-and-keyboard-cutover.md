# 110 Loophole Settings, Command, And Keyboard Cutover

Status: complete
Owner: Tom
Roadmap: g01.015 batch 4
Governing refs: contracts 004-007, 010, 012, and 013; Card 109
Depends on: Card 109
Auto-start next card: no

## Objective

Adopt the shared settings shell and generic keyboard/command discovery stack
without moving Loophole settings content, command meaning, or extended input.

## Repository Scope

- Longhorn: admitted settings/command adapter fixes and fixtures only.
- Loophole: Echo/Aura registrations, product pages, executor, migration, tests,
  and docs.
- Poodle: read-only artifact verification.

## Scope

- sealed settings and command registries
- App, Appearance, Hardware, Keybindings, and Workspace section composition
- storage, backup, restore, recovery, and diagnostics pages
- immediate/staged apply, managed policy, activation, reset, and deep links
- physical-keyboard resolution, sparse profile overrides, conflicts, and labels
- palette search and current availability join
- retained Aura/Pulse command executor and extended-trigger adapter

## Steps

1. [x] Register stable settings sections, fields, capabilities, and apply units.
2. [x] Move appearance and other durable renderer preferences from `localStorage`.
3. [x] Compose existing product pages inside the Longhorn/Poodle settings shell.
4. [x] Register command specs and contexts while retaining product execution.
5. [x] Import keyboard overrides through checked configuration migration.
6. [x] Replace Echo keyboard, conflict, and palette mechanics with Longhorn clients.
7. [x] Keep mouse, wheel, gesture, playback, MIDI, and macros behind an explicit
   Loophole adapter and exclude them from Longhorn compatibility claims.
8. [x] Remove only the generic donor code proven redundant by the selected slice.

## Recorded Boundary

The sealed registry admits real storage diagnostics. Backup and restore pages
remain absent. Loophole has no product-authorized archive picker, restore
atomicity boundary, or destructive recovery policy, so rendering those pages
would claim authority the app does not have. This is an explicit non-admission,
not a fallback control. App and Appearance reset use the shared confirmation
flow; storage transition, locator, warnings, roots, and provenance remain
visible.

Longhorn owns physical single-command keyboard presses. The named Loophole
extended-input adapter retains ordered and delayed keyboard macros. Macro steps
re-enter the command controller and receive fresh execution availability. Echo
retains mouse, wheel, gesture, playback, MIDI, and macro infrastructure without
moving those semantics into Longhorn.

## Acceptance Criteria

- one settings registry drives modal, deep-link, search, and policy projection
- storage/recovery state is visible and destructive actions require confirmation
- one command id means the same action across palette, keyboard, and other inputs
- palette and conflict UI use the actual keyboard resolver
- availability is rechecked at execution; visibility is not authorization
- extended input retains current semantics and named ownership
- no generic Tauri `{commandId,args}` executor enters Longhorn

## Stop Conditions

- one staged action requires uncontracted cross-domain atomicity
- hardware or workspace policy would move into Longhorn
- extended triggers must be silently dropped or reinterpreted
- a product command cannot map to one typed execution adapter

## Evidence Required

- registry and capability inventories
- old/new settings and keymap migration fixtures
- search, conflict, availability, apply, reset, and activation parity traces
- explicit extended-trigger retained-code report

## Next Task

Execute Card 111's complete linear-history adoption.
