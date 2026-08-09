# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

### [ ] Cross-repo dependency exists for npm only — 2026-08-09
- Friction: Longhorn consumes Poodle through pinned npm tarballs with recorded
  SHA-256s and a membership-hashed set id — a considered mechanism with a real
  integrity claim, derived from the root manifest so it cannot rot. There is no
  Rust equivalent. `grep -rn poodle --include=Cargo.toml` across Longhorn
  returns nothing, and Poodle's Rust crates are path dependencies that reach no
  further than Poodle.
- Impact: blocks Card 169 outright. The projection tier's dependency direction
  is correct and its shape is settled, and it still cannot be started, because
  there is no route from a Longhorn crate to `poodle-specs` that survives CI.
  Any future Rust-level sharing between the two repositories hits the same wall.
- Possible fix: publish Poodle's Rust contract crates to crates.io, which is
  the consistent parallel now that g02.014 is taking both repositories to
  public npm. Otherwise extend the artifact-pinning model to Rust, which means
  building distribution machinery for one dependency edge.
- Surface: contract 012, `scripts/poodle-evidence.ts`, Poodle's Rust packages.

### [ ] Repo-wide renames need to be language-aware — 2026-08-09
- Friction: the `bovine` -> `split-shell` rename was applied as a text
  substitution across the repository and hit Rust identifiers, which cannot
  contain a hyphen. `let bovine = ...` became `let split-shell = ...` in five
  places, so two crates stopped compiling and `effigy qa` was red for several
  hours. The thread that ran it had already moved on and did not know.
- Impact: a red gate that describes nobody's current change, and which every
  other concurrent thread has to triage before it can trust its own results.
  Twenty-one of the twenty-six hits were string literals and correct; only
  five were wrong, so the noise ratio made it look worse than it was.
- Possible fix: follow any repo-wide rename with `cargo check --workspace`
  before committing, and prefer a hyphen-free identifier when the new name
  will appear in code as well as in data. A rename that changes a token used
  as both an identifier and a string needs two substitutions, not one.
- Surface: multi-thread working practice, rename tooling.

### [ ] Concurrent threads in one repository undo each other — 2026-08-09
- Friction: three agents were working across Longhorn and Poodle at once. One
  committed mid-way through another's file moves and restored paths that were
  staged for deletion; only a gitignored directory survived untouched. The
  loss was silent — `git grep` searches the index, so a check that looked like
  confirmation reported success against stale state.
- Impact: work redone twice, and a real risk of one thread committing
  another's half-finished changes when staging with `git add -A`.
- Possible fix: give each thread a branch, or stagger them. Failing that,
  stage by explicit path and never `git add -A` in a shared checkout, and
  verify file moves against the working tree rather than the index.
- Surface: multi-thread working practice.

### [x] Greenfield proof froze the tree it was meant to describe — 2026-08-09
- Friction: `verify-greenfield-card125.ts` asserted `git diff --quiet
  <frozen-commit>` across every TypeScript package and every Rust crate, so any
  change to any crate failed an unrelated gate. It blocked the GPUI thread,
  and it had already been rebaselined once during the package consolidation.
  A second check threw when a *sibling* repository was dirty, which made
  Longhorn's `qa` depend on whether anyone was mid-edit in Poodle.
- Impact: two of three concurrent threads stalled on a gate describing neither
  of their changes, with rebaselining the fixture as the only remedy.
- Fixed 2026-08-09: the frozen-source comparison is gone and cleanliness is
  recorded in the report rather than thrown on. The composition claims —
  inventories, per-shape graphs, audits, mounted tests — are computed from the
  current tree and unchanged, so nothing about correctness was traded away.
  The artifact set ids are emitted as evidence rather than asserted, since
  every one is a hash over packed contents and asserting them re-freezes the
  tree through the back door. Cleanliness belongs to a release gate, where a
  tag must name exact clean commits.
- Surface: `scripts/verify-greenfield-card125.ts`,
  `fixtures/greenfield/card125/composition-matrix-v1.json`.

### [ ] `bunx effigy` in CI would run a stranger's package — 2026-08-09
- Friction: Effigy is a local binary at `~/.local/bin/effigy`, not a
  devDependency. An unrelated package named `effigy` exists on npm at `0.0.2`,
  so a workflow step written as `bunx effigy qa` fetches and executes that
  instead. Caught while drafting `release.yml`, before it ran.
- Impact: worst case in a release workflow, which holds publish rights. The
  existing `ci.yml` avoids it only because it inlines every command by hand,
  which is why the trap is not obvious — nothing documents that `bunx effigy`
  is unsafe.
- Possible fix: note it in the Effigy adoption guidance, or publish a stub
  under the real name. Until then every workflow inlines its selectors and
  says why.
- Surface: `.github/workflows/release.yml` in both repositories, Effigy docs.

### [x] A new crate silently staleness-fails an unrelated gate — 2026-08-09
- Friction: adding `crates/longhorn-gpui-windowing` turned `check:api-reference`
  red, because `docs/reference/api-surface.md` enumerates every crate
  directory and asserts the count. The failure surfaces during `effigy qa`,
  several steps after the change that caused it, and its message names a
  generator selector rather than "you added a crate".
- Impact: every new crate costs an unexplained red gate and a hunt for the
  right regenerate command.
- Fixed 2026-08-09 for the second half. `verify-guides-card126.ts` hardcoded
  "Rust 41, TypeScript 18" and then "42, 3"; it now derives both counts from
  `crates/` and `packages/`, so adding a crate no longer reddens it, and the
  message names the regenerate command. `check:api-reference` still requires
  the document to be regenerated — that part is correct, since a stale
  inventory is a real defect — but it now fails alone rather than dragging an
  unrelated proof with it.
- Surface: `scripts/generate-api-reference-card126.ts`, `effigy.toml`.

### [ ] Heavyweight host SDKs have no in-gate home — 2026-08-09
- Friction: `gpui` cannot join the workspace without adding several hundred
  transitive crates and a Metal shader build to `lint:rust`,
  `lint:rust:features`, `test:rust` and `docs:rust`. The only alternative the
  repo offers is `prototypes/`, which is outside every gate, so the binding
  is verified by hand and can rot silently.
- Impact: the one artefact proving a host seam matches its real SDK is the
  one thing CI never builds.
- Possible fix: an Effigy selector that builds excluded prototypes on a
  slower cadence than `qa` — nightly, or as a release gate — so they are
  checked without taxing every run.
- Surface: `effigy.toml`, `prototypes/`, `.github/workflows/ci.yml`.

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

### [x] Poodle 0.1.0 does not export `SplitToggleVisibility` — 2026-08-08
- Friction: `@inflatable-cookie/poodle-svelte` defines `SplitToggleVisibility`
  in `src/types.ts` and uses it for `SplitView`'s `toggleVisibility` prop, but
  its `index.ts` never re-exports it — unlike `SplitOrientation`,
  `ControlDensity`, and the rest, which are all public.
- Impact: a consumer typing that prop must either reach past the package root
  (contract 012 forbids it) or mirror the union. `LayoutSplitView.svelte` now
  derives it via `ComponentProps<typeof SplitView>["toggleVisibility"]`, which
  works but is a workaround for a one-line upstream gap.
- Fixed 2026-08-08 in Poodle Card 020: the export was added to the root
  barrel. The derived alias here can be dropped whenever someone is in the
  file; it costs nothing to leave.
- Surface: `packages/longhorn-poodle-svelte/src/poodle/LayoutSplitView.svelte`,
  poodle `packages/svelte/components/src/index.ts`.

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
