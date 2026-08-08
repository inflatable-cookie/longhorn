# 047 Restore, Conflict, And Recovery Settings Flow

Status: complete
Owner: Tom
Roadmap: g01.008 batch 3
Governing refs: contracts 004, 005, 010, 012, and 013
Depends on: Card 046
Auto-start next card: no

## Objective

Expose non-mutating restore inspection, explicit conflict planning,
confirmation-bound execution, and recovery-required state through the shared
settings shell.

## Scope

- remaining `@inflatable-cookie/longhorn-config` restore protocol/client slice
- archive selection, unlock, inspection, domain compatibility, exclusions, and
  consistency groups
- explicit conflict choices and action plan
- confirmation digest, private staging, safety backup, execution, rollback,
  and recovery receipts
- custom-adapter participation and separate-operation outcomes
- recovery-required shell gate and retry/recover authority
- accessible destructive confirmation and progress states

## Public Behavior

Archive selection and inspection do not mutate. The page displays identity,
integrity, authenticity, domains, exclusions, migrations, conflicts,
consistency groups, and custom-adapter participation before planning.

Execution accepts only the exact current confirmation digest. A stale plan
returns to inspection. Success, verified rollback, separate adapter result,
and recovery-required are distinct terminal states.

Recovery-required blocks ordinary settings mutation through host authority; it
is not a dismissible renderer warning.

## Out Of Scope

- generic merge UI
- automatic conflict choice
- restoring secrets through ordinary archives
- bypassing safety backup
- remote restore authority
- product-specific repair tools

## Steps

1. Generate and validate restore inspection, choices, plan, staging, execution,
   adapter, rollback, and recovery protocol types.
2. Add checked clients over injected archive-selection and unlock authority.
3. Build inspection and domain/action summary state.
4. Build explicit conflict choice and confirmation-bound plan state.
5. Build staging, safety backup, execution, and exact terminal receipts.
6. Surface custom-adapter and separate-operation limits.
7. Gate ordinary mutation while recovery is active or required.
8. Mount locked, corrupt, future, conflict, stale-plan, success, rollback,
   adapter, crash-recovery, and unrecovered states.
9. Audit destructive authority, confirmation binding, secret exclusion,
   accessibility, and teardown.

## Acceptance Criteria

- inspection performs no mutation
- archive identity, integrity, authenticity, exclusions, and consistency
  groups remain distinct
- every conflict needs an explicit choice
- confirmation binds archive, actions, and current evidence
- stale confirmation publishes nothing
- complete selected set stages before live mutation
- safety backup and journal precede publication
- success, rolled back, separate adapter, and recovery required are exact
- recovery-required blocks normal mutation until verified recovery
- secret payload never appears in ordinary restore UI state
- closing the shell cannot imply cancellation after publication starts
- mounted teardown releases listeners without losing host operation authority

## Evidence Required

- restore state-machine and terminal receipt matrix
- conflict and stale-confirmation fixtures
- locked, corrupt, future, migration, and custom-adapter fixtures
- crash/rollback/recovery gate proof
- destructive confirmation and accessibility report
- payload, secret, capability, dependency, and authority audits
- Rust, TypeScript, Svelte, and Effigy QA

## Stop Conditions

- the shell must invent restore or rollback semantics
- a stale plan cannot be rejected before publication
- UI teardown would cancel or orphan host authority ambiguously
- recovery-required cannot gate later mutation
- custom-adapter outcomes would be collapsed into an ordinary-domain receipt

## Next Task

Start ready Card 048. Prove the full settings package family from produced
artifacts.

## Result

Completed 2026-07-29.

- Generated exact restore inspection, planning, execution, adapter, rollback,
  and recovery projections through a checked TypeScript barrel.
- Added narrow Tauri commands and separate read/destructive permissions over
  injected archive selection and unlock authority.
- Added independent restore page registration and a public-Poodle destructive
  flow with explicit domain choices, digest review, terminal receipts, and
  recovery gating.
- Proved locked, corrupt, future, migration, custom-adapter, stale-plan,
  success, rollback, recovery-required, and host-owned teardown behavior.
- Retained archive paths, secrets, payload bytes, executable plans, safety
  backup policy, and journals in host authority.
