# 198 The Proofs Nobody Can Run

Status: ready
Owner: Tom
Roadmap: g02 cross-cutting
Governing refs: contract 012; the packaged-proof convention
Depends on: none
Auto-start next card: no

## Why

**Nineteen of the twenty `effigy` commands documented in `examples/` READMEs do
not exist.** Only `qa` resolves. Measured 2026-08-13 against `effigy.toml`.

Found while writing a README for a new packaged proof: following the
convention meant copying `effigy proof-windowing-build` from the windowing
proof, and that task is not defined. Checking the rest showed it was not one
stale line.

## Two Different Faults

They want different answers, and treating them as one problem produces the
wrong fix for half of them.

### 1. Renamed, not removed — the artifact proofs

`proof:app-shells`, `proof:bridge-topology`, `proof:bridge-topology-artifacts`,
`proof:command-system-artifacts`, `proof:history-tree-artifacts`,
`proof:native-content-artifacts`, `proof:settings-composition`,
`qa:northstar:g01-greenfield-card125`.

Every one has a live `scripts/verify-*.ts` that runs inside `proof:artifacts`,
which `qa` runs on every gate. **The capability is intact; only the documented
name is gone.** Consolidating them into one loop was reasonable and nobody
updated the READMEs.

Cheapest honest fix: point each README at `effigy proof:artifacts`, or at the
`bun scripts/verify-<name>.ts` invocation it now is.

### 2. Genuinely absent — the packaged Tauri proofs

`build:native-content-backing-surface-production-proof`,
`build:native-content-child-view-proof`,
`build:native-content-isolated-window-proof`,
`proof-transfer-build-direct`, `proof-transfer-build-surface`,
`proof-transfer-check-direct`, `proof-transfer-check-surface`,
`proof-windowing-build`, and the three matching `verify:*`.

These have no script equivalent and are not in `proof:artifacts`. There is no
way to build or run any of them from a documented command.

**And four of the six cannot be bundled even by hand.** Only
`tauri-windowing-proof` and `tauri-update-proof` carry an `icons/icon.png`;
`tauri build` fails on the missing icon for the other four.

## What Makes This Worth A Card

`effigy qa` is green. It stays green because `cargo check` and `clippy` compile
these crates happily — `generate_context!` only demands an icon when bundling —
and because the artifact proofs still run under their new name.

So the gate reports health over a layer where **the packaged proofs cannot be
packaged and the documented commands do not exist**. Being run as a real
application is the entire reason a packaged proof exists; a packaged proof that
only compiles is a unit test with a window it never opens.

That is the same shape as the defect g02.009 found in `observe_install`: a
check passing for reasons unrelated to the claim it appears to make.

## The Decision This Card Carries

For the six packaged proofs, one of:

- **Restore the tasks and the icons.** They stay proofs and become runnable.
  Costs an icon per proof and a task per proof, and someone has to confirm each
  still passes.
- **Retire the ones that have served.** A proof that has produced its evidence
  and will not be re-run is a maintenance cost pretending to be a safety net.
  Its recorded finding is the artifact worth keeping, not its source.
- **Keep them compiling only, and say so.** Legitimate if the evidence is
  already recorded, but the READMEs must stop implying a run that cannot happen.

Do not answer this per proof by default. The interesting question is how many
of the six anyone intends to run again.

## Steps

- [ ] Decide the above for the six packaged proofs.
- [ ] Correct the eight artifact-proof READMEs to a command that exists.
- [ ] Whatever is chosen, make the drift catchable: a check that every
      `effigy <task>` named in a README resolves. Nineteen dead commands
      accumulated silently, and only writing a twentieth surfaced it.

## Acceptance

- [ ] Every `effigy` command in an `examples/` README resolves, or the README
      no longer names one.
- [ ] A new README naming a non-existent task fails the gate.
- [ ] For each packaged proof: it builds and runs, or its README says plainly
      that it does not and why.

## Evidence

- [ ] The count before and after, by the same measurement.
- [ ] For any proof retired, where its recorded evidence now lives.

## Stop Conditions

- Stop if restoring a packaged proof means reviving a claim nobody will check
  again. Retiring it and keeping its finding is the better trade, and this card
  should not turn into six resurrections by momentum.

## Continuation

None. This is maintenance of the proof layer rather than a milestone step.
