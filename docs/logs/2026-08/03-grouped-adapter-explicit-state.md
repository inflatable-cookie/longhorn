# Grouped Adapter Explicit State

Date: 2026-08-03
Roadmap: g01.019
Cards: 135-137
State: complete; Nucleus consumer work remains

## Result

Grouped custom-adapter restore now carries one explicit semantic state through
inspection, confirmation, plan, stage, journal, apply, verify, rollback,
recovery, and receipts:

- `Absent`
- `Present { sha256 }`

`BackupAdapterInspectRequest` includes the verified archive source state.
Archive absence must produce absent target evidence and zero archive payloads.
Archive presence must produce present target evidence and one or more payloads.
The same zero/non-empty rule governs staged target and rollback payloads.

Apply and verify requests both include `Target` or `Rollback` plus exact
expected evidence. Target deletion and rollback to absence therefore remain
distinct without a sentinel payload or digest. Execution and recovery receipts
retain both states per domain.

## Durability

Grouped journal version 2 stores explicit target and rollback evidence.
Recovery validates evidence against payload presence before resolving or
calling an adapter. Unsupported versions, corrupt state, or contradictory
evidence remain recovery-required and block normal authority.

## Conformance

- archived optional-file absence commits as deletion beside a WAL-mode SQLite target
- a mixed failure restores a newly created file to exact prior absence and restores SQLite
- process interruption during target apply, target verify, rollback, and boot recovery retries safely
- restart recovery restores an absent prior state and receipts it explicitly
- contradictory archive preview, staged payload, and journal shapes fail closed
- absent targets stay unavailable to separate and single-domain failure-atomic adapters
- present-only grouped fixtures, ordinary restore, separate adapters, and storage transition remain green
- the external-consumer API baseline compiles and pins exact serialized state shapes

Rust 1.85, Clippy, package inventory, binding drift, focused config, Northstar,
and aggregate workspace QA are the closeout gates. No Nucleus source was
edited. No crate, npm package, tag, or hosted release was published.

## Next

Resume Nucleus g05.046 from the explicit consumer handoff. The app still owns
offline authority lifecycle, durable restart orchestration, and product receipt
presentation.
