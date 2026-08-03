# 137 Grouped Adapter Absence Conformance And Nucleus Handoff

Status: complete
Owner: Tom
Roadmap: g01.019 batch 7
Governing refs: contracts 001, 004, and 012; Cards 135-136; Nucleus g05.046
Depends on: Card 136
Auto-start next card: no
Completed: 2026-08-03

## Objective

Prove the explicit state model across real adapter shapes and reopen the exact
Nucleus consumer gate.

## Scope

- optional file and WAL-mode SQLite fixtures
- ordinary and separate-adapter regressions
- public API and package evidence
- Longhorn closeout log
- read-only Nucleus consumer handoff

## Steps

1. Commit an absent optional file beside a present WAL-mode SQLite target.
2. Fail a mixed group and verify rollback of a file to absence.
3. Re-run every interruption and boot-recovery phase.
4. Compile the public adapter and receipt surface as an external consumer.
5. Record exact Nucleus adapter changes without editing Nucleus.

## Acceptance Criteria

- mixed commit and rollback never leave a split generation
- receipts project explicit target and rollback state
- Rust 1.85, Clippy, package, binding, focused, and aggregate QA pass
- Nucleus receives an exact resume path for its paused absence fixture
- no Nucleus source is edited and no package is published

## Evidence Required

- mixed fixture receipts
- interruption matrix
- public API compile receipt
- package and aggregate QA receipts
- Nucleus handoff log

## Stop Conditions

- the mixed SQLite fixture copies a live main/WAL pair
- generic renderer restore authority becomes necessary
- Nucleus lifecycle work is required for Longhorn conformance

## Next Task

Resume Nucleus g05.046 from the recorded consumer handoff.

## Evidence

- mixed absent optional-file and WAL-mode SQLite commit passes
- mixed rollback restores an absent file and the old SQLite generation
- ordinary, separate-adapter, storage-transition, and present grouped suites remain green
- public integration test, Rust 1.85, Clippy, package, binding, Northstar, and workspace QA pass
- closeout and exact read-only Nucleus handoff logs are indexed
