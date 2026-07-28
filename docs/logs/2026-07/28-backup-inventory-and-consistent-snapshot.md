# Backup Inventory And Consistent Snapshot

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added registry-driven include, exclude-with-reason, and custom-adapter
  policy without adding backup behavior to product domains
- added explicit operation scope and stable domain-id inventory
- default-excluded secret, cache, runtime, and log domains
- added bounded strict version-1 manifest types, source evidence, SHA-256,
  immutable payloads, and capture receipts
- captured exact ordinary source bytes or absence under the existing
  store-wide coordinator
- released coordination before returning the immutable snapshot

## Source Semantics

Current and older-valid source remains exact. Older source is validated through
the existing in-memory migration path but is neither rewritten nor replaced by
the migrated value.

Missing source is recorded as absent. Defaults are not materialized.

Readable future, corrupt, mismatched, invalid, or unmigratable source is
retained as `source-preserved` with a typed issue. It is not presented as
ordinary-restorable state. Unreadable source fails the complete capture.

## Bounds And Coordination

Capture uses finite caller lock policy plus configurable per-domain and total
byte limits below hard in-memory ceilings. Aggregate arithmetic is checked.
Length and SHA-256 cover exact retained bytes.

Inventory and ordinary reads share one coordinator acquisition. A
helper-process fixture proves cooperating mutation times out during capture
and succeeds after release.

Pending debounced intent is not part of persisted authority. A fixture proves
capture sees published bytes before forced flush and flushed bytes after it.

## Evidence

- focused backup acceptance suite covers policy completeness, duplicates,
  safe defaults, descriptor drift, stable order, exact digest and length,
  absent, current, older-valid, future, corrupt-readable, unreadable, size
  limits, pending debounce, guard release, and helper-process coordination
- checked-overflow unit proof
- strict unknown-field, version, bounded metadata, digest, and payload-path
  parsing
- Rust 1.85 workspace check
- clean format, clippy, Effigy doctor, test plan, workspace tests, and full QA

## Boundary

This batch adds no ZIP, compression, archive publication, retention, restore,
age, custom adapter execution, Tauri, async runtime, TypeScript, Svelte, or
Poodle dependency.

## Posture

`strict-ready`

Card 005 is complete. Card 006 is the only ready implementation lane.

## Next

Execute card 006: strict ZIP encoding, safe inspection, atomic publication,
and retention. Do not start it from card 005.
