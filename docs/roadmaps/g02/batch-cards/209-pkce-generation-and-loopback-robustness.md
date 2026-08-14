# 209 PKCE Generation And Loopback Robustness

Status: complete
Completed: 2026-08-14
Owner: Tom
Roadmap: g02.023 batch 3
Governing refs: contract 019; memo 023 (opp-pkce, loopback findings)
Depends on: none
Auto-start next card: no

## Objective

Consumers get a CSPRNG generator for PKCE material instead of copying a
proof's timestamp stub, and the loopback listener survives hostile local
noise.

## Why this exists

`CodeVerifier::new` and `AccountFlow::begin` validate but never generate —
"this crate is pure" (`crates/longhorn-licence/src/account.rs:19-21`). The
only in-repo generation example is the proof's timestamp/PID-based stub
(`examples/tauri-update-proof` `main.rs:398-399`, explicitly commented as a
stub), which is exactly the pattern a consumer will copy into production.

The loopback listener (`crates/longhorn-browser/src/loopback.rs`) has two
robustness holes: `receive()` propagates any Io error from `answer()`
(`:113-132`), so a local scanner connecting and RSTing before the 404 write
kills the whole sign-in wait — contradicting the module's own contract
("keeps waiting", `:23-24`); and the deadline is only checked in the accept
loop, so a connection dribbling one byte per <5s holds the flow open past its
deadline (`:142-165`).

## Scope

- `crates/longhorn-licence` — generation API (a `rand` dependency, or a
  documented recipe if the crate must stay pure)
- `crates/longhorn-browser` — probe tolerance, total-connection deadline
- the proof's stub, replaced by the real generator

## Steps

1. Add `CodeVerifier::generate()` (and state generation on `AccountFlow`)
   behind CSPRNG. If adding `rand` breaks the crate's purity claim, decide:
   a feature flag, or the generator lives in a sibling and the pure crate
   keeps validating. Record the choice.
2. Replace the proof's stub with the real generator.
3. Loopback: classify Io errors from `answer()` — a dead non-callback
   connection is noise; swallow and keep waiting. Callback-path failures stay
   fail-closed as documented.
4. Enforce a total-connection deadline inside `answer()`, not only per-`read`.
5. Tests: disconnecting probe (RST before the 404), trickle-past-deadline,
   and the well-behaved probe case that already exists.

## Do Not

- Loosen callback validation to make probes survivable — fail-closed on the
  callback path is documented and correct.
- Leave the stub in the proof "because it is a proof". The audit's point is
  that proofs are the template consumers copy.

## Result

Generation moved into the crate: `CodeVerifier::generate()` (32 bytes from
`getrandom`, base64url — exactly the RFC 7636 floor) and
`AccountFlow::generate(port)` for the common both-fresh case. The purity
claim narrowed honestly rather than staying absolute: the crate still holds
no clock, filesystem, or network, and OS entropy is the one ambient source a
PKCE implementation cannot inject without pushing the footgun back onto
consumers. The update proof's timestamp/PID stub is gone — it now calls
`AccountFlow::generate`, so the pattern a consumer copies is the safe one.
The licence harness's fixed strings stay: deterministic evidence is what a
harness is for.

The loopback listener now treats probe traffic as noise: a connection that
dies before completing a request, a long garbage head not addressed at the
callback, and a 404 write that fails because the probe hung up all leave the
wait running. The callback path still fails closed. And every read is
bounded by the flow's remaining deadline, so a byte-at-a-time peer cannot
hold the flow open past it. Three new tests: disconnect-mid-request,
trickle-past-deadline, and the existing well-behaved probe still passes.

## Acceptance Criteria

- [x] PKCE generation is CSPRNG-backed and the proof uses it
- [x] a disconnecting probe does not kill the sign-in wait, with a test
- [x] no connection outlives the flow deadline, with a test

## Evidence Required

- the purity decision and where the generator lives
- the new tests
- `effigy qa` green

## Stop Conditions

Stop if the purity decision changes the crate's dependency shape in a way that
affects the distribution candidate — that crosses into contract 012 territory.
