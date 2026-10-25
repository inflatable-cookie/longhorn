# Longhorn Agents

Longhorn is a pre-1.0 private workspace of shared Rust and Svelte/TypeScript
systems for Tauri desktop applications. It owns reusable desktop state,
host integration, and orchestration; five sibling apps own product policy and
consume it. It must never become a place for product policy.

This file applies to the whole repository. `CLAUDE.md` is only the bridge to
this file; keep that bridge as `@AGENTS.md`.

## Where things live

- Current state: `docs/README.md`
- Knowledge (one owner per fact): `docs/knowledge/README.md`
- Contracts: `docs/knowledge/contracts/contract-index.md`
- Architecture: `docs/knowledge/architecture/system-architecture.md`
- How to release: `docs/knowledge/contracts/release.md`
- Retired concepts, which must not come back: `docs/knowledge/retired.toml`
- Open questions: `docs/knowledge/questions.md`
- Consumer guides and API reference (product docs): `docs/guides/`,
  `docs/reference/`

The plan (lanes, their documents and their order), leads, papercuts, brief
drafts, tasks and status live in Queue, never in this repository. Read what's
next with `plan.get` (see the `northstar` skill).

## Commands

Use Effigy for repository work:

Effigy guidance is maintained in the installed shared `effigy` Agent Skill;
do not add a Longhorn vendored copy. Longhorn owns the selectors and project
rules below. If missing, install it globally with
`npx skills add inflatable-cookie/effigy -g`, then start a fresh agent session
so the guidance is loaded.

```sh
effigy tasks
effigy doctor
effigy test --plan
effigy qa
```

`doctor` is cheap orientation: it runs `fmt:rust`, `check:runner-tools`, and
built-in scans without compiling the workspace. `effigy qa` is the full
Rust/TypeScript gate. `effigy test` uses the built-in test plan, which runs
nextest and therefore does not cover the doc tests counted by `qa`. Use
`test:rust` when doc tests are part of the check.
The MSRV floor is a release gate. Before a release, run
`effigy ci:rehearse`; it checks clean-runner properties that local `qa` can
miss and is a full rehearsal.

Prefer `effigy <task>` for supported work. Do not mirror Effigy tasks into
package scripts. A missing target-local maintenance task is a routing gap;
use the installed `northstar` skill's tasks (`effigy skill run northstar/<task>`)
rather than substituting commands from another repository.

## Rust build routing

Tom's 2026-10-25 ruling: every ordinary local Longhorn route runs Cargo through
Oxide. `scripts/rust-command.sh` is the one switch point, and it has two forms:

- `<cargo command> [args...]` — one Cargo command.
- `exec [--manifest-path <Cargo.toml>] [--profile <p>] -- <tool> [args...]` —
  one tool that calls Cargo itself, in one lease over one workspace and one
  toolchain.

Locally it runs `oxide` or `oxide exec`; with `GITHUB_ACTIONS=true` it runs plain
`cargo` or the tool itself, so hosted CI keeps the Cargo behavior it always had.
There is no per-selector `:cargo` task and no silent fallback: install Oxide, or
set `EFFIGY_OXIDE_ESCAPE=cargo` for one local run (any other value is refused),
or hand-run Cargo yourself for a one-off investigation. `fmt:rust` and the
`advisories` gate are switched too: they run non-building Cargo subcommands
(`fmt`, `metadata`, `deny`) that Oxide passes straight through.

`scripts/rust-command.ts` builds that argv for scripts and adds no rule of its
own: `rustCommand` for a Cargo command, `rustExec` for a leased tool,
`privateCargoCommand` for a leased tool that keeps private final outputs.

### Storage and lease boundaries

Oxide owns local Cargo output placement. A route that needs a private directory
keeps final outputs on external build scratch and never on the internal drive,
the account home or a drive root:

- `OXIDE_TASK_SCRATCH_ROOT`, when set, must already exist. Missing scratch fails
  the route; nothing creates it, so an unmounted build volume cannot turn into
  an ordinary directory on the internal drive.
- The default is the build volume's own `scratch` directory, which must exist;
  the repository creates only its `longhorn` child below it.
- `scripts/rust-command.ts` resolves a path before accepting it and refuses one
  inside the internal temp directory, the account home, a boot-volume top-level
  directory or a volume mount root.
- Hosted CI keeps its runner-owned temporary storage.

`oxide exec` replaces a `CARGO_TARGET_DIR` inherited on the outer call with
`<workspace>/target`, so the private directory survives only when it is set
inside the lease: `scripts/private-cargo.sh` is the tool behind the lease, it
exports `CARGO_TARGET_DIR` (and, for the two scans, `CARGO_BUILD_BUILD_DIR`) for
the plain Cargo child it runs, and it makes no routing decision of its own.
Setting either variable on the outer `exec` call, or in a proof's member
environment, does nothing. Only `CARGO_TARGET_DIR` moves final outputs: an
inherited `CARGO_BUILD_BUILD_DIR` still sends intermediates to the held slot, so
an inner target is not a second build store.

One lease covers one Cargo workspace and one toolchain. A proof that stages
several consumers, or builds at the MSRV, runs one `exec` boundary per staged
workspace: that workspace's `--manifest-path`, and `RUSTUP_TOOLCHAIN` pinned
before the lease. Inside a lease the tool calls plain `cargo`, never oxide: a
nested lease takes its own slot and can deadlock the host permits.

Staged workspaces need `prepareStagedWorkspace`: it creates an empty real
`target/` directory in the owned disposable workspace before the lease. An
existing real target opts that workspace out of Oxide's automatic first
conversion — which otherwise refuses a workspace with no Git work tree to
confirm — reports `outputs not converted`, and creates no Oxide output root.
Never apply it to a real checkout, never overwrite an existing target or an
owned-output link, and do not substitute `git init` or target-ignore metadata
for it. The conversion refusal is an Oxide defect (upstream `755757db`); this is
the supported interim rule on the installed Oxide (`4df3dfd`). Real Rust stages
live on external build scratch and are removed after use.

### Cargo entry points

| Route | Form |
| --- | --- |
| Every `lint:*`/`test:*`/`docs:*` Rust selector, `check:prototypes`, `check:bindings`, `release:floor`, `ci:rehearse` | `rust-command.sh <cargo command>` |
| `effigy fmt:rust` | `rust-command.sh fmt --check -p …` and `rust-command.sh metadata --no-deps` |
| `[release.gates] advisories` / `effigy release:gates` | `rust-command.sh deny check advisories` |
| `scripts/sync-prototype-locks.ts`, `scripts/prototype-dependency-coverage.ts`, `scripts/generate-api-reference-card126.ts` | `rustCommand(["metadata", …])` |
| Checkout-level `package --list` inventories, bindings `run` checks, `tree` reads and `-p` tests in the proofs and the two absence scans | `rustCommand([...])`, at `RUSTUP_TOOLCHAIN=<MSRV>` where the call was `cargo +<toolchain>` |
| Staged and disposable consumer builds: the `scripts/proof-artifacts.ts` members, `scripts/verify-desktop-support-conformance.ts`, `scripts/verify-agent-tool-dispatch-source-consumer.ts`, `scripts/verify-source-consumer.sh` | `rustExec(privateCargoCommand(...))`, one boundary per staged workspace, at the MSRV pin where the staged crates need it |
| `check:agent-control-release-absence`, `check:agent-tool-dispatch-release-absence` | `rustExec(privateCargoCommand(..., { buildDir: true }))` with `--profile release` |
| `cargo tauri build` in the example and prototype READMEs | documented as `oxide exec -- cargo tauri build`; source guidance only, no live Tauri run is claimed |

Intentional private-layout exceptions: the two release-absence scans read
Cargo's own `target/<profile>/deps` layout, so each pins
`CARGO_BUILD_BUILD_DIR` to its private per-feature target inside the lease and
keeps disjoint feature outputs. Those intermediates do not pool. Every other
staged consumer keeps private final outputs and pools its intermediates.

Genuine Cargo exceptions:

| Route | Category |
| --- | --- |
| `.github/workflows/*`, including its `cargo metadata` step | CI, and workflow edits need approval |
| Effigy's own `cargo update --workspace` lockfile sync from `config/release.toml` | Not a Longhorn entry point; Effigy runs it |
| `scripts/check-runner-tools.ts`, `scripts/check-release-runner-tools.ts` | Read `cargo deny`/`cargo install` as text to check workflow installs; not entry points |
| `scripts/workspace-dependencies.ts` | Reads `Cargo.toml` text; runs no Cargo |
| A consumer's own project commands in `docs/guides/` | Product Cargo behavior in the consumer's checkout, not this repository's route |
| `.git/queue-hooks` warm-merge manifest | Queue-owned, outside the repository |

Integration tests that re-exec themselves (`current_exe()` in
`longhorn-config`, `longhorn-settings-config`, `longhorn-surfaces-config`) need
no change: they re-run their own test binary, not a sibling binary beside it,
and no `CARGO_BIN_EXE_<name>` exists for these crates. They pass under `oxide`.

## Product rules

- Longhorn owns generic Rust, TypeScript, Svelte, host adapters, and their
  checked cross-language contracts.
- Consumer apps own product schemas, panel catalogues, commands, history
  payloads, resources, workflows, and authorization. Do not move that
  authority into Longhorn.
- Poodle owns visual primitives and presentation tokens. Integrate through
  public APIs; do not copy or fork Poodle components here.
- Donor code is evidence, not automatically the shared contract. Preserve it
  until the bounded migration and its conformance evidence close.
- Longhorn is pre-1.0: do not add compatibility aliases, deprecated stubs,
  re-export shims, or silent fallbacks. Coordinate material consumer breaks.

## Guardrails

- Releases need Tom's approval; follow `docs/knowledge/contracts/release.md`.
- Cross-project audits are read-only unless the operator authorizes consumer
  writes for a named migration.
- Do not edit `.github/workflows/` without explicit human approval.
- Stop when a missing contract, contradictory evidence, unresolved product
  choice, or changed package boundary prevents safe progress.
- Normal tasks use the current checkout. Do not start a worker loop or create
  another worktree unless Queue or the operator gives you one.
- Bare `continue` authorizes the next `now` lane in Longhorn's Queue plan.
  Do not invent work when nothing there is ready.
- When a change alters what is true, update the owning knowledge file in the
  same PR. An operator ruling given in conversation goes into its owning file
  before the thread ends.
- A change is complete when behavior, affected docs and fixtures, and
  validation agree; name unresolved limits.

## Repository edges

- Effigy first. Use Bun for TypeScript automation and Bash only as thin glue.
- Rust/TypeScript wire changes require the governing contract and fixtures to
  move with them. Keep generated bindings in the owning TypeScript package.
- Stage explicit paths; never use `git add -A` in a shared checkout. Check
  moves in the working tree, not only in the index. Prefer one branch per
  concurrent thread.
- For a repo-wide rename, use a hyphen-free token when it is also a Rust name.
  Text and identifier changes need separate substitutions; run
  `oxide check --workspace` before committing.
- During a redaction sweep, treat `scripts/` as executable code. A placeholder
  that reads well in prose can be a runtime failure.
- When a small, solvable execution hurdle appears, file it in Queue as a
  papercut and carry on: from `~/Dev/projects/paseo-northstar-queue`, run
  `node bin/queue-cli.mjs papercut.add -` with the JSON payload on stdin:
  `repository: {origin: "inflatable-cookie/longhorn", path}`, `title`,
  `happened`, `impact`, and optional `area` and `fix`. Do not stop or fix it
  unless the current scope includes the fix.

## Validate

Targeted checks per task, full QA at milestones (Tom, 2026-09-30).
Longhorn does not use Northstar's compile-only option: its covering tests
and groups are cheap, and the QA-group records need them (2026-10-09).

- **A task** runs, once, the Effigy selectors for what it changed: the tests
  for the code it touched (for example `effigy test:rust`, `effigy test:ts`,
  `effigy test:vitest`, or the proof selector it changed), a compile of what
  it touched (`effigy check:ts`, `effigy lint:rust`), and `effigy qa:docs` for
  docs. Then it opens the PR. No full `effigy qa`, no repeat passes. A heavy
  proof selector the task changed, or one its brief names (for example
  `effigy proof:artifacts`), is a targeted check: run it once.
- **Prefer a bounded QA group** when one covers the change:
  `effigy --json tasks qa-groups list`, then
  `effigy --json tasks qa-group run <group> --scope <token>... --plan`, then
  the same without `--plan`. Name every input you touched as a `--scope`
  token (`path:`, `cargo-package:`, `bun-package:`, `input:`). A group runs
  all its members. `needs_planner` means stop and report to the planner; it
  never means run the full board. Groups and their limits:
  `docs/knowledge/validation-input-map.md`. Use the global `--json` prefix;
  the trailing form prints a banner first.
- **The PR records QA-group evidence:** the changed inputs, the group and
  exact `--scope` tokens, the plan result (`declared_match` or
  `needs_planner` with its reason), each run's outcome and time, admission as
  reported (`null` means unknown), any extra gap token, or why no group was
  used. The planner sends these records to Effigy.
- **Docs or instruction changes** also run
  `effigy skill run northstar/retired-concepts` and
  `effigy skill run northstar/cut -- check-links`, and inspect the final diff.
- **Full `effigy qa`** runs on `main` at release points
  (`docs/knowledge/contracts/release.md`) and after a major chunk of work, run
  by the planner.
- Use Effigy selectors, not raw `cargo`, `bun` or `vitest`: Effigy admits heavy
  runs host-wide. Never kill by pattern (`pkill -f`, `killall`); stop only
  processes you started.
- Never claim a green doctor result is full validation.

`qa:docs:agent-defaults` forbids a current-directory repo override on the
instruction surface.

## Style

Use the compressed internal style in
`docs/knowledge/contracts/writing-style.md`: short, blunt, high signal, and
clear about outcome, current state, failed validation, and next move.

## Northstar Rust Quality

Scope: Rust source, Cargo manifests, build files, tests, and directly related
documentation under this directory.

Use Northstar's strict everyday-authoring route for ordinary Rust work, run
from this checkout:

```sh
effigy skill run northstar/language:route -- --consumer <this-checkout> --language rust --workflow everyday_authoring
```

Resolve
the repository-owned profile and deviations under `docs/knowledge/contracts/`;
never assume a universal MSRV. Re-enter at task start and coherent batch
closeout. Preserve unrelated work. A quality audit, no-slop pass, or
audit-and-fix request is explicit audit intent; never route it through everyday
authoring.

## Northstar TypeScript/Svelte explicit audit

Use Northstar's TypeScript/Svelte quality pack only when the operator explicitly
requests a TypeScript or Svelte quality audit, no-slop pass, whole-codebase
review, or audit-and-fix action. Ordinary TypeScript/Svelte coding does not
activate it.

For explicit audit intent, run the installed-package route from this
directory: `effigy skill run northstar/language:route -- --consumer . --marker northstar:typescript-quality --workflow explicit_audit_repair`,
then follow the `entrypoint_path` it returns. Resolve package ownership and
strict profile state before assessment. Record findings before mutation, keep
repairs inside recorder-authorized files, preserve pre-existing dirty work, and
use repository-owned compiler, framework, lint, and test evidence without
installing dependencies or inventing commands.

<!-- oxide:begin -->
## Rust commands: use `oxide`

- Write `oxide` wherever you would write `cargo`: `oxide check`, `oxide test`, `oxide run -- <args>`, `oxide add <crate>`.
- Profiles: `quick` (default), `--profile debug` for a debugger, `--release`. Oxide places build output, so it refuses `--target-dir`.
- A `waiting for …` line means another build holds a slot; let it wait. `oxide status` shows the pool, slots and waits.
- Use `oxide clean`, not `cargo clean`.

Managed by `oxide init`; edits between these markers are replaced.
<!-- oxide:end -->
