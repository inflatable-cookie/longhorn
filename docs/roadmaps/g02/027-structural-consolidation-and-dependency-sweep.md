# g02.027 Structural Consolidation And Dependency Sweep

Status: ready
Owner: Tom
Updated: 2026-08-14
Governing refs: contract 017; contract 001; memo 023
Depends on: none

## Outcome

Rules stated once are implemented once. The native-content generation
discipline stops diverging across three adapters, the small duplicated
primitives get one home, the repo's idioms are written down so the next audit
can check them mechanically, and the held-back dependencies move with their
conformance evidence beside them.

## Generation Runway

Memo 023's quality lanes found disciplined code with a few structural debts:
one state machine copied three times and already drifting, six hand-rolled
hex encoders, six near-identical adapter error scaffolds, and four direct
dependencies a major version behind. Pre-1.0 is exactly when these are cheap.
This milestone is the g02.007-shaped sweep applied to what the audit found.

## Planning Gaps

- **`ts-rs` unpin bumps the bindings generator's fragility.** The generator
  parses `ts-rs` output with string splitting; it works because `=11.1.0` is
  pinned. Card 223 treats the unpin as a generator-hardening task, not a
  version bump.
- **Mutation clone cost is a measurement, not a rewrite.** Operation and
  notification mutations deep-clone record vectors under 65 536-record
  ceilings. Correct and deliberate; Card 223 measures before deciding.

## Execution Plan

### Batch 1. One state machine

- [ ] [Card 221](batch-cards/221-native-content-generation-hoist.md): hoist the
  attach-generation state machine from the three mechanism adapters into
  `longhorn-native-content`; per-mechanism error enums stay; the three known
  divergences are reconciled against contract 017's single statement of the
  rule; the re-match `unreachable!` and registry `expect` traps get invariant
  documentation or typed errors.

### Batch 2. One idiom, written down

- [ ] [Card 222](batch-cards/222-shared-primitives-and-idiom-codification.md):
  hex helper into `longhorn-core`; `bounded_text!` macro unification; adapter
  error-scaffold convergence or recorded per-adapter policy; the
  `Display`-via-`{self:?}` decision; `#[allow(missing_docs)]` removed from the
  wire types; `json_string` escaped via `serde_json`; the bindings doc-comment
  misplacement fixed structurally (lib+bin); the panic-invariant idiom
  codified in contracts so audits can check it mechanically.

### Batch 3. The dependency sweep

- [ ] [Card 223](batch-cards/223-dependency-sweep-and-measured-costs.md):
  `keyring` 4, `ed25519-dalek` 3, `base64` 0.23, `ts-rs` 12 (with generator
  hardening), `=tauri-build` pin rationale or removal; lockfile duplicates
  noted as upstream-pinned; the mutation clone-cost measurement; the
  scheduler wake-drop report; doctor's god-file and attention-marker backlog
  triaged (thresholds raised intentionally or files split).

## Dependency Shape

```text
memo 023 (M-native-content, M-missing-docs, M-display, low-tier quality,
          L6/L7 deps, doctor backlog)
 └─ 027 structural consolidation and dependency sweep
     ├─ 221 generation hoist     (independent)
     ├─ 222 shared primitives    (independent)
     └─ 223 dependency sweep     (after 222 — the hex helper touches identity modules)
```

## Goals

- [ ] contract 017's generation rule has one implementation
- [ ] the same five-line primitive does not exist six times with three error
  idioms
- [ ] `effigy doctor` is green because the tree is clean or the thresholds are
  chosen, not because nobody runs it
- [ ] no direct dependency is held back without a recorded reason

## Acceptance Criteria

- [ ] a rule fix in the attach-generation discipline lands in one file
- [ ] every exact pin (`=age`, `=rusqlite`, `=zip`, `=ts-rs`, `=tauri-build`)
  carries a rationale comment or moves to a compatible range
- [ ] the panic-invariant convention is checkable: an auditor can distinguish
  `expect("validated …")` from a straggler by rule, not taste
- [ ] the clone-cost measurement is recorded with its decision, either way

## Explicit Non-goals

- Splitting the large-but-organized modules (`lifecycle.rs`, `generation.rs`,
  `retention.rs`). The audit cleared them; size alone is not a defect.
- A GPUI workspace move. Card 172's cadence decision stands; the release gate
  already builds the seam.

## Next Task

Card 221. The three-way divergence widens with every native-content change;
the hoist gets cheaper the sooner it lands.

## Planning Checkpoint

After Card 223's `ts-rs` decision. If the generator hardening outgrows the
card, that is a shape question for the bindings crate — pause there rather
than shipping a string-split parser against an unpinned dependency.
