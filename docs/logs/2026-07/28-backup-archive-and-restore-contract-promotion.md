# Backup Archive And Restore Contract Promotion

Date: 2026-07-28
State: complete research and planning batch

## Outcome

- audited Soundcheck library backups and DAW restore, Loophole corrupt-source
  preservation and project export, plus negative evidence in Nucleus,
  Jetstream, and Bovine
- selected a strict standard ZIP inner bundle with normalized metadata,
  DEFLATE, SHA-256, bounded readers, and an exact declared entry inventory
- separated operational backup roots from explicit user export destinations
- defined bounded published-state capture under the existing store guard
- separated external snapshot consistency groups from ordinary configuration
- selected whole-archive binary age v1 with injected key authority
- made locked archives non-prunable and distinct from corrupt archives
- defined inspect-bound confirmation, stale-plan detection, and side-effect-free
  migration staging
- replaced an impossible multi-file atomicity claim with journaled
  failure-atomic terminal state and mandatory crash rollback
- compiled cards 005 through 010; only card 005 is ready

## Donor Findings

Soundcheck supplies the strongest invariants: exact bytes, explicit absence,
content hashes, target compatibility, preview binding, stale-plan detection,
pre-restore safety capture, staged writes, post-restore verification, rollback,
SQLite-native snapshots, integrity checks, milestone retention, and
pre-migration backups.

Its database and DAW models remain consumer authority. Its multi-file restore
has no crash journal, and its library backup is one SQLite file rather than a
portable domain archive.

Loophole preserves invalid configuration with best-effort timestamped renames.
That supports source preservation but not a reusable backup protocol.

## Protocol

The inner extension is `.longhorn-backup`; encrypted output is
`.longhorn-backup.age`. Manifest version 1 records app, producer, consistency,
domain state, schemas, sizes, SHA-256, payloads, and exclusions.

Operational retention only deletes successfully inspected same-app archives.
It preserves the new archive, pins, locked files, damage, unknown versions,
foreign files, and unparseable material.

Restore stages and validates everything first. A durable journal and private
old-byte set exist before the first live replacement. Success verifies all
new state. Ordinary failure verifies all old state. A crash or failed rollback
blocks normal writes until verified rollback.

## Boundary

Longhorn does not claim one atomic visibility event across independent files.
Single-domain publication remains old-or-new. Consistent cross-domain readers
use store coordination.

Pending debounced intent is not persisted state. Hosts flush explicitly before
backup when required.

Custom adapters join a restore transaction only when they can preserve,
journal, publish, verify, and roll back exact state. SQLite uses a
database-native snapshot rather than copying live main/WAL files.

## Posture

`strict-ready`

Card 004 is complete. Card 005 is the only ready implementation lane.

## Next

Execute card 005: backup inventory, policy, bounded consistent capture, and
manifest model.
