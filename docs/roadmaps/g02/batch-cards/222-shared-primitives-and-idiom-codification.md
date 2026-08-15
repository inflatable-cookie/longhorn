# 222 Shared Primitives And Idiom Codification

Status: complete
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.027 batch 2
Governing refs: contract 001; memo 023 (quality lane M1, M2, L1-L5;
presentation lane L1/L2)
Depends on: none
Auto-start next card: no

## Objective

The small duplicated primitives get one home, the most public wire types stop
opting out of the docs deny, and the repo's panic-invariant idiom is written
down where the next audit can check it mechanically.

## Why this exists

- Hand-rolled hex encoding exists six times with three error idioms
  (`longhorn-config/src/backup/types/evidence.rs:11-18` swallowing the error,
  `longhorn-settings/src/registry/identity.rs:54-62` and
  `longhorn-settings-config/src/authority.rs:149-154` expecting, plus
  command/command-config/transfer identity modules).
- `#[allow(missing_docs)]` sits on `ts_rs`-exported wire types — the exact
  types that cross to TypeScript:
  `longhorn-operation/src/protocol/mutation/receipt.rs:55,190`,
  `result.rs:13`, `longhorn-notifications/src/protocol/mutation.rs:11,145,264`,
  `snapshot.rs:108`.
- `Display` via `{self:?}` in history-tree and one config error type couples
  host-facing text to internal variant names; siblings hand-write messages.
- Six near-identical Tauri adapter error scaffolds with drifting code names
  (`StateUnavailable` vs `AuthorityStateUnavailable` vs
  `ExecutorStateUnavailable`); `bounded_text!` exists in notifications while
  operation hand-writes the identical newtype.
- The bindings generator's `json_string` escapes only `"`, `\`, `\n`
  (`crates/longhorn-bindings/src/generation.rs:397-410`), and a misplaced doc
  comment (`:95-105`) is invisible because the binary crate never fires
  `missing_docs`.
- The `expect("validated …")` convention is pervasive and good — and exists
  only as folklore, so audits cannot distinguish it from stragglers
  (`longhorn-config/src/location.rs:217,248`,
  `backup/types/identity.rs:405`).

## Scope

- `crates/longhorn-core` — the hex helper
- the six hex call sites, the wire-type docs, the Display decision, the
  adapter scaffolds, `bounded_text!` unification
- `crates/longhorn-bindings` — `json_string` via `serde_json`, lib+bin if
  Card 214 has not already done it
- `docs/contracts/` — the idiom entry

## Steps

1. `longhorn-core` gains the hex helper; six call sites converge. (Two callers
   was the papercut's bar for URL parsing; six settles the question here.)
2. Remove the `#[allow(missing_docs)]` opts-out; document the variants. These
   strings cross to TypeScript — the docs are the consumer's contract.
3. Decide `Display`-via-`{self:?}`: per-variant messages in history-tree and
   the config error, or a recorded acceptance. Message content tests either
   way, so a rename cannot silently change host-facing text.
4. Adapter error scaffolds: converge on one shape and one code vocabulary, or
   record the per-adapter fork as policy. Same for `bounded_text!` vs the
   hand-written twin.
5. `json_string` escapes via `serde_json` (already a dependency); lib+bin the
   generator if not already done so `missing_docs` applies.
6. Write the panic-invariant idiom into contracts: `expect("validated …")`
   with the guarding check named is the convention; bare `unwrap()` on an
   invariant needs the message. Convert the three stragglers.

## Do Not

- Convergence for its own sake. Where the audit found deliberate divergence
  (scale f32/f64, transfer vs surface-transfer), leave it.
- Grow `longhorn-core` an HTTP or crypto concept — the helper is bytes-to-hex,
  nothing more.

## Result

One `lowercase_hex` lives in `longhorn-core` and all six call sites use it.
The wire-type `#[allow(missing_docs)]` opts-out are gone — the variants carry
docs, and the generated TypeScript gained the doc comments with them (the
generated protocol diffs are the evidence). The five `Display`-via-`{self:?}`
sites are hand-written per-variant messages with content tests, so a variant
rename breaks a test instead of a host's log. `json_string` escapes through
`serde_json`, the misplaced doc comment moved, and `longhorn-bindings` is
lib+bin — `missing_docs` applies. The adapter scaffolds kept their per-crate
copies with the fork recorded as policy (convergence was not small). The
idiom is codified in contract 001 (Code Idioms) and the three bare-unwrap
stragglers converted.

## Acceptance Criteria

- [x] one hex implementation, one idiom
- [x] no `#[allow(missing_docs)]` on wire types
- [x] host-facing error text is rename-proof by test
- [x] the idiom entry exists and an auditor can apply it mechanically

## Evidence Required

- the diffs and the contract entry
- `effigy qa` green

## Stop Conditions

None anticipated. If scaffold convergence touches a wire code a consumer
matches on, that is a consumer break and coordinates per contract 001.
