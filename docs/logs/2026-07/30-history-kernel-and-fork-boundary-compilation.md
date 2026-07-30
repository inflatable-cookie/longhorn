# History Kernel And Fork Boundary Compilation

Date: 2026-07-30
State: complete research and planning batch

## Outcome

- re-audited Loophole Pulse, Aura, Echo, persistence, journal, UI, ADR, and
  version-lineage surfaces read-only
- corrected the earlier claim that gesture grouping and forkable undo are live
- counted 83 live Pulse mutation variants and kept all meaning in Loophole
- separated linear history, crash-recovery journal, and project versions
- selected typed consumer payload and injected inverse/coalesce/no-op policy
- selected revision-bound plan, atomic product apply, then checked commit
- replaced renderer-remembered redo with authoritative metadata pages
- kept product snapshot, storage, journal, checkpoint, fsync, autosave, replay,
  and recovery policy outside Longhorn
- defined committed transition records as the lossless journal seam
- kept TypeScript metadata-only and Poodle visual
- promoted memo 015 into architecture, package topology, inventory, spec, and
  compiled contract 008
- compiled Cards 062-069
- made Card 062 the sole ready card

## Donor Findings

Live `pulse-history` is rich but linear. It owns a default-100 undo/redo stack,
typed inverses, automatic adjacent coalescing, a tested grouping API, persisted
stacks, and lightweight projections. The grouping API has no live Pulse call
site. A new record after undo clears redo.

Successful runtime edits record after apply. Undo and redo move the stack
before a fallible runtime apply, and compounds apply sequentially. No donor
test proves exact failure rollback. The shared contract therefore preserves
successful behavior and strengthens failure invariance.

Project snapshots remain canonical. Persisted history is lenient and may drop
wholesale when payload vocabulary drifts. A separate one-app-version session
journal records mutation, undo, and redo transitions, replays beyond a loaded
revision, tolerates torn tails, and preserves cross-session undo.

Aura's panel receives only eight applied entries and remembers disappeared
redo entries locally. Its live jump path loops undo/redo by entry id with a
512-step guard. The shared protocol instead exposes authoritative bounded
past/current/future pages and one checked checkout transaction.

Forkable undo is archived research. Loophole project versions are separate
immutable snapshot lineage above working-tree undo.

## Compiled Runway

1. Card 062 — donor fixtures and typed entry/policy foundation
2. Card 063 — transactional linear navigation and failure invariance
3. Card 064 — coalescing, grouping, retention, and projections
4. Card 065 — structural persistence and committed transitions
5. Card 066 — generated clients, Tauri, Svelte, and Poodle
6. Card 067 — rich/minimal artifact proof and linear checkpoint
7. Card 068 — private forkable-tree prototype
8. Card 069 — promote, retain, or reject decision and closeout

Card 062 is ready. Cards 063-069 remain planned. Card 067 pauses before branch
work. Card 069 cannot auto-promote the prototype.

## Limits

- no donor repository changed
- no branch behavior is claimed as live Loophole functionality
- no product payload crosses the generic renderer protocol
- no public branch package is planned before the promotion decision
- no event-sourcing, collaboration, or project-version authority moved into
  history
- no code changed in this batch

## Validation

- focused g01.011 Northstar path checks
- documentation links and indexes
- roadmap ready/planned state checks
- `git diff --check`

## Posture

`strict-ready`

## Next

Execute Card 062. Stop if typed product meaning, model apply, or persistence
location must enter the pure history crate.
