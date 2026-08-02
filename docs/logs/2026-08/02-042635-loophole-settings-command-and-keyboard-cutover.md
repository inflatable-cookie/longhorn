# Loophole Settings, Command, And Keyboard Cutover

Date: 2026-08-02
Roadmap: g01.015
Card: 110
State: complete

## Changed

- Replaced Aura's parallel settings modal mechanics with one sealed Longhorn
  registry, checked settings session, and public Poodle shell.
- Registered App autosave as an immediate apply unit backed by Aura's settings
  ConfigStore and Pulse activation.
- Registered Appearance as a staged apply unit backed by the renderer
  preferences ConfigStore. Apply emits a typed cross-window activation event;
  Cancel restores the authoritative value; reset removes the override.
- Kept Hardware and Workspace pages as typed Loophole content without claiming
  generic settings authority.
- Added real storage profile, root, locator, transition, warning, and provenance
  diagnostics.
- Replaced Aura's command catalogue, palette search, keyboard resolution,
  conflict, capture, and keymap mutation paths with the sealed Longhorn command
  registry, controller, session, and Poodle projections.
- Retained Aura/Pulse typed command execution. Availability is joined into
  discovery and re-read by the executor.
- Imported representable Echo keyboard overrides once into sparse Longhorn
  keymap directives. Last legacy winner is preserved. A macro collision disables
  the shadowed base binding without pretending the macro is generic.
- Retained ordered and delayed keyboard macros behind the named Loophole Echo
  compatibility adapter. Macro steps enter the command controller. Mouse, wheel,
  gesture, playback, MIDI, and macro infrastructure remains Echo-owned.
- Removed the redundant generic Aura keyboard adapter, host provider, chord
  capture, conflict, keymap wire, and mutation commands.

## Authority Boundary

Longhorn owns the sealed settings and command structure, apply protocol,
physical single-command keyboard resolution, sparse keymap persistence,
conflicts, capture, search, and palette projection. Loophole owns settings
meaning, command meaning, current availability, Aura/Pulse execution, and all
extended-input semantics.

Backup and restore pages are not admitted. Loophole has no product-authorized
archive picker, restore atomicity boundary, or destructive recovery policy.
Rendering inert controls would be a false authority claim. App and Appearance
reset use the shared confirmation flow; storage state and recovery-relevant
transition diagnostics remain visible.

## Evidence

- `fixtures/migration/loophole-card110/settings-command-cutover-v1.json`
- `scripts/verify-loophole-card110.ts`
- Registry test proves the exact six admitted pages, immediate/staged units,
  reset features, and backup/restore exclusion.
- Command tests prove every built-in keyboard command exists in the sealed
  registry, last-winner import, and macro shadowing without generic import.
- Renderer boundary tests prove current availability admission and typed
  product execution.

## Validation

- Aura Svelte check: 0 errors; one pre-existing tsconfig warning.
- Focused native command, settings, and renderer-preference tests: 5 passed.
- Focused renderer command, macro, and palette-focus tests: 17 passed.
- Full Loophole validation passed: Aura 993, Echo 414, Pulse 382, and Spark 11
  tests. One existing Spark test remains classified leaky by nextest.
- Card 110 migration verifier: passed.
- Repository-wide Prettier remains red on 123 pre-existing files; Card 110 files
  were formatted directly.

## Next

Execute Card 111. Replace Pulse's generic linear-history structure without
losing payload, inverse, grouping, transaction, journal, recovery, or project
version semantics.
