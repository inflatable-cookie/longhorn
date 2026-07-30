# 065 History Persistence And Committed Transition Stream

Status: planned
Owner: Tom
Roadmap: g01.011 batch 2
Governing refs: contracts 003, 004, 008, 010, and 012; research memo 015
Depends on: Card 064
Auto-start next card: no

## Objective

Add versioned structural history persistence, registered payload codecs and
migrations, visible recovery outcomes, and committed transition records
without taking ownership of product snapshots or journal files.

## Scope

- structural format family and version
- payload codec family and version
- bounded encoded payload weight
- complete linear state envelope
- checked load and migration
- explicit discard-history recovery receipt
- committed record, coalesce, navigation, prune, import, and reset transitions
- Loophole-shaped project snapshot and disposable journal adapters
- non-editor persistence round trip

## Public Behavior

Load validates the complete envelope before acceptance. Future, corrupt,
unbounded, or incompatible payloads reject visibly. A consumer migration may
choose an explicit history discard; no parse failure silently becomes empty
history.

Only committed state emits transition records. Consumers combine them with
product revision and durability policy. Journal failure does not rewrite an
in-memory success as durable success.

## Out Of Scope

- filesystem paths or I/O implementation
- config-domain registration by default
- canonical product snapshot migration
- autosave, checkpoint cadence, fsync, replay choice, or crash UI
- TypeScript
- branch graph persistence

## Steps

1. Define bounded structural and payload codec identities.
2. Encode the complete linear state and retained baseline.
3. Validate and decode before state acceptance.
4. Add registered structural and payload migration hooks.
5. Define explicit preserve, migrate, reject, and discard recovery outcomes.
6. Emit exact committed transition records for every structural mutation.
7. Prove product revision remains consumer-owned.
8. Add Loophole-shaped snapshot import and journal-record adapters.
9. Prove non-editor round trip and corruption behavior.

## Acceptance Criteria

- structural and payload versions are independent
- future and corrupt input fails before replacing live state
- migration output passes all current invariants
- silent empty-history fallback is impossible
- transition records exist only for committed changes
- product snapshot and model revision stay outside the kernel
- Loophole-shaped recovery can retain cross-session undo and journal suffix
- no filesystem or config dependency enters `longhorn-history`

## Evidence Required

- golden structural envelope
- version, corruption, bound, and migration matrix
- explicit discard receipt
- committed-transition trace for every mutation class
- Loophole-shaped snapshot/journal trace
- non-editor round trip
- dependency and durability-truth audit

## Stop Conditions

- the kernel must choose a storage root or journal file
- payload migration must understand product state
- compatibility needs silent data loss
- a transition is emitted before history commit

## Next Task

Card 066 is planned. Generate metadata-only clients and compose narrow Tauri,
Svelte, and public-Poodle edges.
