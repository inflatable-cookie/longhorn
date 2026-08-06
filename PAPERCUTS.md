# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

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
