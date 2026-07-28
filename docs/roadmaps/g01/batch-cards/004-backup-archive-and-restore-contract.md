# 004 Backup Archive And Restore Contract

Status: complete
Owner: Tom
Completed: 2026-07-28
Roadmap: g01.002 batch 3
Governing refs: contracts 001, 004, and 012; research memo 006
Auto-start next card: no

## Objective

Close the archive, encryption, consistent-snapshot, retention, and atomic
restore decisions required before compiling backup implementation.

## Known Scope

- registry-driven domain inventory and explicit inclusion policy
- versioned checksummed manifest
- consistent multi-domain snapshot under the store coordination authority
- staged archive verification and atomic publication
- pre-migration operational backup
- retention by count and age
- inspect-before-restore compatibility report
- safety backup, staged validation, and failure-atomic terminal restore set
- secure-store exclusion and custom adapter boundary
- encrypted archive policy without moving key authority into ordinary config
- machine-readable backup and restore receipts

## Closed Contract Gate

Contract 004 now answers:

- standard ZIP inner bundle, strict version-1 manifest, DEFLATE, normalized
  metadata, bounded readers, and SHA-256
- binary age v1 over the complete inner archive
- injected key authority, locked state, rotation, and explicit re-encryption
- bounded source capture under the store guard, with compression outside it
- source-preserved recovery states and explicit unreadable failure
- external snapshot consistency groups for large/custom authorities
- safe retention over inspected manifests with pins and clock diagnostics
- journaled failure-atomic restore instead of false multi-file atomicity
- exact rollback payloads, verified rollback, and crash recovery
- inspect-time planning and execution-time in-memory migration
- injected operational roots versus explicit export destinations
- capability-declared custom adapter participation

No donor proves the full protocol. Soundcheck supplies exact-byte snapshots,
preview binding, integrity checks, safety capture, milestone retention,
verification, and rollback. External specifications close the portable
archive and encryption gaps. Research memo 006 records the translation.

## Promotion

Contract 004 now owns:

- archive, manifest, compression, encryption, and compatibility
- key authority and secret exclusion
- bounded consistent capture and external consistency groups
- corruption, partial availability, publication, and retention
- inspect-bound planning and journaled restore/rollback
- migration and custom-adapter participation
- truthful receipts and multi-file visibility limits

## Out Of Scope

- implementation, archive dependencies, or format prototypes
- user-facing backup/settings UI
- cloud synchronization or server replication
- secure-store provider selection
- consumer migration
- configuration debounce and flush

## Completion Evidence

- donor audit covers Soundcheck library and DAW restore, Loophole recovery and
  export, plus negative evidence from Nucleus, Jetstream, and Bovine
- current primary archive, encryption, rename, and SQLite snapshot sources are
  recorded
- every gate question has one promoted answer
- compatibility and encryption claims are versioned and testable
- restore has an implementable crash and failure model
- cards 005 through 010 split implementation into bounded batches

## Next Task

Execute card 005: backup inventory, policy, bounded coordinated snapshot, and
manifest model. Do not pull ZIP publication, restore, or age encryption into
that card.
