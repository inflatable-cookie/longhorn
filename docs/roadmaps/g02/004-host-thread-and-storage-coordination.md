# g02.004 Host Thread And Storage Coordination

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001, 004, and 010; research memo 018
Depends on: none

## Outcome

Move fsync-heavy storage commands off the Tauri main thread, stop the file
lock spin-wait from freezing the UI, let plain loads self-heal terminal
restore phases, and make lock-poison handling consistent with the workspace.

## Generation Runway

Fourth g02 milestone. Bounded to `longhorn-config` coordination/restore and
the `longhorn-tauri-{config,settings,command}` command layers; storage
formats and the invoke wire surface stay fixed.

## Execution Plan

### Batch 1. Off-main-thread storage commands

- [x] [Card 143](batch-cards/143-async-storage-commands-and-lock-waiting.md)
  makes storage-heavy commands async over `spawn_blocking` and takes the
  coordination wait off the event thread

### Batch 2. Restore self-heal and poison consistency

- [x] [Card 144](batch-cards/144-restore-self-heal-and-poison-consistency.md)
  recovers terminal restore phases on bare load and surfaces coordination
  poison as typed errors

## Goals

- [x] no storage command blocks the main thread on IO or lock contention
- [x] invoke payloads, command names, and receipts unchanged
- [x] `ConfigStore::load` self-heals `Succeeded`/`RolledBack` journals the way
  coordinated load-sets already do
- [x] poison handling is a recorded decision at both sites (typed error
  reverted: recovery is load-bearing for in-process boot recovery)

## Acceptance Criteria

- [x] contended-lock and storage-transition scenarios keep the event loop
  responsive in the mock host proofs
- [x] crash-after-success followed by bare `load` returns data, not
  `Unavailable(RestoreActive)`
- [x] config, settings, command, and workspace QA pass; consumer invoke
  fixtures unchanged

## Explicit Non-goals

- storage format or journal schema changes
- new async runtime dependencies beyond Tauri's
- consumer repository edits

## Next Task

Promote Card 145 (g02.005).
