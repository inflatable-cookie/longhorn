# g01.002 Configuration, Backup, And Recovery

Status: complete
Owner: Tom  
Updated: 2026-07-28  
Governing refs: contract 004

## Outcome

Ship the foundation store and path policy used by every later durable Longhorn
system.

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

Status: complete; cards 005 through 010 complete

### 4. Consumer conformance

- Loophole machine/windowing fixture
- Soundcheck settings/window fixture
- Bovine workspace preference fixture

Status: complete as card 010

### 5. Storage layout profiles and transition

- immutable storage identity and platform-directory facts
- native, unified, and portable versioned profiles
- fixed bootstrap locator and layout diagnostics
- journaled profile transition and declarative legacy import
- database placement by lifecycle with native adapters for live stores

Status: complete; cards 011 and 012 complete

## Acceptance

- contract 004 acceptance passes
- crashes and invalid archives cannot replace known-good state
- all three consumers map without shared product schemas
- backup contents and exclusions are inspectable before restore
- one profile selection resolves every ordinary root on all three platforms
- profile changes preserve one recoverable authority and retain the source

## Completion

[012 Storage Profile Transition And Legacy Import](batch-cards/012-storage-profile-transition-and-legacy-import.md)
closed the final storage-layout gate. `g01.003` and Card 017 are complete;
Card 018 is the sole ready `g01.004` lane.
