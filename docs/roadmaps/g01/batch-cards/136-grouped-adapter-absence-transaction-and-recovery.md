# 136 Grouped Adapter Absence Transaction And Recovery

Status: complete
Owner: Tom
Roadmap: g01.019 batch 6
Governing refs: contracts 001 and 004; Card 135
Depends on: Card 135
Auto-start next card: yes
Completed: 2026-08-03

## Objective

Make target deletion and rollback to absence durable, exact, and restart-safe.

## Scope

- stage payload/evidence validation
- grouped journal version and entry model
- target apply and verification
- reverse rollback and boot recovery
- corrupt and contradictory journal handling

## Steps

1. Require zero payloads for absent and non-empty payloads for present state.
2. Persist target and rollback states independently in every journal entry.
3. Apply and verify target state through the explicit request.
4. Roll back and verify prior state through a distinct explicit request.
5. Reject old, corrupt, or contradictory journals without guessing.

## Acceptance Criteria

- archived absence deletes and verifies absent
- rollback-to-absent survives restart
- contradiction fails before mutation or leaves recovery required
- target and rollback operations remain idempotent
- normal loads and writes remain blocked around incomplete recovery

## Evidence Required

- journal fixtures
- apply, verification, rollback, and recovery interruption fixtures
- fail-closed contradiction receipts
- focused transaction tests

## Stop Conditions

- recovery must infer evidence from payload count
- an adapter cannot distinguish target from rollback verification
- a journal migration would guess old absent intent

## Next Task

Card 137 closes mixed-adapter evidence and the Nucleus handoff.

## Evidence

- version-2 journal stores target and rollback evidence independently
- absent state requires zero payloads; present state requires a non-empty set
- target and rollback apply/verify requests carry kind and expected state
- archived deletion and restart rollback-to-absence fixtures pass
- apply, verify, rollback, boot-recovery interruption, and contradictory journal fixtures pass
