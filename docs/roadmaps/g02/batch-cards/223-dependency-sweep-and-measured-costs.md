# 223 Dependency Sweep And Measured Costs

Status: ready
Owner: Tom
Roadmap: g02.027 batch 3
Governing refs: contract 012; memo 023 (hygiene L6/L7; quality opp-3;
presentation L4; doctor backlog)
Depends on: Card 222 (the primitive convergence touches the same identity
modules some bumps will regenerate)
Auto-start next card: no

## Objective

No direct dependency is held back without a recorded reason, the two measured
cost questions get numbers instead of impressions, and doctor's backlog is
triaged to green-by-choice.

## Why this exists

- Direct deps a major behind: `keyring` 3.6.3 (4.1.6 exists),
  `ed25519-dalek` 2.2.0 (3.0.0), `ts-rs =11.1.0` (12.0.1), `base64` 0.22.1
  (0.23.1). Exact pins (`=age`, `=rusqlite`, `=zip`, `=ts-rs`, `=tauri-build`)
  carry no rationale comments; `=tauri-build 2.6.3` dates to the initial
  commit and produces a mixed stack (tauri 2.11.5 with codegen/macros 2.6.3).
  Lockfile duplicates (`base64` 0.21.7, `sha2` 0.10.9) are upstream-pinned —
  note, don't chase.
- `ts-rs` unpin is a generator-hardening task: the bindings generator parses
  `ts-rs` output by string splitting and survives only because of the pin.
- Two unmeasured costs: operation/notification mutations deep-clone record
  vectors under 65 536-record ceilings (`operation/src/catalogue/mutate.rs`,
  `notifications/src/ledger/mutate.rs`), and the windowing scheduler drops
  wakes silently when the handler has died
  (`longhorn-tauri-windowing/src/lifecycle/services/scheduler.rs:139-146`).
- Doctor is red today on findings nobody has triaged: 40 god-file errors
  (mostly test files and prototypes) and one attention-marker false positive
  (`decision.rs:142`'s comment contains `[SECURITY]`).

## Scope

- `Cargo.toml` workspace deps + rationale comments
- `crates/longhorn-bindings` — generator hardening if `ts-rs` moves
- the two measurements, recorded with decisions
- doctor backlog disposition

## Steps

1. Bump `keyring`, `ed25519-dalek`, `base64` with their conformance suites
   (keyring contract tests, licence verify suite). Each bump lands with its
   evidence or stays with a written reason.
2. `ts-rs` 12: harden the generator's parsing first (or pin the output shape
   with a self-test), then unpin, regenerate, and diff every fixture. If the
   hardening outgrows the card, stop per the milestone checkpoint.
3. `=tauri-build`: find the reason or remove the pin; a mixed Tauri stack
   needs a recorded justification either way. Rationale comments on every
   remaining exact pin.
4. Measure the mutation clone cost at ceiling scale; decide keep-or-change
   with the number recorded. Deliberately-simple-and-correct is a legitimate
   outcome — with the number.
5. Scheduler: a dropped wake reports (reporter call or `schedule` error)
   instead of vanishing.
6. Doctor backlog: split the few genuinely oversized *source* files or raise
   thresholds intentionally for tests/prototypes; teach the attention-marker
  scan that a quoted `[SECURITY]` in a comment is not a marker. Doctor ends
   green-by-choice.

## Do Not

- Chase the transitive lockfile duplicates — upstream-pinned, recorded once.
- Bump `age`/`rusqlite`/`zip` casually: g02.007 already moved them under
  frozen evidence; a further bump needs the same conformance treatment.

## Acceptance Criteria

- [ ] every held-back direct dependency carries a recorded reason
- [ ] every exact pin has a rationale comment or is gone
- [ ] both measurements are recorded with their decisions
- [ ] `effigy doctor` is green with intentional thresholds

## Evidence Required

- the conformance runs per bump
- the two measurement numbers
- doctor output, green

## Stop Conditions

Stop if a bump breaks a conformance suite in a way that implies behavior
change — that is a finding, not a version number.
