# g02.026 Automation And Gate Convergence

Status: in progress — 219 and 220 landed in full 2026-08-15 (workflow edits
approved); Card 218's closure operator-held on Poodle v0.2.0
Owner: Tom
Updated: 2026-08-14
Governing refs: contract 012; memo 023
Depends on: none; Card 218's final closure waits on the Poodle v0.2.0 release
that unblocks g02.014

## Outcome

The gates mean what they claim. CI and local qa check the same things, the
MSRV has one declaration, the release gates are cheap-first and defined once,
an advisory cannot land invisibly, and the linked-Poodle exemption that makes
local evidence possible cannot silently ride into a release.

## Generation Runway

Memo 023's automation lane found no broken gate — it found gates that drift
because they are transcribed by hand (`ci.yml`'s 13 binding domains vs
`effigy.toml`'s 15), guards wired into nothing (`check:runner-tools`), and one
deliberate exemption (linked Poodle) with no release-time assertion that it is
off. Contract 012's distribution claims depend on this envelope being honest.

## Planning Gaps

- **Poodle v0.2.0 is the operator-named precondition for publication.**
  Card 218 builds the assertion machinery now (fail release on
  `linkedPoodleAccepted`, integrity-bytes verification, pack-level typecheck
  against registry Poodle) so the moment v0.2.0 publishes, the exemption dies
  by gate rather than by memory. Its last acceptance criterion is held until
  then.
- **Release-runner effigy version** (0.9.1 pinned vs 0.11.0+local) — whether
  to bump the pin or assert parity is a small tooling call inside Card 219,
  recorded there.

## Execution Plan

### Batch 1. The exemption's exit

- [ ] [Card 218](batch-cards/218-linked-poodle-exit-gate.md): release gates
  fail when `linkedPoodleAccepted` is true; `poodle-release.ts` verifies
  integrity bytes, not a version string; `ci-rehearse` sees bun global-link
  state; pack-level typecheck of `longhorn-poodle-svelte` against registry
  Poodle. Final criterion held for Poodle v0.2.0.

### Batch 2. One declaration per fact

- [x] [Card 219](batch-cards/219-gate-hygiene-and-single-sourcing.md): lazy
  `poodleRoot` plus documented `POODLE_REPO`; `check:runner-tools` joins `qa`;
  `ci.yml` routes through effigy selectors; MSRV single-sourced from
  `rust-toolchains.env` with a `Cargo.toml` cross-check; duplicate
  `[release]`/`[release.gates]` collapsed, ordered cheap-first; `docs:rust`
  joins a gate; doctor made cheap again; release-runner effigy pinned in
  lockstep; pack tooling converged; stale scripts docs fixed. **Landed
  2026-08-14, with holds**: the gate passes without `POODLE_REPO` (a stacked
  doc-drift failure under the import crash was fixed too), runner-tools is in
  qa with a widened scan, the MSRV is single-sourced, release config has one
  home, and rustdoc is gated. The `ci.yml` selector routing, action pins,
  `npm@latest`, and effigy-pin lockstep are workflow edits, held for
  approval. Doctor keeps its floor deliberately; the pack split and the
  hand-maintained docs path lists are recorded as chosen.

### Batch 3. Seeing the supply chain

- [x] [Card 220](batch-cards/220-supply-chain-visibility.md): `deny.toml` with
  the 13 known unmaintained advisories explicitly allowed and dated, wired to
  CI; committed `gen/schemas` policy decided and gated; CI toolchain pinned;
  workflow actions pinned; `npm install -g npm@latest` out of the
  publish-rights step. **Landed 2026-08-14, with holds**: `deny.toml` carries
  the full known set (14 advisories — the audit undercounted), gated in
  `[release.gates]`; gen/schemas stay committed by recorded decision; the
  redundant workspace excludes were removed. Toolchain/action pins and the
  `npm@latest` removal are workflow edits, held for approval.

## Dependency Shape

```text
memo 023 (C1-residual, H3, H4, M-ci, M-MSRV, M-dup-gates, M-doctor,
          M-effigy-version, M-pack-path, M-advisories, L1-L10 DX lane)
 └─ 026 automation and gate convergence
     ├─ 218 linked-poodle exit   (operator-held closure on Poodle v0.2.0)
     ├─ 219 gate hygiene         (independent)
     └─ 220 supply-chain gates   (independent)
```

219 and 220 are independent; 218's machinery lands now and its closure joins
the g02.014 critical path.

## Goals

- [ ] a drifted gate list fails compilation of the gate, not silently
- [ ] `effigy qa` plus release gates cover everything release confidence needs
- [ ] the exemption that keeps local development possible cannot pass a
  release unnoticed
- [ ] a future vulnerability-class advisory is visible the day it publishes

## Acceptance Criteria

- [ ] `effigy release gates` passes on a machine without `POODLE_REPO` set
- [ ] adding a bindings domain touches one list, not three
- [ ] bumping the MSRV touches one file and one cross-check, not fifteen
- [ ] `cargo deny check advisories` runs in CI with a reviewed allow list

## Explicit Non-goals

- Publishing anything. g02.014 owns publication; this milestone makes its
  evidence honest.
- Changing `.github/workflows/` trigger policy — push/PR absence is
  deliberate and documented; workflow edits still need explicit approval.

## Next Task

Card 219. It fixes the gate that currently fails for the wrong reason (H3)
and the guard that runs nowhere (H4) — both cheap, both load-bearing for
every other card in this suite.

## Planning Checkpoint

After Batch 2. If selector-routing `ci.yml` surfaces tasks that only exist
locally (effigy version split-brain), that resolves before Card 218's exit
gate is trusted to mean the same thing on the runner.
