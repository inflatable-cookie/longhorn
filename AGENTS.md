# Longhorn Agents

This file applies to the whole repository.

## Start Here

```sh
effigy tasks
effigy doctor
effigy test --plan
effigy qa
```

`doctor` runs cheap health (`fmt:rust`, `release:floor`) plus built-in scans.
Full Rust/TypeScript validation is `effigy qa`, not doctor.

Then read:

- `docs/README.md`
- `docs/vision/README.md`
- `docs/architecture/system-architecture.md`
- `docs/contracts/contract-index.md`
- `docs/roadmaps/README.md`
- `docs/logs/README.md`

Prefer `effigy <task>` for supported repo work. Do not mirror Effigy tasks into
package scripts. Use `--repo <PATH>` only for another repository.

## Working Posture

- Longhorn is pre-1.0. Do not add compatibility shims or silent fallbacks.
- Preserve unrelated consumer-repo work. Cross-project audits are read-only
  unless a migration batch explicitly names that repo.
- Do not move app-specific authority into Longhorn.
- Poodle owns visual primitives. Longhorn may integrate with them but must not
  fork them.
- A donor implementation is evidence, not automatically the shared contract.
- Stop when a missing contract or unresolved product choice changes package
  boundaries.

## Continuation

The operator has granted standing execution authority for bare `continue`.
Resume the next bounded card or planning batch named by the roadmap front
doors, promoting a planned card into execution when its governing refs and
scope are complete. Stop only for a missing contract, contradictory evidence,
failed gate, destructive external action, or product choice the recorded
evidence cannot resolve.

## Style

Use the compressed internal style in
`docs/policy/internal-writing-style.md`.
