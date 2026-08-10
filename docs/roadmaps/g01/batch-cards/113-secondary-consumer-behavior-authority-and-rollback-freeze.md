# 113 Secondary-consumer Behavior, Authority, And Rollback Freeze

Status: complete
Owner: Tom
Roadmap: g01.016 batch 1
Governing refs: contracts 001, 003-007, 009-010, 012-017;
`../../../architecture/secondary-consumer-migration-map.md`
Depends on: Card 112
Auto-start next card: no

## Objective

Freeze current Soundcheck, Split-shell, and Jetstream behavior, authority, selected
package shape, and rollback inputs before any consumer migration write.

## Repository Scope

- Longhorn: documentation, fixtures, and focused verification may change.
- Soundcheck, soundcheck-library, Signal, Split-shell, Jetstream, and Poodle:
  read-only.

## Scope

- exact branches, commits, worktree posture, toolchains, dependencies, and app ids
- Soundcheck storage, window, settings, backup, scan, and inspection-helper seams
- Split-shell workspace preference and Poodle presentation seams
- Jetstream bridge, keyboard, command, backing surface, viewport, and teardown seams
- selected versus rejected Longhorn packages for each consumer
- retained product/sibling authority and per-slice rollback evidence
- overlapping consumer work and write-admission gates

## Steps

1. Record exact repository and Poodle artifact receipts.
2. Recheck each app's active Northstar lane and consumer constraints.
3. Freeze source-backed behavior matrices without copying product payloads.
4. Map each shared mechanism to a current Longhorn contract and package.
5. Record retained product, sibling, and platform authority.
6. Record selected, optional, rejected, and deferred package edges.
7. Define one cutover and rollback boundary per planned consumer slice.
8. Classify dirty or overlapping consumer work before write admission.

## Acceptance Criteria

- every source receipt and worktree posture is exact
- Soundcheck stable-name storage and external SQLite authority are explicit
- Split-shell remains the minimal no-service, no-Surface, no-layout case
- Jetstream product commands, renderer, WGPU, world, and semantic input stay local
- all planned shared behavior maps to implemented Longhorn packages
- every donor slice has one rollback route without dual writes or silent fallback
- unrelated Split-shell documentation work remains untouched
- no consumer or Poodle files change

## Evidence Required

- checked multi-repository receipt fixture
- source-token and authority matrix verifier
- selected/rejected package inventory
- rollback and overlap report
- focused Northstar validation

## Stop Conditions

- current behavior contradicts a promoted Longhorn contract
- a selected source is dirty in migration-owned code
- a product or sibling authority would move into Longhorn
- a required public Poodle seam is absent
- rollback requires a second active write authority

## Completion Evidence

- fixture:
  the migration receipt (retired 2026-08-10; in git history)
- verifier: removed 2026-08-10 — Longhorn no longer keeps consumer-aware proofs; the recorded fixture is the retained evidence
- exact clean receipts: Soundcheck, soundcheck-library, Signal, Jetstream, and
  Poodle; exact 19-path docs-only overlap in Split-shell
- checked graph: 23 source matrices, 27 selected Longhorn package paths, and
  zero selected/forbidden overlap
- rollback: retained sources, exact receipts, and previous locks required;
  dual writes, silent fallback, and automatic cleanup forbidden
- publication: package managers, tags, and hosted releases remain false
- focused gate: `effigy qa:northstar:g01-secondary-consumer-card113`

## Next Task

Execute Card 114's private artifact admission. Do not modify a consumer or
publish packages.
