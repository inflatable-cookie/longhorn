# Loophole Full-hosting Migration Compilation

Date: 2026-08-01
Status: complete; roadmap compiled and paused at operator gate

## Changed

- audited current Loophole and Poodle read-only
- froze exact commits, active hierarchy, storage, window, layout, settings,
  command, Poodle, and history seams
- classified generic Echo mechanisms and retained Loophole product authority
- added an executable Card 102 fixture and verifier
- compiled g01.015 Cards 102-112
- held Card 103 on the exact storage matrix

## Baseline

- Loophole: clean `main` at
  `da08b50e7cc69b7d13636c94fc571a64db4ae8ca`
- Poodle: clean `main` at
  `208532f0d18dcd1683cdef157e370d0ba0f0d3b3`
- Aura id: `com.inflatablecookie.loophole`
- hierarchy: `display -> window -> Surface -> region -> panel`
- habitats: regional and focused-panel
- regional count: eight
- Pulse history payload variants: 83
- branch history: disabled and deferred

Archived Aura/Pulse applications were excluded from current authority. Chorus
contracts, active Echo/Pulse/Aura code, and current Poodle public contracts
were used in that order.

## Main Findings

### Storage

Loophole is the explicit stable storage identity. The canonical app id remains
the locator and machine identity. Chorus chooses one shared product root so
Aura, Spark, Pulse, Signal, and helpers can agree on local files.

The current Longhorn profiles do not reproduce all three Chorus paths:

- macOS matches the unified-root shape
- Windows differs on roaming `%APPDATA%` versus local data
- Linux differs on lowercase `loophole` versus exact stable name `Loophole`

Card 103 records both choices and recommends one shared-product-root profile
with exact `Loophole` leaf everywhere. No approximation or per-purpose override
was selected.

Current Echo import is copy-if-missing. It lacks locator-last commit, durable
journal, complete conflict inventory, verification, database snapshots, and
receipt-bound cleanup. Card 105 replaces it after profile and artifact gates.

### Hosting

Aura's current shell service combines window, Surface, layout, and persistence
authority. Its persistence logs failure but still retains and returns the
in-memory result. The migration splits window, Surface, and layout documents
and publishes only checked durable mutation success.

Focused panels remain a first-class Surface habitat. Promotion, fallback
return, empty-host policy, and fullscreen remain Loophole policy around generic
Longhorn lifecycle and window mechanisms.

Poodle already exposes public external drag preparation and drop hooks. The
migration can replace Aura's local Poodle MIME shaping with host-created
sessions without a new private Poodle seam.

### Commands And Settings

The generic keyboard, sparse keymap, conflict, palette, and settings-shell
mechanics can transfer. Loophole's mouse, wheel, gesture, playback, MIDI, and
macro model exceeds Longhorn contract 006. Those paths stay in a named
Loophole adapter and are excluded from closeout removal claims.

### History

The migration uses the public linear kernel only. Pulse retains the 83-variant
payload, inverse/coalesce/no-op rules, atomic apply, canonical state, project
versions, save/autosave, and JSONL journal policy.

The import must retain both undo and redo stacks, ids, revisions, grouping,
limit, and position without replaying entries into the loaded project. Undo,
redo, and checkout move to plan/apply/commit. The existing mutation/undo/redo
journal consumes committed transition evidence. Silent fallback to empty
history becomes visible recovery. Fork-tree adoption remains a later g01.017
decision.

## Admission Gates

1. Operator selects Windows storage parent and Linux leaf in Card 103.
2. Loophole baseline health must pass. It currently stops on one
   `libsqlite3-sys` link collision between Pulse `rusqlite 0.31` and the
   Soundcheck `rusqlite 0.40.1` graph.
3. The exact private Longhorn/Poodle graph must install from produced artifacts
   outside sibling/workspace resolution.

No Loophole migration writes are admitted. Card 104 may perform only the
explicit baseline dependency reconciliation after Card 103 completes.

## Validation

`bun scripts/verify-loophole-card102.ts` passes with
`pass_with_admission_gates`. It verifies 14 frozen source seams, exact commits,
the Tauri identity/capability posture, five-level hierarchy, eight regions,
both habitats, 83 history variants, and the compatible Longhorn Surface shape.

The full donor health failure is retained as baseline evidence rather than
misreported as a Longhorn regression.

## Next Move

Resolve Card 103's two storage choices. Then implement the selected profile in
Longhorn and proceed to clean donor/artifact admission.
