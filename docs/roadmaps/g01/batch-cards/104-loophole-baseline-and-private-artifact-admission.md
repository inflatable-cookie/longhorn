# 104 Loophole Baseline And Private Artifact Admission

Status: complete
Owner: Tom
Roadmap: g01.015 batch 1
Governing refs: contracts 003, 010, 012, and 013; Cards 102-103
Depends on: Card 103
Auto-start next card: no

## Objective

Restore a clean Loophole baseline and prove the exact private Longhorn/Poodle
graph before migration writes.

## Repository Scope

- Longhorn: artifact selectors, compatibility evidence, and docs.
- Loophole: only the bounded Cargo dependency reconciliation and read-only
  package/toolchain inspection until admission passes.
- Poodle and Soundcheck: read-only unless a separate owning-repo card is named.

## Steps

1. Reproduce the two-`libsqlite3-sys` conflict from the pinned donor commit.
2. Reconcile one compatible SQLite graph without changing product data meaning.
3. Run the complete Loophole baseline and record unrelated failures separately.
4. Select every package required by Cards 105-111.
5. Freeze exact clean Longhorn, Poodle, Loophole, Signal, and Soundcheck receipts.
6. Install packed npm and private Rust artifacts in isolated Loophole shapes.
7. Prove Tauri, Rust 1.85, Svelte, Poodle, protocol, and optional-edge compatibility.
8. Admit only Card 105's storage slice.

## Acceptance Criteria

- Loophole baseline health passes from one clean commit
- SQLite reconciliation has migration/readback evidence where needed
- clean artifact consumers use no sibling or workspace resolution
- source-linked development inputs match exact produced artifacts
- one Svelte/Poodle runtime resolves
- full optional hierarchy packages are present; fork-tree packages are absent
- package-manager publication remains out of scope

## Stop Conditions

- SQLite alignment changes a database schema or project meaning without contract
- any selected source is dirty or moving
- clean proof needs an unpacked sibling dependency
- produced artifacts differ from recorded commits

## Evidence Required

- clean baseline selector and SQLite reconciliation report
- exact source, lock, package, and artifact receipts
- isolated Rust and renderer install results
- optional-edge, peer-runtime, capability, and generated-binding audits

## Next Task

Execute Card 105's storage and registered-domain transition.
