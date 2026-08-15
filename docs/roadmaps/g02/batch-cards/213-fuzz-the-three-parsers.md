# 213 Fuzz The Three Parsers

Status: complete
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.024 batch 3
Governing refs: contract 004; contract 008; memo 023 (coverage gap 2)
Depends on: none
Auto-start next card: no

## Objective

The three hand-rolled untrusted-input parsers meet generated input:
the zip backup inspector, `parse_utc_timestamp`, and the history/history-tree
envelope decoders.

## Why this exists

The state/data lane traced ~300 panic paths and found zero reachable panic on
untrusted input — by reading. The three parsers are the highest-value place
for that reading to be wrong: the zip archive inspector
(`crates/longhorn-config/src/backup/archive/`, bounds-checked but intricate),
`parse_utc_timestamp` (`identity.rs:360-416`, one strictness test), and the
history envelope decoders (`persistence/decode.rs`, wire limits present).
`proptest` is already a workspace dependency (`Cargo.toml:118`); nothing uses
it in scope.

## Scope

- property tests in the three crates' test suites
- a bounded iteration count wired into `qa` — this is a gate, not a fuzz farm
- any crash found is fixed in the same card

## Steps

1. Zip inspector: generate archives with adversarial central directories,
   truncated entries, overlapping offsets, declared-vs-actual size mismatch.
   Property: never panics, never reads past input, classification always one
   of the documented failure classes.
2. `parse_utc_timestamp`: generated date strings around the grammar edges —
   fractional seconds, leap-ish dates, offset forms the parser rejects.
   Property: accepted strings round-trip; rejected strings reject with the
   documented error.
3. History envelopes: mutated valid envelopes (bit flips, truncation, length
   field lies). Property: decode fails classified, never panics, never
   over-reads.
4. Wire into `test:rust` or a named selector with a fixed, small case count —
   record the count and its cost.

## Do Not

- Add a fuzzing framework dependency. `proptest` is present; `cargo fuzz` is
  a heavier commitment this card does not take.
- Tune properties until they pass. A failing property is the card working.

## Result

All three parsers have proptest suites at 64 fixed cases each; **the findings
list is empty** — zero panics, zero misclassifications, zero
non-determinism across six properties. The audit's read held under generated
input, which is the strongest evidence the parsers now have.

- Zip inspector (`longhorn-config/.../archive/tests.rs`): arbitrary and
  mutated archives (bit flips, truncation, EOCD/central-header length lies)
  inspect deterministically, failures stay typed, and any accepted output
  hashes back to the exact input.
- `parse_utc_timestamp`: calendar-valid generated timestamps round-trip
  against an independent reference computation; arbitrary strings accept only
  the documented grammar and never panic.
- History envelope decoders (both crates): mutated envelopes fail classified
  or re-encode cleanly.

Measured added cost: ~0.2s across all suites. One gate pin followed the work:
`verify-history-tree-artifacts.ts`'s test-count assertion moved 51 → 53 with
its comment extended (the pin is deliberate; the bump is the recorded kind).

## Acceptance Criteria

- [x] all three parsers have property suites running in `qa`
- [x] every crash or misclassification found is fixed or recorded — none found
- [x] the added gate cost is measured and under a recorded bound (~0.2s)

## Evidence Required

- the property suites and their case counts
- the findings list, empty or closed
- `effigy qa` green with the new cost recorded

## Stop Conditions

Stop if a found defect reaches a wire format consumers already persist —
that is a migration question, not a test fix.
