# 010 Custom Backup Adapters And Consumer Conformance

Status: complete
Owner: Tom
Roadmap: g01.002 batch 4
Governing refs: contracts 001, 003, 004, and 012; research memo 006
Auto-start next card: no

## Objective

Prove the backup and restore contracts across ordinary donor-shaped
configuration plus an externally snapshotted database without moving consumer
schemas or transaction authority into Longhorn.

## Scope

- capability-declared custom capture and restore adapter interfaces
- coordinated-bounded, external-snapshot, and excluded consistency modes
- adapter payload path, size, verification, and receipt integration
- explicit refusal when exact rollback cannot be proven
- SQLite native-snapshot fixture without copying live main/WAL files
- Loophole machine/window/layout configuration fixture
- Soundcheck settings/window configuration fixture
- Bovine workspace-presentation fixture
- mixed ordinary/external consistency-group inspection
- consumer mapping and adoption notes without donor writes

## Public Behavior

Adapters retain domain and external transaction authority. Longhorn sequences
and reports them through declared capabilities.

An adapter joins a failure-atomic restore set only when it can stage target
state, preserve exact current state, journal its operation, verify publication,
and verify rollback. Otherwise it is excluded or receives a separate explicit
nontransactional operation and receipt.

An external snapshot is independently consistent by default. It cannot claim
the ordinary configuration capture instant without a consumer-supplied
higher-level authority.

## Out Of Scope

- direct changes to Loophole, Soundcheck, Bovine, or other consumer repos
- app-specific schemas, DAW state policy, or SQLite schema knowledge
- cloud/server synchronization
- settings UI

## Acceptance Criteria

- adapter capabilities determine participation without runtime guessing
- ordinary and external payload paths remain confined and declared
- a non-rollback adapter cannot enter a failure-atomic restore set
- SQLite fixture uses its backup API and verifies the result
- live SQLite WAL state survives the fixture capture
- mixed archives report separate consistency groups truthfully
- all three ordinary donor fixtures round-trip without shared product schemas
- exclusions and source-preserved states remain inspectable
- no donor repository is modified
- package graph keeps Tauri, Svelte, Poodle, and consumer code outside core

## Stop Conditions

- Longhorn must understand a donor schema
- a live database must be copied as ordinary files
- an adapter can claim rollback without verification
- an external snapshot is reported as one coordinated cut without authority
- work expands into consumer migration

## Next Task

Card 012 is ready for profile transition and legacy import. Do not auto-start
it.
