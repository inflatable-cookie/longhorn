# Longhorn Agents

Longhorn is a pre-1.0 Rust desktop framework for Tauri and GPUI, with
Svelte/TypeScript clients for webview apps. It owns reusable desktop state,
host integration, and orchestration. [Consumer repositories](docs/knowledge/architecture/repo-authority-map.md)
own product policy.

This file applies to the whole repository. `CLAUDE.md` bridges to it with
`@AGENTS.md`.

## Where things live

- Docs entry point: `docs/README.md`
- Knowledge (one owner per fact): `docs/knowledge/README.md`
- Contracts: `docs/knowledge/contracts/contract-index.md`
- Architecture: `docs/knowledge/architecture/system-architecture.md`
- How to release: `docs/knowledge/contracts/release.md`
- Open questions: `docs/knowledge/questions.md`
- Consumer guides and API reference (product docs): `docs/guides/`,
  `docs/reference/`

The plan (lanes, their documents and their order), leads, papercuts, brief
drafts, tasks and status live in Nucleus, never in this repository. Read what's
next with `plan.get` (see the Nucleus skill).

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

`doctor` is cheap orientation: it runs `health` (`fmt:rust`,
`check:runner-tools`, `check:release-gates`, `check:markers`,
`check:generated-sources`, `check:agent-control-skill`) and built-in scans
without compiling the workspace. `effigy qa` is the full
Rust/TypeScript gate. `effigy test` uses the built-in test plan, which runs
nextest. `test:rust` runs Cargo's workspace unit, integration and doc tests
in one route. The focused PTY and Git workspace selectors use nextest for isolation
and timeouts. The workspace route also exercises PTY sessions in one process.

`check:ts` typechecks the packages, repository scripts and agent-control
proof drivers. `check:svelte` checks the adapter templates and treats warnings
as errors. `lint:ts`, `fmt:ts`, and `check:knip` run in `qa`. `fmt:ts`
uses Node for Oxfmt, including its Svelte formatter.
`check:machete` requires local cargo-machete and runs in the
`rust-dependencies` QA group.

The MSRV floor is a release gate. Before a release, run
`effigy ci:rehearse`; it checks clean-runner properties that local `qa` can
miss and is a full rehearsal.

Prefer `effigy <task>` for supported work. Do not mirror Effigy tasks into
package scripts. A missing target-local maintenance task is a routing gap;
use the installed `northstar-docs` skill's tasks (`effigy skill run northstar-docs/<task>`)
rather than substituting commands from another repository.

## Rust build routing

Use Effigy selectors for ordinary work. Their local Rust routes use Oxide
through `scripts/rust-command.sh`. When editing a route, follow the
[routing table and storage rules](scripts/README.md#rust-routing).
The same section explains one-off Cargo investigations and the hosted CI
exception.

Integration tests that re-exec themselves (`current_exe()` in
`longhorn-config`, `longhorn-settings-config`, `longhorn-surfaces-config`,
`longhorn-git-workspace`) need no change: they re-run their own test binary, not
a sibling binary beside it, and no `CARGO_BIN_EXE_<name>` exists for these
crates. They pass under `oxide`.

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
- Having no callers today never justifies removing public surface. Before
  proposing any removal, read
  [docs/reference/held-surface.md](docs/reference/held-surface.md) (held and
  retained surface) and the contracts, and record a new keep decision there.
  The register owns the detail.

## Guardrails

- Releases need Tom's approval; follow `docs/knowledge/contracts/release.md`.
- Cross-project audits are read-only unless the operator authorizes consumer
  writes for a named migration.
- Do not edit `.github/workflows/` without explicit human approval.
- Stop when a missing contract, contradictory evidence, unresolved product
  choice, or changed package boundary prevents safe progress.
- Normal tasks use the current checkout. Do not start a worker loop or create
  another worktree unless Nucleus or the operator gives you one.
- Bare `continue` authorizes the next `now` lane in Longhorn's Nucleus plan.
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
- When a small, solvable execution hurdle appears, file it in Nucleus as a
  papercut and carry on: run `nucleus item.add papercut "<title>"` with the
  Markdown body on stdin, naming the repository (`inflatable-cookie/longhorn`
  and the path), what happened, the impact, and optionally the area and a
  suggested fix. A worker without engine credentials records the same papercut
  in the PR body and the planner files it (Nucleus papercut `i-b00a333d`).
  Do not stop or fix it unless the current scope includes the fix.
- Changelog fragments are `changelog.d/<task-or-PR>-<slug>.<type>.md`, with
  `type` one of `added`, `changed`, `fixed`, `removed` or `security`; breaking
  content is `changed`, and one fragment holds one entry. Check them with
  `effigy skill run northstar-docs/changelog -- check`. Owner:
  [release](docs/knowledge/contracts/release.md#changelog-fragments).

## Validate

Targeted checks per task, full QA at milestones (Tom, 2026-09-30).
Longhorn does not use Northstar's compile-only option: its covering tests
and groups are cheap, and the QA-group records need them (2026-10-09).

- **A task** runs, once, the Effigy selectors for what it changed: the tests
  for the code it touched (for example `effigy test:rust`, `effigy test:ts`,
  `effigy test:vitest`, or the proof selector it changed), a compile of what
  it touched (`effigy check:ts`, `effigy lint:rust`), `effigy fmt:rust` once
  when it touches Rust sources, and `effigy qa:docs` for docs. Then it opens
  the PR. No full `effigy qa`, no repeat passes. A heavy
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
- **Static gate changes** use the `typescript-static` or
  `rust-dependencies` QA group and the proofs for any changed generated or
  packed output. Keep scope and evidence in the PR.
- **Docs or instruction changes** also run
  `effigy skill run northstar-docs/check-links`, and inspect the final diff.
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

The Oxide guidance below applies to direct Rust commands during one-off
investigations. Use Effigy wherever a Longhorn selector covers the work.

<!-- oxide:begin -->
## Rust commands: use `oxide`

- Write `oxide` wherever you would write `cargo`: `oxide check`, `oxide test`, `oxide run -- <args>`, `oxide add <crate>`.
- Profiles: `quick` (default), `--profile debug` for a debugger, `--release`. Oxide places build output, so it refuses `--target-dir`.
- A `waiting for …` line means another build holds a slot; let it wait. `oxide status` shows the pool, slots and waits.
- Use `oxide clean`, not `cargo clean`.

Managed by `oxide init`; edits between these markers are replaced.
<!-- oxide:end -->
