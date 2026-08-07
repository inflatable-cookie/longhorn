# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

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
- Friction: five `@poodle/*` entries in `devDependencies` and `overrides`
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
