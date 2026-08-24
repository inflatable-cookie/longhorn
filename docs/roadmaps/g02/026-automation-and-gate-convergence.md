# g02.026 Automation And Gate Convergence

Status: in progress — 219 and 220 landed in full 2026-08-15 (workflow edits
approved); Card 218 is part-landed and needs recompiling
Owner: Tom
Updated: 2026-08-24
Governing refs: contract 012; memo 023
Depends on: nothing outstanding — Card 218's Poodle precondition is met at
public 0.2.2, which also unblocks g02.014

## Outcome

The gates mean what they claim. CI and local qa check the same things, the
MSRV has one declaration, the release gates are cheap-first and defined once,
an advisory cannot land invisibly, and the linked-Poodle exemption that made
local evidence possible cannot silently ride into a release — it is gone
rather than gated, which answers the same question.

## Generation Runway

Memo 023's automation lane found no broken gate — it found gates that drift
because they are transcribed by hand (`ci.yml`'s 13 binding domains vs
`effigy.toml`'s 15), guards wired into nothing (`check:runner-tools`), and one
deliberate exemption (linked Poodle) with no release-time assertion that it is
off. Contract 012's distribution claims depend on this envelope being honest.

## Planning Gaps

- ~~**Poodle v0.2.0 is the operator-named precondition for publication.**~~
  **Closed 2026-08-24.** The precondition was the components, not the version
  number, and public Poodle 0.2.2 ships them. The exemption died by deletion
  before any of Card 218's machinery was built, so the gate on
  `linkedPoodleAccepted` has nothing left to assert; integrity-bytes
  verification and the pack-level typecheck survive as ordinary release
  hardening.
- **Release-runner effigy version** (0.9.1 pinned vs 0.11.0+local) — whether
  to bump the pin or assert parity is a small tooling call inside Card 219,
  recorded there.

## Execution Plan

### Batch 1. The exemption's exit

- [ ] [Card 218](batch-cards/218-linked-poodle-exit-gate.md):
  `poodle-release.ts` verifies integrity bytes, not a version string;
  `ci-rehearse` sees bun global-link state; pack-level typecheck of
  `longhorn-poodle-svelte` against registry Poodle. **Held whole on
  2026-08-15**, then **part-landed 2026-08-24**: g16.008 removed the exemption
  from `effigy.toml`'s `proof:artifacts` (step 5) once public Poodle 0.2.2
  shipped `SettingsShell`, `UpdateCenter` and `UpdateStatus`. The release gate
  on `linkedPoodleAccepted` (step 1) went obsolete with the field. The three
  remaining items are release hardening under a framing that is no longer the
  exemption's exit, so the card needs recompiling before it is ready. Nothing
  else in g02.026 depends on this.

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
  qa with a widened scan, the MSRV is single-sourced, and rustdoc is gated.
  The `ci.yml` selector routing, action pins, `npm@latest`, and effigy-pin
  lockstep landed 2026-08-15 with approval. The pack split and the
  hand-maintained docs path lists are recorded as chosen.

  **Completed 2026-08-15** — two items were reported as done on 2026-08-14
  but were not: `[release.gates]` was still declared in both
  `config/release.toml` and `effigy.toml`, and because the two tables merged
  rather than conflicted, `fmt`, both Clippy passes and the full test suite
  each ran twice per release — once directly and once through `workspace =
  "effigy qa"`. The gates now live only in `config/release.toml`, seven of
  them, and a measured `effigy release gates` run is 543s with every gate
  passing. Cheap-first turned out to be unbuyable — effigy executes gates in
  name order, so both files' "cheapest first" comments were false; that is
  now written down in PAPERCUTS.md instead of asserted. And `health` still
  mapped to `release:floor`, which is
  two Clippy passes plus the full test suite at the MSRV toolchain; "doctor
  keeps its floor deliberately" recorded the contradiction rather than
  resolving it. `health` is now `fmt:rust` + `check:runner-tools` and
  `effigy doctor` finishes in about eight seconds, compiling nothing.

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
     ├─ 218 linked-poodle exit   (step 5 landed; remainder needs recompile)
     ├─ 219 gate hygiene         (independent)
     └─ 220 supply-chain gates   (independent)
```

219 and 220 are independent and complete. 218 is part-landed: the exemption
is out and g02.014 is unblocked, and what is left of the card is release
hardening off the critical path.

## Goals

- [x] a drifted gate list fails compilation of the gate, not silently
- [x] `effigy qa` plus release gates cover everything release confidence needs
- [x] the exemption that kept local development possible cannot pass a
  release unnoticed — **met by deletion 2026-08-24**, not by the gate Card 218
  planned. Public Poodle 0.2.2 removed the need for it, so there is no
  exemption to ride into a release
- [x] a future vulnerability-class advisory is visible the day it publishes

## Acceptance Criteria

- [x] `effigy release gates` passes on a machine without `POODLE_REPO` set
- [x] adding a bindings domain touches one list, not three
- [x] bumping the MSRV touches one file and one cross-check, not fifteen
- [x] `cargo deny check advisories` runs in CI with a reviewed allow list

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
locally (effigy version split-brain), that resolves before Card 218's
remaining runner-environment work is trusted to mean the same thing on the
runner.
