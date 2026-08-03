# 130 Grouped Adapter Recovery And Conformance

Status: complete
Owner: Tom
Roadmap: g01.019 batch 3
Governing refs: contracts 001 and 004; Cards 128-129
Depends on: Card 129
Auto-start next card: yes

## Objective

Prove catalogue-bound boot recovery and realistic mixed-adapter atomicity.

## Scope

- grouped restore operation state and ordinary-write blocking
- exact-catalogue boot rollback and cleanup
- interruption at durable phases
- corrupt journal and missing/changed adapter refusal
- mixed adapter and WAL-mode SQLite fixtures

## Steps

1. Integrate grouped state with store load and mutation gates.
2. Implement idempotent catalogue-bound recovery.
3. Inject interruption after journal, apply, verify, and terminal markers.
4. Prove two adapter implementations in one group.
5. Prove SQLite snapshot, apply, rollback, and boot recovery.

## Acceptance Criteria

- restart never exposes a mixed writable generation
- absent or changed adapters retain recovery-required state
- repeated recovery is idempotent
- SQLite main/WAL files are never copied as ordinary domain files
- current separate-adapter and ordinary file recovery remain unchanged

## Evidence Required

- interruption/restart matrix
- catalogue mismatch and corruption fixtures
- mixed-adapter trace
- SQLite conformance trace
- focused Rust QA

## Stop Conditions

- recovery requires an already-open live SQLite authority
- rollback evidence cannot be verified independently
- grouped state can be bypassed by ordinary mutation

## Next Task

Card 131 closes public evidence and writes the Nucleus resume handoff.

## Evidence

- grouped journal state blocks ordinary load, mutation, and single-adapter restore
- exact-catalogue recovery is renderer-free and fails closed on drift or corruption
- applying, verifying, and rolling-back interruption fixtures recover the old group
- mixed opaque-file and WAL-mode SQLite commit and rollback fixtures pass
