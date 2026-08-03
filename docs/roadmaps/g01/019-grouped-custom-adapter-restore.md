# g01.019 Grouped Custom-adapter Restore

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001, 004, and 012; Nucleus contract 032 and g05.046
Depends on: g01.002 and Card 127 compatibility checkpoint

## Outcome

Add one portable failure-atomic transaction for a selected set of custom
restore adapters. Bind one archive and confirmation to the whole set, stage
everything before mutation, journal every target, roll back exactly, and
recover during boot without moving application policy into Longhorn.

## Generation Runway

This is an interrupt lane discovered by a real Nucleus consumer. It completes
the custom-adapter promise already present in contract 004. It does not edit
Nucleus, publish packages, or displace g01.017; Card 070 stays ready while this
lane closes the restore safety gap.

## Execution Plan

### Batch 1: Contract and protocol

- [x] freeze grouped participation, confirmation, stage, apply, verify, and receipt types
- [x] freeze Longhorn, adapter, and consumer authority boundaries
- [x] keep existing separate and single-domain adapter behavior explicit

### Batch 2: Transaction and journal

- [x] re-inspect and stage the complete set before mutation
- [x] persist bounded target and rollback payloads plus every adapter target
- [x] publish, verify, fail, and roll back the group under one coordinator

### Batch 3: Recovery and conformance

- [x] block ordinary load and mutation while grouped recovery is active
- [x] recover through the exact boot catalogue after interruption
- [x] prove stale evidence, deterministic failure, mixed adapters, and SQLite

### Batch 4: Public evidence and handoff

- [x] expose and document the public Rust API
- [x] prove existing separate adapters and ordinary file restore are unchanged
- [x] leave one exact Nucleus resume handoff without claiming its restore complete

### Batch 5: Explicit state evidence

- [x] replace optional/digest-only grouped state with explicit present/absent evidence
- [x] bind archive source state and explicit target/current evidence into confirmation
- [x] expose target/rollback state through plan and receipt projections

### Batch 6: Durable deletion and recovery

- [x] apply absent targets as zero-payload deletion and verify absence
- [x] retain rollback-to-absent independently in the versioned journal
- [x] reject every contradictory evidence/payload shape before mutation or recovery

### Batch 7: Conformance and Nucleus resume

- [x] prove mixed optional-file and WAL-mode SQLite commit and rollback
- [x] re-prove interruption during apply, verify, rollback, and boot recovery
- [x] freeze the public API baseline and leave an exact Nucleus consumer handoff

## Goals

- [x] one confirmation covers the exact sorted selected custom-domain set
- [x] all targets and rollback payloads are private and durable before mutation
- [x] any post-journal failure ends fully applied, fully rolled back, or recovery-required
- [x] boot recovery needs no renderer or live product authority
- [x] ordinary file restore and separate adapter execution keep their current contracts
- [x] archived absence is a first-class target rather than an omitted digest
- [x] target absence and rollback absence remain distinct and restart-safe

## Acceptance Criteria

- [x] an empty, duplicate, excluded, separate, changed, or unavailable group fails before mutation
- [x] stale archive, confirmation, descriptor, adapter, preview, or current evidence fails before mutation
- [x] stage failure leaves every live domain unchanged
- [x] apply or verification failure rolls every domain back exactly
- [x] interruption in applying, verifying, and rolling-back phases recovers the complete old generation
- [x] mixed grouped adapters and a WAL-mode SQLite fixture pass
- [x] group journal corruption blocks normal writes and never guesses recovery
- [x] no Nucleus source is edited and no package is published
- [x] absent target and absent rollback fixtures use zero payloads and verify absence
- [x] contradictory manifest, preview, stage, or journal presence fails closed
- [x] present-only and separate-adapter behavior remains green

## Batch Cards

Complete:

- `batch-cards/128-grouped-adapter-restore-contract-and-protocol.md`
- `batch-cards/129-grouped-adapter-restore-execution-and-journal.md`
- `batch-cards/130-grouped-adapter-recovery-and-conformance.md`
- `batch-cards/131-grouped-adapter-public-evidence-and-nucleus-handoff.md`
- `batch-cards/135-grouped-adapter-explicit-state-contract.md`
- `batch-cards/136-grouped-adapter-absence-transaction-and-recovery.md`
- `batch-cards/137-grouped-adapter-absence-conformance-and-nucleus-handoff.md`

## Planning Checkpoint

Cards 128-131 established the grouped primitive. Nucleus g05.046 then exposed
the target-absence gap. Cards 135-137 close that state-model defect without
moving Nucleus lifecycle or domain policy into Longhorn.

## Next Task

Resume Nucleus g05.046 from the explicit absence consumer handoff. No Longhorn
card auto-starts.
