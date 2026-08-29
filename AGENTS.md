# Longhorn Agents

Longhorn is a pre-1.0 private workspace of shared Rust and Svelte/TypeScript
systems for Tauri desktop applications. It owns reusable desktop state,
host integration, and orchestration; five sibling apps own product policy and
consume it. Publication is deferred.

This file applies to the whole repository. `CLAUDE.md` is only the bridge to
this file; keep that bridge as `@AGENTS.md`.

## Start Here

Use Effigy for repository work:

```sh
effigy tasks
effigy doctor
effigy test --plan
effigy qa
```

`doctor` is cheap orientation: it runs `fmt:rust`, `check:runner-tools`, and
built-in scans without compiling the workspace. `effigy qa` is the full
Rust/TypeScript gate. `effigy test` uses the built-in test plan, which runs
Cargo nextest and therefore does not cover the doc tests counted by `qa`.
The MSRV floor is a release gate. Before a release, run
`effigy ci:rehearse`; it checks clean-runner properties that local `qa` can
miss and is a full rehearsal.

Read the relevant project context before changing a boundary:

- `docs/README.md`
- `docs/vision/README.md`
- `docs/architecture/system-architecture.md`
- `docs/contracts/contract-index.md`
- `docs/roadmaps/generation-index.md`
- `docs/logs/README.md`

Prefer `effigy <task>` for supported work. Do not mirror Effigy tasks into
package scripts. A missing target-local maintenance task is a routing gap;
use the installed Northstar consumer-safe route rather than substituting
commands from another repository.

## Boundaries

- Longhorn owns generic Rust, TypeScript, Svelte, host adapters, and their
  checked cross-language contracts.
- Consumer apps own product schemas, panel catalogues, commands, history
  payloads, resources, workflows, and authorization. Do not move that
  authority into Longhorn.
- Poodle owns visual primitives and presentation tokens. Integrate through
  public APIs; do not copy or fork Poodle components here.
- Donor code is evidence, not automatically the shared contract. Preserve it
  until the bounded migration and its conformance evidence close.
- Cross-project audits are read-only unless a named migration batch authorizes
  consumer writes.

## Working Posture

- Longhorn is pre-1.0: do not add compatibility aliases, deprecated stubs,
  re-export shims, or silent fallbacks. Coordinate material consumer breaks.
- Stop when a missing contract, contradictory evidence, unresolved product
  choice, or changed package boundary prevents safe progress.
- Normal tasks use the current checkout. Do not start a worker loop or create
  another worktree unless an explicit handoff authorizes that route.
- Bare `continue` authorizes the next bounded card or planning batch named by
  the roadmap front doors. Do not invent work when no lane is ready.
- A change is complete when behavior, affected docs and fixtures, validation,
  and any governing batch evidence agree; name unresolved limits.

## Repository Edges

- Effigy first. Use Bun for TypeScript automation and Bash only as thin glue.
- Rust/TypeScript wire changes require the governing contract and fixtures to
  move with them. Keep generated bindings in the owning TypeScript package.
- Stage explicit paths; never use `git add -A` in a shared checkout. Check
  moves in the working tree, not only in the index. Prefer one branch per
  concurrent thread.
- For a repo-wide rename, use a hyphen-free token when it is also a Rust name.
  Text and identifier changes need separate substitutions; run
  `cargo check --workspace` before committing.
- Do not edit `.github/workflows/` without explicit human approval.
- During a redaction sweep, treat `scripts/` as executable code. A placeholder
  that reads well in prose can be a runtime failure.
- When a small, solvable execution hurdle appears, append a terse friction,
  impact, plausible fix, and affected surface to `PAPERCUTS.md` before
  continuing. Do not stop or fix it unless the current scope includes the fix.

## Validation

Use the narrowest relevant Effigy selector while working, then run
`effigy qa` after a solid batch. For docs or instruction changes, run the
relevant docs links/path checks and inspect the final diff. Never claim a
green doctor result is full validation.

## Style

Use the compressed internal style in
`docs/policy/internal-writing-style.md`: short, blunt, high signal, and clear
about outcome, current state, failed validation, and next move.

<!-- northstar:rust-quality:start -->
## Northstar Rust Quality

Scope: Rust source, Cargo manifests, build files, tests, and directly related
documentation under this directory.

Use Northstar's strict everyday-authoring route for ordinary Rust work. Resolve
the repository-owned profile and deviations under `docs/contracts/`; never
assume a universal MSRV. Re-enter at task start and coherent batch closeout.
Preserve unrelated work. A quality audit, no-slop pass, or audit-and-fix request
is explicit audit intent; never route it through everyday authoring.
<!-- northstar:rust-quality:end -->

<!-- northstar:typescript-quality:start -->
## Northstar TypeScript/Svelte explicit audit

Use Northstar's TypeScript/Svelte quality pack only when the operator explicitly
requests a TypeScript or Svelte quality audit, no-slop pass, whole-codebase
review, or audit-and-fix action. Ordinary TypeScript/Svelte coding does not
activate it.

For explicit audit intent, load the main Northstar router and select
`TypeScript/Svelte explicit audit-and-repair`. Resolve package ownership and
strict profile state before assessment. Record findings before mutation, keep
repairs inside recorder-authorized files, preserve pre-existing dirty work, and
use repository-owned compiler, framework, lint, and test evidence without
installing dependencies or inventing commands.
<!-- northstar:typescript-quality:end -->
