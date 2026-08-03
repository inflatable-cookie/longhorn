# 128 Grouped Adapter Restore Contract And Protocol

Status: complete
Owner: Tom
Roadmap: g01.019 batch 1
Governing refs: contracts 001, 004, and 012; Nucleus contract 032 and g05.046
Depends on: Card 127
Auto-start next card: yes

## Objective

Freeze one generic grouped custom-adapter restore protocol before mutation code.

## Scope

- grouped failure-atomic capability separate from single-domain claims
- exact group selection and confirmation digest
- side-effect-free stage, apply, observe, and rollback payload boundary
- durable journal and boot-catalogue recovery rules
- execution, recovery, and receipt types in `longhorn-config`

## Out Of Scope

- Nucleus code or SQLite product policy
- renderer protocol or settings UI
- package publication
- ordinary file-restore redesign

## Steps

1. Reconcile contract 004 with the live single-domain adapter API.
2. Define the minimum object-safe grouped adapter extension.
3. Define bounded owned stage payloads and semantic evidence.
4. Define group plan, execution, recovery, and receipt outcomes.
5. Record journal authority and consumer quiescence boundaries.

## Acceptance Criteria

- one confirmation binds archive, exact set, adapters, previews, and evidence
- group capability cannot be inferred from `FailureAtomic`
- all recovery inputs are durable or reconstructable from the exact catalogue
- adapter policy and app lifecycle stay downstream
- Cards 129-131 can execute without a fresh package-boundary decision

## Evidence Required

- promoted contract and architecture text
- compiled g01.019 runway
- public type/API skeleton with compile-time object-safety proof
- focused docs QA

## Stop Conditions

- recovery needs live renderer state
- portable rollback requires a Longhorn-owned database schema
- group participation would silently change existing adapters

## Next Task

Card 129 implements complete staging, the durable journal, apply, verification,
and exact rollback.

## Evidence

- contract 004 and system architecture freeze the grouped authority boundary
- `BackupAdapterGroupedRestore` is object-safe and opt-in
- grouped plan, execution, recovery, error, and receipt types are public
- existing `Separate` participation remains distinct
