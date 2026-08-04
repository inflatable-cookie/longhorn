# 148 Dependency Refresh Sweep

Status: complete
Owner: Tom
Roadmap: g02.007 batch 1
Governing refs: contracts 001, 004, 010, and 012; g02 candidate runway
Depends on: none
Auto-start next card: no
Completed: 2026-08-04

## Objective

Close compatible transitive drift and give each held-back pin a
bump-or-retain decision backed by its conformance suite.

## Scope

- workspace `cargo update` (compatible ranges only)
- `rusqlite =0.31.0` -> 0.40.x with SQLite adapter and grouped-restore
  conformance
- `zip =5.1.1` -> evaluation against deterministic backup-archive
  publication (byte-exact archives gate the bump)
- `ts-rs =11.0.0` -> 12.x with full bindings regeneration and fixture diff
- `sha2 0.10.9` -> 0.11 evaluation (digest stability, trait surface)

## Steps

1. Apply `cargo update`; run workspace tests and Clippy.
2. Bump rusqlite; run SQLite native-snapshot, grouped-restore, and storage
   suites.
3. Evaluate zip: bump behind the archive-determinism regression; retain
   with reason if byte-exactness breaks.
4. Bump ts-rs; regenerate all bindings; require zero unexplained fixture
   diffs.
5. Evaluate sha2 0.11; digests must remain identical.
6. Record each decision; full `effigy qa`.

## Acceptance Criteria

- compatible drift applied and green
- four recorded bump-or-retain decisions with conformance evidence
- golden fixtures and archives unchanged or explicitly regenerated
- full `effigy qa` passes

## Evidence Required

- per-crate decision records with suite receipts
- QA receipts

## Stop Conditions

- a bump forces a storage-format, protocol, or archive-layout change
- rusqlite API migration exceeds the adapter seams

## Evidence

- rusqlite 0.40.1 and zip 7.2.0 (the 1.85 MSRV ceiling) bumped with
  green conformance; sha2 0.11
  bumped behind one explicit hex seam with frozen digest fixtures proving
  identical output; ts-rs at 11.1.0 with the 12 major deferred to its own
  card (605-site generator API change)
- tauri 2.11.5 via compatible drift; one mock-origin test fix for the
  hardened remote-origin ACL
- transitive drift bounded by the Rust 1.85 gate: darling, plist,
  serde_with, time, idna_adapter, writeable, and the icu family MSRV-pinned
  to their newest 1.85-compatible releases
- 149 workspace suites, Clippy, fmt, bindings checks, and full `effigy qa`
  green
- log: `docs/logs/2026-08/04-dependency-refresh-sweep.md`

## Next Task

Promote Card 149 (g02.008).
