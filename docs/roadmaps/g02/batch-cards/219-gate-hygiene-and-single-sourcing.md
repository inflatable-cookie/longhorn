# 219 Gate Hygiene And Single-sourcing

Status: complete — workflow edits held for approval
Completed: 2026-08-14
Owner: Tom
Roadmap: g02.026 batch 2
Governing refs: contract 012; contract 001; memo 023 (H3, H4, M-ci, M-MSRV,
M-dup-gates, M-docs:rust, M-doctor, M-effigy, M-pack, L1-L10 DX lane)
Depends on: none
Auto-start next card: no

## Objective

Every gate fact has one declaration, every guard runs somewhere automatic,
and the release gates are defined once, cheap-first.

## Why this exists

The automation lane found no broken gate — it found drift by transcription:

- `private-candidate` hard-fails without undocumented `POODLE_REPO` via an
  import-time throw in code the script never uses
  (`scripts/private-candidate-card127/support.ts:9-16`);
  `scripts/README.md:44` claims no script takes a `*_REPO` override.
- `check:runner-tools` is wired into nothing automated — only manual
  `ci-rehearse.sh:38`. The guard against the release-run-6 failure class can
  regress silently.
- `ci.yml` hand-transcribes gates and has drifted: 13 binding domains vs 15
  (`:93-98` vs `effigy.toml:53`), a laxer svelte-check threshold (`:91` vs
  `effigy.toml:47`), hardcoded MSRV while `release.yml` reads the baseline.
- The MSRV has ~15 hardcoded copies across proof scripts; nothing derives
  them from `release-baselines/rust-toolchains.env` or cross-checks
  `Cargo.toml:63`. `LONGHORN_CURRENT_STABLE` is read by nothing.
- `effigy qa` is defined twice as a release gate (`config/release.toml:41`,
  `effigy.toml:141`); `[release]` keys are duplicated across both files; the
  45s floor runs before the 16ms env check.
- `docs:rust` is in no gate. `health` maps to the floor (two clippy passes +
  full tests at MSRV) — doctor is not cheap, contradicting its own comment
  and AGENTS.md.
- `release.yml:95-97` pins setup-effigy 0.9.1; local is v0.11.0+local.
- qa packs with `bun pm pack --dry-run`; release packs with `npm pack`.
- `scripts/README.md` is stale (twelve vs thirteen proofs; names deleted
  `poodle-evidence.ts`); `generate-api-reference-card126.ts:39` names a task
  that does not exist and hardcodes `0.1.0`; `check:runner-tools` scans only
  `scripts/`+`.github/`; `verify-documented-commands.ts` scans only
  `examples/`; `test-packages.sh` comments name deleted packages;
  `effigy test` (nextest) and `test:rust` (cargo test) differ in doc-test
  coverage; `qa:docs:*` lists are hand-maintained.

## Scope

- `effigy.toml`, `config/release.toml`, `.github/workflows/ci.yml` (with
  approval), `release-baselines/`, the named scripts
- one decision on the release-runner effigy version

## Steps

1. `support.ts`: `poodleRoot` becomes a lazy function; `POODLE_REPO` is
   documented in `scripts/README.md`; the false `*_REPO` claim corrected.
2. `check:runner-tools` joins `qa`; its scan covers `effigy.toml` and
   `private/`-shaped surfaces, not just `scripts/`+`.github/`.
3. `ci.yml` routes through effigy selectors like `release.yml`; the
   transcribed lists are deleted. (Workflow edit — explicit approval first.)
4. MSRV: proof scripts read `release-baselines/rust-toolchains.env`; a check
   cross-validates against `Cargo.toml` `rust-version`; `LONGHORN_CURRENT_STABLE`
   is consumed or deleted.
5. Collapse the duplicated `[release]`/`[release.gates]` definitions into one
   file; order gates cheap-first.
6. `docs:rust` joins `qa` or a release gate (Card 217's gpui README fix
   follows whichever). `health` maps to genuinely cheap checks; the floor
   stays in release gates.
7. Release-runner effigy: bump the pin or assert feature parity — decide and
   record. Converge pack tooling on one executable, or record why the two
   paths must differ.
8. Fix the stale scripts docs: README counts and module names, the
   nonexistent task reference, the `0.1.0` hardcode, the test-packages
   comment, the nextest/cargo-test divergence (pick one runner or document
   the split), `qa:docs:*` lists generated or scoped honestly.

## Do Not

- Change CI trigger policy — manual dispatch is deliberate and documented.
- Single-source by generating TOML from TOML with a new tool. Reading one
  file from the other is enough.

## Acceptance Criteria

- [x] `effigy release gates` passes without `POODLE_REPO` set
- [ ] adding a bindings domain touches one list — **held**: the drift lives in
  `ci.yml`'s transcription; the workflow edit needs approval (see Result)
- [x] bumping the MSRV touches one file plus one cross-check
- [x] `effigy qa` includes runner-tools; rustdoc is gated (release gates)
- [x] `effigy doctor` cost re-checked — kept deliberately, reason recorded

## Evidence Required

- the gate runs, before/after
- the deduplicated release config
- `effigy qa` and `effigy release gates` green

## Stop Conditions

Stop on any `.github/workflows/` edit — explicit human approval first, per
the Effigy footguns.
