# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

### [ ] Peered packages need a consumer override under `file:` refs — 2026-08-08
- Friction: `longhorn-poodle-svelte` and `longhorn-tauri` declare
  `@inflatable-cookie/longhorn` as a peer at `0.1.0`. A consumer that installs
  longhorn as `file:../longhorn/packages/longhorn` does not satisfy that peer
  by itself, so bun reaches for the registry and 404s. Nucleus and soundcheck
  happened to carry `overrides` already; jetstream did not and failed to
  install until one was added.
- Impact: every new consumer hits a confusing registry 404 for a package that
  is sitting on disk beside them, and the fix is not discoverable from the
  error.
- Possible fix: it disappears once longhorn publishes and consumers depend by
  version. Until then, the getting-started guide should show the override
  block alongside the `file:` dependencies rather than leaving it implicit.
- Surface: `docs/guides/getting-started.md`, consumer manifests.

### [ ] Poodle 0.1.0 does not export `SplitToggleVisibility` — 2026-08-08
- Friction: `@inflatable-cookie/poodle-svelte` defines `SplitToggleVisibility`
  in `src/types.ts` and uses it for `SplitView`'s `toggleVisibility` prop, but
  its `index.ts` never re-exports it — unlike `SplitOrientation`,
  `ControlDensity`, and the rest, which are all public.
- Impact: a consumer typing that prop must either reach past the package root
  (contract 012 forbids it) or mirror the union. `LayoutSplitView.svelte` now
  derives it via `ComponentProps<typeof SplitView>["toggleVisibility"]`, which
  works but is a workaround for a one-line upstream gap.
- Possible fix: add the export to Poodle's root barrel, then drop the derived
  alias here.
- Surface: `packages/longhorn-poodle-svelte/src/poodle/LayoutSplitView.svelte`,
  poodle `packages/svelte/components/src/index.ts`.

### [ ] `split.test.ts` asserts a Poodle attribute the pinned pack lacks — 2026-08-08
- Friction: Card 161 mapped region-hidden to SplitView `hidden` rather than
  `collapsed`, and the test asserts `data-primary-hidden`. The pinned pack
  renders `data-primary-collapsed` and has no hidden attribute, so the test
  fails. Confirmed failing at `HEAD` in a worktree, so it is not merge drift.
- Impact: `test:vitest` has one standing failure with no defect behind it, and
  it reads as a regression to anyone who has not checked `HEAD`.
- Possible fix: nothing to do here — it clears when longhorn moves onto a
  released poodle. Worth listing because it is the second failure in one
  session that looked caused by the current change and was not.
- Surface: `packages/longhorn-poodle-svelte/tests/poodle/split.test.ts`,
  Card 161.

### [ ] SSR vitest suites flake under machine load — 2026-08-08
- Friction: `packages/commands/tests-svelte/ssr.test.ts` and
  `packages/config/tests-svelte/ssr.test.ts` carry 20s and 15s timeouts. Both
  spend nearly all of it in transform/collect, so they fail when the machine
  is busy — e.g. `effigy test:vitest` run alongside a workspace clippy.
- Impact: `effigy test:vitest` fails intermittently with no real defect. It
  cost two misdiagnoses in one session: once read as caused by the
  `@inflatable-cookie` rename (it was a stale `node_modules`), once as a
  regression from the bindings change (it was load).
- Possible fix: raise the two timeouts, or mark the SSR suites as a serial
  lane so they do not compete with a parallel Rust build.
- Surface: `packages/commands/tests-svelte/ssr.test.ts`,
  `packages/config/tests-svelte/ssr.test.ts`.

### [ ] Endpoint URL validation duplicated across capability crates — 2026-08-07
- Friction: `longhorn-update::EndpointUrl` and `longhorn-licence::ActivationUrl`
  independently parse and validate an HTTPS URL. The rules differ on purpose
  (update allows loopback HTTP for a local shim; activation does not, because
  its requests carry credentials), but the parsing is the same thirty lines
  twice.
- Impact: a parsing bug fixed in one is not fixed in the other. The IPv6
  bracket case was caught in `longhorn-update` by a test; nothing guarantees
  the licence side gets the same scrutiny.
- Possible fix: promote a shared URL primitive when a third caller appears.
  Not `longhorn-core` today — two callers do not justify growing core an
  HTTP concept, and coupling two optional capability crates so one cannot be
  composed without the other is worse than the duplication.
- Surface: `crates/longhorn-update/src/source.rs`,
  `crates/longhorn-licence/src/activation.rs`.

### [ ] Root package.json pins poodle to machine-local build artifacts — 2026-08-06
- Friction: five `@inflatable-cookie/poodle-*` entries in `devDependencies` and `overrides`
  resolve to `file:../poodle/.artifacts/g12.016-A698XB/packs/*.tgz` — a
  poodle build-output directory outside this repo, dated 2026-07-29. The
  first CI run to reach `bun install --frozen-lockfile` failed on all five
  with `ENOENT`, having never run a single check.
- Impact: the committed manifest is unresolvable on any machine but this
  one, so the whole TypeScript CI lane is dead and a fresh clone cannot
  install. Blocks tagging v0.1.0.
- Possible fix: decided 2026-08-06 to block the tag on a poodle release
  rather than vendor the packs or drop the CI coverage — poodle gets a tag
  and longhorn consumes it by version. Poodle currently has no tags.
- Surface: `package.json`, `bun.lock`, `.github/workflows/ci.yml` clients
  job, portfolio distribution strategy.

### [ ] Effigy doctor launches full QA by default — 2026-08-06
- Friction: the standard `effigy doctor` route followed Longhorn's `health = [{ task = "qa" }]` mapping and started the full Rust/TypeScript validation suite during repository orientation.
- Impact: a health check becomes a long-running execution path, obscures the intended diagnostic output, and can consume substantial local resources before the agent has selected a bounded task.
- Possible fix: give `health` a cheap diagnostic baseline and expose the full suite as an explicit QA task, or make the doctor output state clearly that it is about to run the full suite.
- Surface: `effigy.toml` health task / Effigy doctor workflow

### [ ] MSRV-gated Clippy lints surface late — 2026-08-06
- Friction: raising the declared floor (1.85 -> 1.90 -> 1.95) each time
  unlocked new Clippy lints on pre-existing code (`collapsible_if`,
  `question_mark`), discovered only when a release gate ran, not when the
  floor changed.
- Impact: two unplanned fix rounds mid-release-prep; a floor bump silently
  carries a lint debt that surfaces at the worst moment.
- Possible fix: run `cargo clippy --workspace --all-targets -- -D warnings`
  as an immediate step in any floor change, before committing the bump.
- Surface: tagged-release runbook, `scripts/check-release-floor.sh`.

### [ ] Toolchain floor lived in 15 proof scripts, not the manifest — 2026-08-06
- Friction: raising `rust-version` in `Cargo.toml` left 34 effigy gate
  invocations and 15 TypeScript proof scripts still pinning the old
  toolchain; the scripts are what actually enforce the floor, and a stale
  pin only failed once code used newer language features.
- Impact: the declared floor and the enforced floor can silently disagree.
- Possible fix: single source for the pinned toolchain (the new
  `release-baselines/rust-toolchains.env`) that scripts and gates read,
  rather than a literal repeated per call site.
- Surface: `effigy.toml`, `scripts/*.ts`, `release-baselines/`.

### [ ] Candidate receipt freezes consumer graphs, coupling unrelated repos — 2026-08-06
- Friction: the Card 127/149 receipt asserts clean selected manifests across
  seven consumer repositories, so an unrelated in-flight line in a consumer
  blocks a Longhorn-side freeze indefinitely.
- Impact: Card 149 has waited on nucleus, then soundcheck; the window where
  every consumer is simultaneously quiescent is rare and shrinking.
- Possible fix: separate the artifact identity claim (tag + source-consumer
  gate, Longhorn-only) from the cross-repo compatibility claim (receipt), so
  the former never waits on the latter.
- Surface: `scripts/private-candidate-card149/consumers.ts`, Card 149.
