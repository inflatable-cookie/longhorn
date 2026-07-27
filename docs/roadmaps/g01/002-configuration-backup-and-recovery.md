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
- debounce and explicit flush in a later bounded card
- failure-injection tests

Status: active; coordinated atomic mutation card ready

### 3. Backup and restore

- domain inventory and checksummed manifest
- staged publish, retention, pre-migration backup
- inspect, safety backup, staged restore, atomic commit, receipt
- secure-store exclusion and custom adapters

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

Execute
[002 Coordinated Atomic Configuration Mutation](batch-cards/002-coordinated-atomic-configuration-mutation.md)
when implementation resumes. Compile debounce and explicit flush separately
after it closes. Backup, restore, and consumer conformance remain later cards.
