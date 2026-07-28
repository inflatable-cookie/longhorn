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
- inspect, safety backup, staged restore, journaled failure-atomic commit
- secure-store exclusion and custom adapters

Status: ready; cards 005 through 009

### 4. Consumer conformance

- Loophole machine/windowing fixture
- Soundcheck settings/window fixture
- Bovine workspace preference fixture

Status: planned as card 010

## Acceptance

- contract 004 acceptance passes
- crashes and invalid archives cannot replace known-good state
- all three consumers map without shared product schemas
- backup contents and exclusions are inspectable before restore

## Current Gate

Execute
[005 Backup Inventory And Consistent Snapshot](batch-cards/005-backup-inventory-and-consistent-snapshot.md).
Later cards own ZIP publication, restore planning, journaled recovery, age
encryption, and custom adapter conformance.
