# 004 Backup Archive And Restore Contract

Status: paused on backup contract  
Owner: Tom  
Roadmap: g01.002 batch 3  
Governing refs: contracts 001, 004, and 012  
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
- safety backup, staged validation, and all-or-none declared restore set
- secure-store exclusion and custom adapter boundary
- encrypted archive policy without moving key authority into ordinary config
- machine-readable backup and restore receipts

## Contract Gate

Contract 004 does not yet answer:

- archive container, compression, deterministic layout, and format versioning
- whether encryption wraps the archive or individual domain payloads
- key acquisition, rotation, unavailable-key, and recovery behavior
- snapshot lock lifetime and treatment of unreadable or recovering domains
- how large-domain streaming interacts with one store-wide coordination guard
- retention ordering when clocks disagree or manifests are corrupt
- atomic restore mechanics across multiple domain files
- rollback behavior after a restore publication fails partway
- migration timing during inspect, staging, and commit
- destination authority for operational backup versus user export
- custom adapter participation in snapshot, verification, and restore

No audited donor proves a complete portable archive and atomic multi-domain
restore protocol. Implementation would otherwise invent persistence guarantees
inside the card.

## Required Promotion

Promote these decisions into contract 004 before implementation:

- archive and manifest format
- compression and encryption boundary
- key-authority and secret-exclusion rules
- consistent snapshot and coordination lifetime
- corruption, partial availability, and retention behavior
- staged restore transaction and rollback protocol
- migration and custom-adapter participation
- receipt and inspection shape

## Out Of Scope

- implementation, archive dependencies, or format prototypes
- user-facing backup/settings UI
- cloud synchronization or server replication
- secure-store provider selection
- consumer migration
- configuration debounce and flush

## Ready When

- every contract-gate question has one promoted answer
- archive compatibility and encryption claims are versioned and testable
- snapshot and restore compose over the existing coordination authority
- all-or-none restore has a concrete crash/failure model
- secret and custom-adapter boundaries are explicit
- acceptance covers corruption, unavailable keys, partial failure, retention,
  migration, rollback, and inspection
- implementation is compiled as one or more separate bounded cards

## Next Task

Research archive, encryption, consistent-snapshot, and atomic restore options.
Promote the selected protocol into contract 004. Do not implement backup while
this card is paused.
