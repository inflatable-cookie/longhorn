# g01.002 Configuration, Backup, And Recovery

Status: active  
Owner: Tom  
Updated: 2026-07-28  
Governing refs: contract 004

## Outcome

Ship the foundation store used by every later durable Longhorn system.

## Batches

### 1. Domain store

- storage classes, injected roots, domain registry, codecs, defaults
- typed diagnostics and future-schema protection
- sequential migrations and validation

Status: complete

### 2. Safe mutation

- unique same-directory temporary files, flush, atomic replacement
- stable store-wide coordination, patch transactions, durability receipts
- bounded debounce and explicit flush
- failure-injection tests

Status: complete

### 3. Backup and restore

- domain inventory and checksummed manifest
- staged publish, retention, pre-migration backup
- inspect, safety backup, staged restore, atomic commit, receipt
- secure-store exclusion and custom adapters

Status: paused on archive, encryption, snapshot, and atomic-restore contract

### 4. Consumer conformance

- Loophole machine/windowing fixture
- Soundcheck settings/window fixture
- Bovine workspace preference fixture

## Acceptance

- contract 004 acceptance passes
- crashes and invalid archives cannot replace known-good state
- all three consumers map without shared product schemas
- backup contents and exclusions are inspectable before restore

## Current Gate

Research and promote the decisions named by
[004 Backup Archive And Restore Contract](batch-cards/004-backup-archive-and-restore-contract.md).
Backup implementation and consumer conformance remain later cards.
