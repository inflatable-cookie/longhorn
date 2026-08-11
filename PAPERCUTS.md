# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

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

## Closed

### [x] A `file:` install links files, so new files never reach consumers — 2026-08-10
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
- Fix (2026-08-11): `docs/guides/getting-started.md` documents the trap and
  the portfolio path — from the consumer, `effigy deps link bun ../longhorn`
  (not raw `bun link`) so each package is a directory symlink and new files
  appear without reinstall. Re-link after `bun install`; unlink when done.
  Publishing by version also closes it (reinstall gets a complete tarball).
- Not: a consumer `postinstall` hook (fires at install time; breakage is later).
- Surface: `docs/guides/getting-started.md`, consumer `file:` deps.

### [x] Peered packages need a consumer override under `file:` refs — 2026-08-08
- Friction: `longhorn-poodle-svelte` and `longhorn-tauri` declare
  `@inflatable-cookie/longhorn` as a peer at `0.1.0`. A consumer that installs
  longhorn as `file:../longhorn/packages/longhorn` does not satisfy that peer
  by itself, so bun reaches for the registry and 404s. Nucleus and soundcheck
  happened to carry `overrides` already; jetstream did not and failed to
  install until one was added.
- Impact: every new consumer hits a confusing registry 404 for a package that
  is sitting on disk beside them, and the fix is not discoverable from the
  error.
- Fix (2026-08-11): `docs/guides/getting-started.md` now shows the
  `dependencies` + `overrides` pair for sibling `file:` installs, and states
  that the override is required until consumers depend by published version.
- Surface: `docs/guides/getting-started.md`, consumer manifests.

### [x] `check:bindings` cannot catch a generator that emits an undeclared type — 2026-08-10
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
- Fix (2026-08-11): `longhorn-bindings` `apply()` now runs
  `assert_protocol_references_resolve` on every `protocol.ts` artifact before
  write/check. Referenced PascalCase names must resolve to a local
  `export type`, an `import type { … }` name, or a TypeScript builtin; comments
  and camelCase field interiors are ignored. Tagged-union field-map skips stay
  warnings — those types are declared, just not flat-mapped.
- Not: adding `check:ts` to `check:bindings`.
- Surface: `crates/longhorn-bindings/src/generation.rs`.
