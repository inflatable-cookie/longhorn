# Longhorn Agents

Longhorn is a pre-1.0 private workspace of shared Rust and Svelte/TypeScript
systems for Tauri desktop applications, consumed by five sibling apps.
Publication is deferred.

This file applies to the whole repository.

## Start Here

```sh
effigy tasks
effigy doctor
effigy test --plan
effigy qa
```

`doctor` runs cheap health (`fmt:rust`, `check:runner-tools`) plus built-in
scans, and finishes in seconds — it compiles nothing. Full Rust/TypeScript
validation is `effigy qa` — not doctor, and not `effigy test`, whose built-in
plan runs `cargo nextest` and so skips the doc tests the gate counts. The MSRV
floor is a release gate. Before a release, also run `effigy ci:rehearse` — it
exercises the clean-runner properties that local `qa` can miss, and it is a
full rehearsal, not a quick check.

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
- Repo-wide renames: prefer a hyphen-free identifier when the token is also a
  Rust name; run `cargo check --workspace` before committing. A rename that
  hits both strings and identifiers needs two substitutions, not one.
- Shared checkouts: stage by explicit path, never `git add -A`. Verify moves
  against the working tree, not `git grep` on the index. Prefer a branch per
  concurrent thread when more than one agent touches the same repo.
- Redaction sweeps: do not treat `scripts/` as prose. A placeholder that reads
  fine in a document is a runtime failure in executable code.

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
