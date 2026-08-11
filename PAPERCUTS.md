# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

### [ ] A green local gate says nothing about a clean runner — 2026-08-11
- Friction: Longhorn's release workflow ran for the first time on 2026-08-11.
  It failed six times before going green, on eight distinct defects, with
  `effigy qa` green here throughout. Four properties of a developer machine
  were doing the work, and every defect came from one of them: a sibling Poodle
  checkout, a cargo cache still holding versions the lockfile no longer names,
  `CI` unset so tool output arrives uncoloured, and ripgrep installed.
- Impact: ten to thirty minutes of CI per defect, found one at a time, during a
  release. Worse than the cost is the false reading — a fully green board on a
  repository that could not be cloned and built.
- The four reproductions, all cheap once known:
  - `git clone` to a directory with no sibling Poodle, then run the gate there.
  - `CARGO_HOME=$(mktemp -d) cargo fetch --locked`, then run the proofs against
    it. `--offline` only fails where a stale pin is absent from the cache.
  - `CI=1 effigy proof:artifacts`. picocolors enables colour whenever `CI` is
    set, with no terminal involved, so piping output is not enough to get plain
    text.
  - Grep for tools a runner lacks. `rg` was the only one, in two places, and
    the second was in a shell script after a sweep that only read TypeScript
    spawn calls had declared the first to be the last.
- Possible fix: fold the four into one `effigy ci:rehearse` task, so the check
  before a release is a command rather than a recollection. Each is minutes;
  together they are still far cheaper than one failed run.
- Two of the eight were duplicated helpers rotted in the copies — the workspace
  dependency table in seven proofs, and a vitest summary regex in five. Both
  are single sources now. A duplicated helper only diverges where nobody looks,
  and nobody looks locally.
- One defect hid another twice over. The missing `rg` in check-release-floor.sh
  failed a pipeline under `set -o pipefail`, which the `if !` read as the
  toolchain being absent — and the toolchain *was* absent, because release.yml
  installed only stable. A wrong error message that happens to be true is worse
  than one that is plainly wrong.
- Surface: `effigy.toml`, `scripts/`, `.github/workflows/release.yml`.

### [ ] `check:bindings` cannot catch a generator that emits an undeclared type — 2026-08-10
- Friction: Card 177 added `SurfacePresentation` to the Rust surface model and
  regenerated. `packages/longhorn/src/surfaces/generated/protocol.ts` then
  *referenced* `SurfacePresentation` in four places while declaring it nowhere,
  because the generator's declaration list had not been extended.
  `effigy check:bindings` passed. It compares generated output against
  committed output, so when the generator is wrong both sides agree and the
  gate is satisfied by a file that does not compile.
- Impact: the natural loop after a protocol change — `generate:bindings`, then
  `check:bindings` — reports success on a broken package. `effigy check:ts`
  does catch it, so `qa` is not blind, but the bindings gate alone reads as
  authoritative and is not.
- Fix: make the generator refuse to emit. It already knows enough — it prints
  `tagged unions not in the field map: SurfacePresentation, …` as a warning at
  exactly the moment it should be failing. Every type name referenced in an
  emitted declaration should resolve to a declaration in the same file or a
  known primitive, and anything else should be an error rather than a line of
  output nobody reads.
- Not: adding `check:ts` to `check:bindings`. That makes the gate slower and
  still only catches the class after the fact; the generator is the place that
  knows.

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
