# 129 Grouped Adapter Restore Execution And Journal

Status: complete
Owner: Tom
Roadmap: g01.019 batch 2
Governing refs: contracts 001 and 004; Card 128
Depends on: Card 128
Auto-start next card: yes

## Objective

Implement complete pre-mutation staging and one durable grouped adapter journal.

## Scope

- exact plan and confirmation revalidation
- complete bounded target and rollback staging
- private durable payload publication
- ordered apply and full-set semantic verification
- exact group rollback and machine-readable receipts

## Steps

1. Reject invalid selection and stale plan evidence.
2. Stage every adapter under the coordinator without live mutation.
3. Validate and durably publish journal payloads before apply.
4. Apply and observe every target deterministically.
5. Roll every entry back on injected apply or verification failure.

## Acceptance Criteria

- no target mutates before every stage is valid and durable
- journal lists every target and exact old/target evidence
- failures never return a mixed terminal generation
- cleanup follows a durable terminal marker
- ordinary file and explicit single-adapter restore tests remain green

## Evidence Required

- direct grouped execution tests
- stage/apply/verify failure matrix
- journal and payload bounds tests
- focused Rust QA

## Stop Conditions

- a stage cannot represent exact rollback through bounded opaque payloads
- one adapter requires mutation before the group journal exists
- current evidence cannot be rechecked under coordination

## Next Task

Card 130 adds boot recovery, interruption fixtures, mixed adapters, and SQLite.

## Evidence

- the complete selection is re-inspected and staged under one coordinator
- target and rollback payloads plus journal are synced before mutation
- target apply uses stable order; rollback uses reverse order and full verification
- stage, apply, verify, stale-evidence, bounds, and confirmation fixtures pass
