# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

### [ ] A `file:` install links files, so new files never reach consumers — 2026-08-10
- Friction: `bun install` for a `file:` dependency builds real directories
  containing one symlink per file, resolved at install time. Edits to an
  existing file are live through the link, but a file *added* to Longhorn
  afterwards has no link and simply does not exist in the consumer. Nucleus
  failed to launch on `Failed to resolve import "../generated/fields.ts"`
  while happily reading the edit that introduced that import — four of the
  eight generated field maps resolved and four did not, split exactly on
  whether they predated the install.
- Impact: adding any file to `packages/longhorn/src` silently breaks every
  consumer until it reinstalls, and the symptom points at the new file rather
  than at the install. Vite's optimized-dep cache holds the bad resolution
  too, so a plain reinstall is not always enough.
- Reproduced minimally 2026-08-10. With `file:../dep`, `node_modules/dep` is a
  real directory holding one symlink per file, so a file added to the source
  afterwards is invisible while edits to existing files are live. With
  `bun link`, `node_modules/dep` is a single symlink to the directory and a
  new file appears with no reinstall.
- Possible fix: `bun link` rather than `file:`. **Not** a `postinstall` hook —
  that fires during install, which is exactly when the symlinks are already
  correct; the breakage happens later, in Longhorn, with no install running in
  the consumer for a hook to fire from. Publishing to the registry also closes
  it, because a version bump forces a reinstall and the tarball is complete.
- Surface: consumer `package.json` `file:` deps, `packages/longhorn/src`.

### [ ] A public-readiness redaction disabled a release gate — 2026-08-10
- Friction: `6a84574c docs: remove third-party identity so the repo can be made
  public` replaced a consumer's real path with the literal placeholder
  `../<private-consumer>` in `scripts/private-candidate-card149/consumers.ts` —
  executable code, not prose. The candidate verifier has been unable to resolve
  it ever since.
- Impact: g02.008 spent weeks recorded as "operator-held on nucleus quiescence"
  while the actual blocker was that the gate could not run at all. Nobody
  noticed because the recorded reason was plausible and nobody re-ran it.
- Possible fix: the path now comes from `LONGHORN_PRIVATE_CONSUMER`, with an
  unset value recorded as a named omission. The general lesson is that a
  redaction sweep over a repository should not treat `scripts/` as prose — a
  placeholder that reads fine in a document is a runtime failure in code.
- Surface: `scripts/private-candidate-card149/consumers.ts`, any future
  redaction sweep.

### [ ] A receipt pinning five repositories goes stale silently — 2026-08-10
- Friction: Card 149's candidate receipt pins five external consumer graphs and
  Poodle's artifact set. Between one attempt to generate it and the next, four
  independent things had drifted: the TypeScript package count (18 to 3 via
  g02.013), Loophole's entire structure (restarted greenfield, old app moved to
  `loophole-legacy`), the redacted consumer path above, and Poodle's package
  set. Each failure surfaced one at a time, only when the previous one was
  fixed.
- Impact: the receipt describes a world that no longer exists, and the only way
  to discover that is to run it — which nobody does, because it is held for a
  reason that stopped being true.
- Possible fix: run it often enough to fail early, or pin fewer things. A
  compatibility claim over five moving repositories has the staleness rate of
  the fastest one.
- Surface: `scripts/private-candidate-card149/`, g02.008.

### [x] `cargo fmt --all` reformats sibling repositories — 2026-08-09
- Friction: `--all` is not workspace-scoped the way `--workspace` is. It walks
  every *local* package in the graph, so the moment `crates/longhorn-poodle`
  took a relative path dependency, `fmt:rust` started formatting Poodle — and
  failing, because Longhorn is edition 2024 and Poodle is 2021, so the same
  files pass Poodle's own gate and fail Longhorn's. Fifty-four diffs in a
  repository this change never touched. `clippy`, `test` and `doc` were
  unaffected; they all say `--workspace`.
- Impact: the fix task would have *written* to a sibling checkout another
  thread was working in. On a shared tree that is the same failure mode as
  `git add -A`, arriving through a formatter.
- Fixed 2026-08-09: `fmt:rust` derives its package list from `cargo metadata
  --no-deps`, which is genuinely members-only, and passes each as `-p`.
  `.github/workflows/ci.yml:34` and `release.yml:92` still say
  `cargo fmt --all` and need the same change; workflows are not edited without
  approval, so this is flagged rather than done.
- Surface: `effigy.toml`, `.github/workflows/ci.yml`,
  `.github/workflows/release.yml`.

### [ ] CI claims no sibling checkouts; two manifests need one — 2026-08-09
- Friction: `ci.yml` states it "exists to prove a clean clone with no sibling
  checkouts, no `[patch]` config, and no warm caches". Two manifests contradict
  it. `package.json` pins
  `"@inflatable-cookie/poodle-core": "file:../poodle/.artifacts/…"`, and
  `crates/longhorn-poodle` now takes
  `poodle-specs = { path = "../../../poodle/…" }`. Both are the sanctioned
  temporary shape while Poodle is untagged, and both need a sibling Poodle to
  resolve.
- Impact: the stated contract and the manifests disagree, so anyone reading
  `ci.yml` will believe a clean-clone build works when it cannot. It also means
  the two lanes fail for a reason unrelated to whatever change triggered them.
- Possible fix: none until Poodle is tagged — at which point every path ref
  swaps to a git ref together, npm and Cargo alike, and the claim becomes true.
  Until then the comment should say what is actually the case, so the gap is
  known rather than discovered during a release.
- Surface: `.github/workflows/ci.yml`, `package.json`,
  `crates/longhorn-poodle/Cargo.toml`.

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

### [x] `bunx effigy` in CI would run a stranger's package — 2026-08-09
- Friction: Effigy is a local binary at `~/.local/bin/effigy`, not a
  devDependency. An unrelated package named `effigy` exists on npm at `0.0.2`,
  so a workflow step written as `bunx effigy qa` fetches and executes that
  instead. Caught while drafting `release.yml`, before it ran.
- Impact: worst case in a release workflow, which holds publish rights. The
  existing `ci.yml` avoids it only because it inlines every command by hand,
  which is why the trap is not obvious — nothing documents that `bunx effigy`
  is unsafe.
- Fixed 2026-08-09: `inflatable-cookie/setup-effigy@v1` already exists and
  installs the real binary from an Effigy release; monkey has been using it
  since 0.8.17. Both release workflows now use it and call `effigy ci` /
  `effigy qa` directly, which also retires the hand-transcribed gate lists —
  a copy of a selector can drift from the selector, and this session already
  shipped one that pointed at a renamed proof.
- Remaining: the trap itself is undocumented. Nothing tells a new workflow
  author that `bunx effigy` resolves to someone else's package, and both
  `ci.yml` files still inline their commands for the same original reason.
- Surface: `.github/workflows/` in both repositories, Effigy adoption docs.

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

### [x] Heavyweight host SDKs have no in-gate home — 2026-08-09
- Friction: `gpui` cannot join the workspace without adding several hundred
  transitive crates and a Metal shader build to `lint:rust`,
  `lint:rust:features`, `test:rust` and `docs:rust`. The only alternative the
  repo offers is `prototypes/`, which is outside every gate, so the binding
  is verified by hand and can rot silently.
- Impact: the one artefact proving a host seam matches its real SDK is the
  one thing CI never builds. It rotted exactly as predicted: the render
  binary was broken by a signature change in the session that introduced it,
  and a hand-run caught it.
- Fixed 2026-08-09 by Card 172, against measurements rather than impressions.
  `gpui` is 757 packages and 3.3 GiB of linked artifacts, but 37s cold and
  5.6s warm on this machine — heavy in disk and CPU, not in wall clock. So the
  exclusion stays and a named selector covers the gap: `check:prototypes`
  runs `cargo check --all-targets --locked` over all six prototypes in 1.3s
  warm, sits deliberately outside `qa`, and is wired into both `release:gates`
  and `[release.gates]`. "The host seam still matches its SDK" is a claim that
  must hold before a tag, not before every commit.
  Proved by breaking `project_notification_stack`'s signature and watching the
  selector fail, then restoring it.
- Surface: `effigy.toml`, `prototypes/`.

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
- Friction: the SSR suites under `packages/longhorn-poodle-svelte/tests/`
  carry 15s and 20s timeouts and spend nearly all of it in transform, so they
  fail whenever the machine is busy — including inside `effigy qa` itself,
  where the Rust lanes are the competing load. Measured 2026-08-09:
  `config-svelte/ssr.test.ts` takes 6.8s alone and times out at 15s in a full
  gate run. The margin is roughly 2x and the gate routinely eats it.
- Impact: `effigy qa` fails intermittently with no real defect, and the
  failure names an SSR import check, which reads like a genuine regression.
  Three misdiagnoses so far: once read as the `@inflatable-cookie` rename (it
  was a stale `node_modules`), once as the bindings change, once as the new
  `longhorn-poodle` crate. Each cost a full re-run to disprove.
- Possible fix: raise the timeouts to a multiple of the measured cost rather
  than a round number, or give the SSR suites a serial lane so they do not
  compete with a parallel Rust build. Not done here: `packages/*` is outside
  this thread's remit.
- Surface: `packages/longhorn-poodle-svelte/tests/config-svelte/ssr.test.ts`,
  `packages/longhorn-poodle-svelte/tests/commands-svelte/ssr.test.ts`.

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
